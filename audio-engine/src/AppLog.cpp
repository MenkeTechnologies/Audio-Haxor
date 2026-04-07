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
/** Set in `initAppLogFromEnv` when a log path is resolved — parent dir is the Tauri app data folder. */
juce::File g_appDataDirFromLogEnv;

/** Same cap as host `write_app_log_line` (`src-tauri/src/lib.rs`). */
static constexpr juce::int64 kMaxEngineLogBytes = 5 * 1024 * 1024;

/** Rename to `*.1` (replacing an old backup); if rename fails, delete or truncate so the log cannot grow without bound. */
static void rotateEngineLogIfNeeded(const juce::File& logFile)
{
    if (logFile.getFullPathName().isEmpty())
        return;
    if (!logFile.existsAsFile())
        return;
    if (logFile.getSize() <= kMaxEngineLogBytes)
        return;

    const juce::File parent = logFile.getParentDirectory();
    (void) parent.createDirectory();
    const juce::File backup = parent.getChildFile(logFile.getFileName() + ".1");
    if (backup.existsAsFile())
        (void) backup.deleteFile();

    if (logFile.moveFileTo(backup))
        return;

    if (logFile.deleteFile())
        return;
    (void) logFile.replaceWithText("");
}

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
    g_appDataDirFromLogEnv = juce::File();
    juce::String p = juce::SystemStats::getEnvironmentVariable("AUDIO_HAXOR_ENGINE_LOG", {}).trim();
    if (p.isEmpty())
        p = juce::SystemStats::getEnvironmentVariable("AUDIO_HAXOR_APP_LOG", {}).trim();
    g_engineLogPath = p;
    const char* mirror = std::getenv("AUDIO_HAXOR_ENGINE_LOG_STDERR");
    g_mirrorEngineLogToStderr = (mirror != nullptr && mirror[0] != '\0');

    if (g_engineLogPath.isNotEmpty())
    {
        g_appDataDirFromLogEnv = juce::File(g_engineLogPath).getParentDirectory();
        rotateEngineLogIfNeeded(juce::File(g_engineLogPath));
    }
}

juce::File appDataDirectoryForAudioEngine()
{
    if (g_appDataDirFromLogEnv.getFullPathName().isNotEmpty())
        return g_appDataDirFromLogEnv;
    juce::File dir = juce::File::getSpecialLocation(juce::File::userApplicationDataDirectory)
                         .getChildFile("MenkeTechnologies")
                         .getChildFile("audio-haxor");
    (void) dir.createDirectory();
    return dir;
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

    rotateEngineLogIfNeeded(f);

    const auto path = f.getFullPathName().toStdString();
    std::ofstream out(path, std::ios::app | std::ios::binary);
    if (!out.is_open())
        return;
    out << line.toRawUTF8();
    out.flush();
}

} // namespace audio_haxor
