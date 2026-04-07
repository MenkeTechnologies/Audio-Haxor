#include <iostream>
#include <string>

#include <juce_gui_basics/juce_gui_basics.h>

#include "Engine.hpp"

int main()
{
    juce::ScopedJuceInitialiser_GUI juceInit;
    audio_haxor::Engine engine;

    std::string line;
    while (std::getline(std::cin, line))
    {
        const juce::String trimmed = juce::String(line).trim();
        if (trimmed.isEmpty())
            continue;

        const auto parsed = juce::JSON::parse(trimmed);
        if (parsed.isVoid())
        {
            std::cout << R"({"ok":false,"error":"bad JSON"})" << '\n' << std::flush;
            continue;
        }

        const juce::var out = engine.dispatch(parsed);
        std::cout << juce::JSON::toString(out, true) << '\n' << std::flush;
    }
    return 0;
}
