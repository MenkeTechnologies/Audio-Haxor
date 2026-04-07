#include "CrashHandler.hpp"

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

namespace audio_haxor {
namespace {

char g_appLogPathForCrash[4096];

static void writeAll(int fd, const char* data, size_t len)
{
    size_t off = 0;
    while (off < len)
    {
        const ssize_t n = ::write(fd, data + off, len - off);
        if (n <= 0)
            break;
        off += (size_t) n;
    }
}

static void writeBacktraceToFd(int fd)
{
#if defined(__APPLE__) || defined(__linux__) || defined(__unix__)
    void* frames[48];
    const int n = ::backtrace(frames, 48);
    if (n > 0)
        ::backtrace_symbols_fd(frames, n, fd);
#else
    const char msg[] = "(backtrace not available on this platform)\n";
    writeAll(fd, msg, sizeof(msg) - 1);
#endif
}

static void onFatalSignal(int sig, siginfo_t* info, void* /*uctx*/)
{
    char line[320];
    int k;
    if (info != nullptr)
        k = std::snprintf(line, sizeof(line), "ENGINE [fatal signal %d] si_addr=%p\n", sig, info->si_addr);
    else
        k = std::snprintf(line, sizeof(line), "ENGINE [fatal signal %d]\n", sig);
    if (k > 0 && k < (int) sizeof(line))
        writeAll(STDERR_FILENO, line, (size_t) k);
    writeBacktraceToFd(STDERR_FILENO);

    if (g_appLogPathForCrash[0] != '\0')
    {
        const int fd = ::open(g_appLogPathForCrash, O_WRONLY | O_APPEND | O_CREAT, 0644);
        if (fd >= 0)
        {
            const char hdr[] = "[ENGINE crash] ";
            writeAll(fd, hdr, sizeof(hdr) - 1);
            if (k > 0 && k < (int) sizeof(line))
                writeAll(fd, line, (size_t) k);
            writeBacktraceToFd(fd);
            ::close(fd);
        }
    }

    ::_exit(128 + sig);
}

static void engineTerminateHandler()
{
    const char msg[] = "ENGINE: std::terminate (uncaught exception)\n";
    writeAll(STDERR_FILENO, msg, sizeof(msg) - 1);
    writeBacktraceToFd(STDERR_FILENO);
    if (g_appLogPathForCrash[0] != '\0')
    {
        const int fd = ::open(g_appLogPathForCrash, O_WRONLY | O_APPEND | O_CREAT, 0644);
        if (fd >= 0)
        {
            writeAll(fd, msg, sizeof(msg) - 1);
            writeBacktraceToFd(fd);
            ::close(fd);
        }
    }
    std::_Exit(1);
}

} // namespace

void installEngineCrashHandlers()
{
    std::memset(g_appLogPathForCrash, 0, sizeof(g_appLogPathForCrash));
    if (const char* eng = ::getenv("AUDIO_HAXOR_ENGINE_LOG"))
    {
        if (eng[0] != '\0')
            std::strncpy(g_appLogPathForCrash, eng, sizeof(g_appLogPathForCrash) - 1);
    }
    else if (const char* app = ::getenv("AUDIO_HAXOR_APP_LOG"))
    {
        if (app[0] != '\0')
            std::strncpy(g_appLogPathForCrash, app, sizeof(g_appLogPathForCrash) - 1);
    }

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
    sa.sa_flags = SA_SIGINFO | SA_RESETHAND;

    (void) sigaction(SIGSEGV, &sa, nullptr);
    (void) sigaction(SIGBUS, &sa, nullptr);
    (void) sigaction(SIGILL, &sa, nullptr);
    (void) sigaction(SIGFPE, &sa, nullptr);
    (void) sigaction(SIGABRT, &sa, nullptr);

    std::set_terminate(engineTerminateHandler);
}

} // namespace audio_haxor

#endif
