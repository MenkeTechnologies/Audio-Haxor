#pragma once

#include <juce_core/juce_core.h>

namespace audio_haxor {

/** Read `AUDIO_HAXOR_ENGINE_LOG` (path to `engine.log`); if unset, falls back to `AUDIO_HAXOR_APP_LOG`. Also reads `AUDIO_HAXOR_ENGINE_LOG_STDERR`. */
void initAppLogFromEnv();

/** Append one timestamped line `[UTC] ENGINE: …` to the resolved log file (same bracket timestamp as host `write_app_log_line`).
    If `AUDIO_HAXOR_ENGINE_LOG_STDERR` is set (any non-empty value), the same line is also written to stderr. */
void appLogLine(const juce::String& message);

} // namespace audio_haxor
