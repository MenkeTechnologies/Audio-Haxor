#include "AppLog.hpp"

#include <chrono>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iostream>
#include <mutex>

namespace audio_haxor {
namespace {

std::mutex g_engineLogMutex;
juce::String g_engineLogPath;
bool g_mirrorEngineLogToStderr = false;

static juce::String utcTimestampString()
{
    using namespace std::chrono;
    const auto now = system_clock::now();
    const std::time_t t = system_clock::to_time_t(now);
    std::tm tm{};
#if defined(_WIN32)
    gmtime_s(&tm, &t);
#else
    gmtime_r(&t, &tm);
#endif
    char buf[32];
    std::strftime(buf, sizeof(buf), "%Y-%m-%d %H:%M:%S", &tm);
    return juce::String(buf);
}

} // namespace

void initAppLogFromEnv()
{
    juce::String p = juce::SystemStats::getEnvironmentVariable("AUDIO_HAXOR_ENGINE_LOG", {}).trim();
    if (p.isEmpty())
        p = juce::SystemStats::getEnvironmentVariable("AUDIO_HAXOR_APP_LOG", {}).trim();
    g_engineLogPath = p;
    const char* mirror = std::getenv("AUDIO_HAXOR_ENGINE_LOG_STDERR");
    g_mirrorEngineLogToStderr = (mirror != nullptr && mirror[0] != '\0');
}

void appLogLine(const juce::String& message)
{
    const std::lock_guard<std::mutex> lock(g_engineLogMutex);

    const juce::String line = juce::String("[") + utcTimestampString() + "] ENGINE: " + message + "\n";

    if (g_mirrorEngineLogToStderr)
    {
        std::cerr << line.toRawUTF8();
        std::cerr.flush();
    }

    if (g_engineLogPath.isEmpty())
        return;

    juce::File f(g_engineLogPath);
    const juce::File parent = f.getParentDirectory();
    (void) parent.createDirectory();

    constexpr juce::int64 kMaxLogSize = 5 * 1024 * 1024;
    if (f.existsAsFile() && f.getSize() > kMaxLogSize)
    {
        const juce::File backup = parent.getChildFile(f.getFileName() + ".1");
        (void) f.moveFileTo(backup);
    }

    const auto path = f.getFullPathName().toStdString();
    std::ofstream out(path, std::ios::app | std::ios::binary);
    if (!out.is_open())
        return;
    out << line.toRawUTF8();
    out.flush();
}

} // namespace audio_haxor
