#pragma once

#include <juce_audio_formats/juce_audio_formats.h>

namespace audio_haxor {

/** Decode `path` and return min/max peaks per column (mono mix). Does not require AudioDeviceManager. */
juce::var waveformPreview(juce::AudioFormatManager& formatManager, const juce::var& req);

/** STFT magnitude spectrogram (dB). Does not require AudioDeviceManager. */
juce::var spectrogramPreview(juce::AudioFormatManager& formatManager, const juce::var& req);

} // namespace audio_haxor
