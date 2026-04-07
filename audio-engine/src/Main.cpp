#include <iostream>
#include <memory>
#include <string>
#include <thread>

#include <juce_events/juce_events.h>
#include <juce_gui_basics/juce_gui_basics.h>

#include "AppLog.hpp"
#include "CrashHandler.hpp"
#include "Engine.hpp"
#include "ParentWatchdog.hpp"

#ifndef AUDIO_ENGINE_VERSION_STRING
#define AUDIO_ENGINE_VERSION_STRING "2.3.0"
#endif

static juce::var errObjSimple(const juce::String& msg)
{
    auto* o = new juce::DynamicObject();
    o->setProperty("ok", false);
    o->setProperty("error", msg);
    return o;
}

/** One JSON line in → one JSON line out. Runs on the stdin thread (not the JUCE message thread). */
static void runStdinJsonLoop(audio_haxor::Engine& engine)
{
    audio_haxor::appLogLine("stdin reader thread started");
    std::string line;
    while (std::getline(std::cin, line))
    {
        const juce::String trimmed = juce::String(line).trim();
        if (trimmed.isEmpty())
            continue;

        const auto parsed = juce::JSON::parse(trimmed);
        if (parsed.isVoid())
        {
            audio_haxor::appLogLine("stdin: JSON parse failed");
            std::cout << R"({"ok":false,"error":"bad JSON"})" << '\n' << std::flush;
            continue;
        }

        // Always dispatch on this thread. `callAsync` + blocking on `fut.get()` from a worker thread
        // deadlocks on macOS until `[NSApp run]` is pumping; ordering hacks (atomic gates) could spin
        // forever if that pump never delivered the ready callback. `Engine::dispatch` serializes on
        // `impl->mutex`.
        try
        {
            std::cout << juce::JSON::toString(engine.dispatch(parsed), true) << '\n' << std::flush;
        }
        catch (const std::exception& e)
        {
            audio_haxor::appLogLine(juce::String("dispatch exception: ") + e.what());
            std::cout << juce::JSON::toString(errObjSimple(juce::String("exception: ") + e.what()), true) << '\n'
                      << std::flush;
        }
        catch (...)
        {
            audio_haxor::appLogLine("dispatch exception: ...");
            std::cout << juce::JSON::toString(errObjSimple("internal error"), true) << '\n' << std::flush;
        }
    }
    audio_haxor::appLogLine("stdin closed, exiting");
    juce::MessageManager::callAsync([&engine]() {
        engine.shutdownEditors();
        juce::MessageManager::getInstance()->stopDispatchLoop();
    });
}

int main()
{
    audio_haxor::initAppLogFromEnv();
    audio_haxor::startParentWatchdogFromEnv();
    audio_haxor::installEngineCrashHandlers();
    juce::ScopedJuceInitialiser_GUI juceInit;
    audio_haxor::appLogLine(juce::String("started v") + AUDIO_ENGINE_VERSION_STRING);
    audio_haxor::Engine engine;
    audio_haxor::appLogLine("Engine constructed");

    std::thread stdinThread([&engine]() { runStdinJsonLoop(engine); });

    audio_haxor::appLogLine("entering runDispatchLoop (message thread)");
    juce::MessageManager::getInstance()->runDispatchLoop();
    audio_haxor::appLogLine("runDispatchLoop returned");
    stdinThread.join();
    return 0;
}
