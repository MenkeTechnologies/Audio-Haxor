#include "ParentWatchdog.hpp"
#include "AppLog.hpp"

#include <chrono>
#include <cstdlib>
#include <cstring>
#include <thread>

#if defined(_WIN32)
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#else
#include <cerrno>
#include <csignal>
#include <unistd.h>
#endif

namespace audio_haxor {
namespace {

#if defined(_WIN32)
static void exitAudioEngine(const char* reason)
{
    audio_haxor::appLogLine(juce::String("parent watchdog: ") + reason);
    std::_Exit(0);
}

static void watchdogLoopWin(DWORD parentPid)
{
    for (;;)
    {
        std::this_thread::sleep_for(std::chrono::seconds(2));
        HANDLE h = OpenProcess(SYNCHRONIZE, FALSE, parentPid);
        if (h == nullptr)
        {
            const DWORD err = GetLastError();
            if (err == ERROR_INVALID_PARAMETER)
                exitAudioEngine("OpenProcess: parent gone or invalid PID");
            continue;
        }
        const DWORD w = WaitForSingleObject(h, 0);
        CloseHandle(h);
        if (w == WAIT_OBJECT_0)
            exitAudioEngine("parent process exited");
    }
}
#else
static void exitAudioEngine(const char* reason)
{
    audio_haxor::appLogLine(juce::String("parent watchdog: ") + reason);
    std::_Exit(0);
}

static void watchdogLoopPosix(pid_t parentPid)
{
    for (;;)
    {
        std::this_thread::sleep_for(std::chrono::seconds(2));
        if (kill(parentPid, 0) != 0)
        {
            if (errno == ESRCH)
                exitAudioEngine("parent PID no longer exists");
            continue;
        }
    }
}
#endif

} // namespace

void startParentWatchdogFromEnv()
{
    const char* env = std::getenv("AUDIO_HAXOR_PARENT_PID");
    if (env == nullptr || env[0] == '\0')
        return;

    char* end = nullptr;
    const long parsed = std::strtol(env, &end, 10);
    if (parsed <= 0 || end == env)
        return;

#if defined(_WIN32)
    const DWORD ppid = static_cast<DWORD>(parsed);
    audio_haxor::appLogLine(juce::String("parent watchdog: armed, parent_pid=") + juce::String((int) ppid));
    std::thread([ppid]() { watchdogLoopWin(ppid); }).detach();
#else
    const pid_t ppid = static_cast<pid_t>(parsed);
    audio_haxor::appLogLine(juce::String("parent watchdog: armed, parent_pid=") + juce::String((int) ppid));
    std::thread([ppid]() { watchdogLoopPosix(ppid); }).detach();
#endif
}

} // namespace audio_haxor
