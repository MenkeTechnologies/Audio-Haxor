#include "CrashHandler.hpp"

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <exception>

#if defined(_WIN32)

namespace audio_haxor {
void installEngineCrashHandlers()
{
    /* SIGPIPE N/A; use structured exception handling later if needed. */
}
} // namespace audio_haxor

#else

#include <fcntl.h>
#include <signal.h>
#include <unistd.h>

#if defined(__APPLE__) || defined(__linux__) || defined(__unix__)
#include <execinfo.h>
#endif

#if defined(__APPLE__)
#include <mach-o/dyld.h>
#endif

namespace audio_haxor {
namespace {

/* Pre-opened log file descriptor. Opening inside the signal handler is not async-signal-safe
 * (libc `::open` can lock the VFS cache / allocator), and when the heap is already corrupted
 * — which is the exact situation where we most need a crash log — that lock acquisition is
 * what silently killed the handler. Open once at install time, reuse for every crash. */
int g_crashLogFd = -1;

/* Dedicated alt stack for the fatal signal handler. If the crashing thread's stack is
 * exhausted or corrupted, delivering the signal onto the *same* stack faults inside
 * `_sigtramp` before our handler runs — producing a silent kill. `sigaltstack` + `SA_ONSTACK`
 * switches to this preallocated 64 KiB region. Static storage so we don't allocate in the
 * handler path. */
constexpr size_t kAltStackSize = 64 * 1024;
alignas(16) char g_altStack[kAltStackSize];

static void writeAll(int fd, const char* data, size_t len)
{
    if (fd < 0)
        return;
    size_t off = 0;
    while (off < len)
    {
        const ssize_t n = ::write(fd, data + off, len - off);
        if (n <= 0)
            break;
        off += (size_t) n;
    }
}

/* Async-signal-safe hex writer — `snprintf("%p")` is technically not on the POSIX safe list
 * (it can take a locale lock on some libcs), and when the heap is poisoned even that is
 * dangerous. This routine only touches the stack and the passed-in fd. */
static void writeHexPtr(int fd, void* p)
{
    char buf[2 + 16 + 1];
    buf[0] = '0';
    buf[1] = 'x';
    const uintptr_t v = reinterpret_cast<uintptr_t>(p);
    const char* hex = "0123456789abcdef";
    for (int i = 0; i < 16; ++i)
    {
        buf[2 + i] = hex[(v >> ((15 - i) * 4)) & 0xF];
    }
    buf[18] = '\n';
    writeAll(fd, buf, sizeof(buf));
}

static void writeDec(int fd, int v)
{
    char buf[16];
    int len = 0;
    if (v < 0)
    {
        buf[len++] = '-';
        v = -v;
    }
    char tmp[12];
    int ti = 0;
    if (v == 0)
        tmp[ti++] = '0';
    else
    {
        while (v > 0 && ti < (int) sizeof(tmp))
        {
            tmp[ti++] = (char) ('0' + (v % 10));
            v /= 10;
        }
    }
    while (ti > 0 && len < (int) sizeof(buf))
        buf[len++] = tmp[--ti];
    writeAll(fd, buf, (size_t) len);
}

/* Async-signal-safe backtrace dump. macOS `backtrace_symbols_fd` internally calls `malloc` —
 * poisoned-heap crashes (the whole reason we're in this handler) re-enter the broken allocator
 * and kill the handler mid-dump with no log. Raw addresses only — symbolicate later with
 * `atos -o audio-engine -l <load_addr> <addr>` or feed to a helper. */
static void writeBacktraceRawToFd(int fd)
{
#if defined(__APPLE__) || defined(__linux__) || defined(__unix__)
    void* frames[64];
    const int n = ::backtrace(frames, 64);
    for (int i = 0; i < n; ++i)
        writeHexPtr(fd, frames[i]);
#else
    const char msg[] = "(backtrace not available on this platform)\n";
    writeAll(fd, msg, sizeof(msg) - 1);
#endif
}

/* Dump the loaded dyld image list so every address in the backtrace can be mapped back to a
 * dylib. `atos -o <dylib> -l <base>` wants the base address to subtract from each frame —
 * without this map we only have raw pointers and no way to know which plugin (or VST3
 * bundle's own `Contents/MacOS/<bin>`) loaded at which base.
 *
 * Signal-safety note: `_dyld_image_count` / `_dyld_get_image_name` / `_dyld_get_image_header`
 * walk dyld's internal in-process image list. Apple doesn't formally document them as
 * async-signal-safe, but they perform no allocation and only read a dyld-owned read-only
 * array — in practice they are the same kind of "safe enough" that `backtrace()` is. We run
 * them AFTER the backtrace is already on disk so if this walk faults we at least have the
 * frame addresses. Each line is `image 0x<hex_base> <path>\n`. */
static void writeImageListToFd(int fd)
{
#if defined(__APPLE__)
    const uint32_t count = ::_dyld_image_count();
    for (uint32_t i = 0; i < count; ++i)
    {
        const char* name = ::_dyld_get_image_name(i);
        const struct mach_header* hdr = ::_dyld_get_image_header(i);
        if (name == nullptr || hdr == nullptr)
            continue;
        const char prefix[] = "image ";
        writeAll(fd, prefix, sizeof(prefix) - 1);
        /* Base = in-memory header pointer. `atos -l 0x<this>` wants exactly this value. */
        char hex[2 + 16];
        hex[0] = '0';
        hex[1] = 'x';
        const uintptr_t v = reinterpret_cast<uintptr_t>(hdr);
        const char* tbl = "0123456789abcdef";
        for (int j = 0; j < 16; ++j)
            hex[2 + j] = tbl[(v >> ((15 - j) * 4)) & 0xF];
        writeAll(fd, hex, sizeof(hex));
        writeAll(fd, " ", 1);
        /* strlen walks read-only program memory — safe. */
        size_t nlen = 0;
        while (name[nlen] != '\0' && nlen < 4096)
            ++nlen;
        writeAll(fd, name, nlen);
        writeAll(fd, "\n", 1);
    }
#else
    (void) fd;
#endif
}

static void onFatalSignal(int sig, siginfo_t* info, void* /*uctx*/)
{
    /* Order matters: write a "handler entered" marker BEFORE any other operation so we know
     * the handler at least ran, even if everything after this faults. */
    const char entered[] = "\n[ENGINE crash] handler entered\n";
    writeAll(STDERR_FILENO, entered, sizeof(entered) - 1);
    writeAll(g_crashLogFd, entered, sizeof(entered) - 1);

    writeAll(STDERR_FILENO, "ENGINE [fatal signal ", 21);
    writeAll(g_crashLogFd, "ENGINE [fatal signal ", 21);
    writeDec(STDERR_FILENO, sig);
    writeDec(g_crashLogFd, sig);
    writeAll(STDERR_FILENO, "] si_addr=", 10);
    writeAll(g_crashLogFd, "] si_addr=", 10);
    writeHexPtr(STDERR_FILENO, info != nullptr ? info->si_addr : nullptr);
    writeHexPtr(g_crashLogFd, info != nullptr ? info->si_addr : nullptr);

    writeBacktraceRawToFd(STDERR_FILENO);
    writeBacktraceRawToFd(g_crashLogFd);

    /* First fsync — guarantee the frame addresses are durable on disk BEFORE we walk the
     * (slightly riskier) dyld image list. If the image walk faults, at least the backtrace
     * already hit the log. */
    if (g_crashLogFd >= 0)
        (void) ::fsync(g_crashLogFd);

    const char imgHdr[] = "\n[ENGINE crash] dyld images\n";
    writeAll(STDERR_FILENO, imgHdr, sizeof(imgHdr) - 1);
    writeAll(g_crashLogFd, imgHdr, sizeof(imgHdr) - 1);
    writeImageListToFd(STDERR_FILENO);
    writeImageListToFd(g_crashLogFd);

    if (g_crashLogFd >= 0)
        (void) ::fsync(g_crashLogFd);

    ::_exit(128 + sig);
}

static void engineTerminateHandler()
{
    const char msg[] = "ENGINE: std::terminate (uncaught exception)\n";
    writeAll(STDERR_FILENO, msg, sizeof(msg) - 1);
    writeAll(g_crashLogFd, msg, sizeof(msg) - 1);
    writeBacktraceRawToFd(STDERR_FILENO);
    writeBacktraceRawToFd(g_crashLogFd);
    if (g_crashLogFd >= 0)
        (void) ::fsync(g_crashLogFd);
    const char imgHdr[] = "\n[ENGINE terminate] dyld images\n";
    writeAll(STDERR_FILENO, imgHdr, sizeof(imgHdr) - 1);
    writeAll(g_crashLogFd, imgHdr, sizeof(imgHdr) - 1);
    writeImageListToFd(STDERR_FILENO);
    writeImageListToFd(g_crashLogFd);
    if (g_crashLogFd >= 0)
        (void) ::fsync(g_crashLogFd);
    std::_Exit(1);
}

} // namespace

void installEngineCrashHandlers()
{
    /* Pre-open the crash log at install time. Opening inside the handler can deadlock or
     * corrupt memory when the heap is the thing that's broken. */
    const char* logPath = ::getenv("AUDIO_HAXOR_ENGINE_LOG");
    if (logPath == nullptr || logPath[0] == '\0')
        logPath = ::getenv("AUDIO_HAXOR_APP_LOG");
    if (logPath != nullptr && logPath[0] != '\0')
    {
        /* O_CLOEXEC so children (e.g. AU helper subprocesses) don't inherit the fd. */
        g_crashLogFd = ::open(logPath, O_WRONLY | O_APPEND | O_CREAT | O_CLOEXEC, 0644);
    }

    /* Install an alternate signal stack. `SA_ONSTACK` below must be paired with this.
     * Without it, a stack-corruption crash (or a crashing-thread stack that is too close to
     * its guard page) delivers the signal onto the same bad stack and silently kills the
     * process — no handler ever runs. */
    stack_t alt;
    std::memset(&alt, 0, sizeof(alt));
    alt.ss_sp = g_altStack;
    alt.ss_size = kAltStackSize;
    alt.ss_flags = 0;
    (void) ::sigaltstack(&alt, nullptr);

    struct sigaction ign;
    std::memset(&ign, 0, sizeof(ign));
    ign.sa_handler = SIG_IGN;
    sigemptyset(&ign.sa_mask);
    ign.sa_flags = 0;
    (void) sigaction(SIGPIPE, &ign, nullptr);

    struct sigaction sa;
    std::memset(&sa, 0, sizeof(sa));
    sa.sa_sigaction = onFatalSignal;
    sigemptyset(&sa.sa_mask);
    /* `SA_ONSTACK`: run handler on the alt stack we just installed.
     * No `SA_RESETHAND`: the old flag reset the handler to `SIG_DFL` after one signal, so any
     * secondary fault inside the handler itself (e.g. `backtrace_symbols_fd` hitting a broken
     * allocator) silently killed the process. We want the handler re-entrant-ish; if we do
     * crash inside the handler, let us at least try to handle it. The handler calls `_exit`
     * so there's no real re-entry risk on the happy path. */
    sa.sa_flags = SA_SIGINFO | SA_ONSTACK;

    (void) sigaction(SIGSEGV, &sa, nullptr);
    (void) sigaction(SIGBUS, &sa, nullptr);
    (void) sigaction(SIGILL, &sa, nullptr);
    (void) sigaction(SIGFPE, &sa, nullptr);
    (void) sigaction(SIGABRT, &sa, nullptr);
    /* SIGTRAP: macOS libmalloc's xzone allocator traps here when it detects freelist
     * corruption (`_xzm_xzone_malloc_freelist_outlined` → platform crash-reason →
     * `brk #1`). Apple's ReportCrash still captures the `.ips` file from the Mach
     * exception port first, but adding SIGTRAP to our POSIX handler ensures we *also*
     * get a copy in `engine.log` with our image list, for machines where
     * `~/Library/Logs/DiagnosticReports` is scrubbed or inaccessible. */
    (void) sigaction(SIGTRAP, &sa, nullptr);

    std::set_terminate(engineTerminateHandler);
}

} // namespace audio_haxor

#endif
