#pragma once

#include <juce_core/juce_core.h>
#include <memory>

namespace audio_haxor {

class Engine
{
public:
    Engine();
    ~Engine();

    juce::var dispatch(const juce::var& req);

private:
    struct Impl;
    std::unique_ptr<Impl> impl;
};

} // namespace audio_haxor
