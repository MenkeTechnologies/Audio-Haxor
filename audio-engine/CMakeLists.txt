# audio-engine — JUCE-based AudioEngine (stdin/stdout JSON) for device I/O, file playback, VST3 scan.
cmake_minimum_required(VERSION 3.22)
project(audio_engine VERSION 2.4.4 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(FetchContent)
FetchContent_Declare(
  JUCE
  GIT_REPOSITORY https://github.com/juce-framework/JUCE.git
  GIT_TAG        8.0.4
)
FetchContent_MakeAvailable(JUCE)

juce_add_console_app(audio_engine
  PRODUCT_NAME "audio-engine"
  COMPANY_NAME "MenkeTechnologies"
)

# Tauri AudioEngine must be named `audio-engine` (hyphen), not CMake target `audio_engine`.
set_target_properties(audio_engine PROPERTIES OUTPUT_NAME "audio-engine")

target_sources(audio_engine PRIVATE
  src/Main.cpp
  src/AppLog.cpp
  src/CrashHandler.cpp
  src/ParentWatchdog.cpp
  src/VisualPreview.cpp
  src/Engine.cpp
)

target_compile_definitions(audio_engine PRIVATE
  JUCE_WEB_BROWSER=0
  JUCE_USE_CURL=0
  JUCE_PLUGINHOST_VST3=1
  JUCE_PLUGINHOST_LADSPA=0
  JUCE_PLUGINHOST_LV2=0
)

if(APPLE)
  target_compile_definitions(audio_engine PRIVATE JUCE_PLUGINHOST_AU=1)
else()
  target_compile_definitions(audio_engine PRIVATE JUCE_PLUGINHOST_AU=0)
endif()

target_compile_definitions(audio_engine PRIVATE
  "AUDIO_ENGINE_VERSION_STRING=\"${PROJECT_VERSION}\""
)

target_link_libraries(audio_engine PRIVATE
  juce::juce_audio_basics
  juce::juce_audio_devices
  juce::juce_audio_formats
  juce::juce_audio_processors
  juce::juce_audio_utils
  juce::juce_dsp
  juce::juce_core
  juce::juce_events
  juce::juce_gui_basics
  PUBLIC
  juce::juce_recommended_config_flags
  juce::juce_recommended_warning_flags
)

# Match Rust/Cargo layout: target/debug/audio-engine, target/release/audio-engine
if(CMAKE_CONFIGURATION_TYPES)
  set_target_properties(audio_engine PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY_DEBUG "${CMAKE_SOURCE_DIR}/../target/debug"
    RUNTIME_OUTPUT_DIRECTORY_RELEASE "${CMAKE_SOURCE_DIR}/../target/release"
    RUNTIME_OUTPUT_DIRECTORY_MINSIZEREL "${CMAKE_SOURCE_DIR}/../target/release"
    RUNTIME_OUTPUT_DIRECTORY_RELWITHDEBINFO "${CMAKE_SOURCE_DIR}/../target/release"
  )
else()
  if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release CACHE STRING "" FORCE)
  endif()
  string(TOLOWER "${CMAKE_BUILD_TYPE}" _audio_engine_bt)
  set_target_properties(audio_engine PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_SOURCE_DIR}/../target/${_audio_engine_bt}"
  )
endif()
