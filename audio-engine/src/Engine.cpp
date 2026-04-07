#include "Engine.hpp"

#include <bit>
#include <cmath>
#include <memory>
#include <mutex>
#include <optional>
#include <unordered_map>

#include <juce_audio_devices/juce_audio_devices.h>
#include <juce_audio_formats/juce_audio_formats.h>
#include <juce_audio_processors/juce_audio_processors.h>
#include <juce_audio_utils/juce_audio_utils.h>
#include <juce_dsp/juce_dsp.h>

namespace audio_haxor {
namespace {

#ifndef AUDIO_ENGINE_VERSION_STRING
#define AUDIO_ENGINE_VERSION_STRING "2.0.0"
#endif

static constexpr float kTestToneHz = 440.0f;
static constexpr float kTestToneGain = 0.05f;
static constexpr float kInputPeakDecay = 0.95f;
static constexpr uint32_t kMaxBufferFrames = 8192;

static juce::var errObj(const juce::String& msg)
{
    auto* o = new juce::DynamicObject();
    o->setProperty("ok", false);
    o->setProperty("error", msg);
    return o;
}

static juce::var okObj()
{
    auto* o = new juce::DynamicObject();
    o->setProperty("ok", true);
    return o;
}

static juce::String cmdKey(const juce::var& req)
{
    if (req.isObject())
        return req["cmd"].toString().toLowerCase();
    return {};
}

static juce::var bufferSizeJson(juce::AudioIODevice* dev)
{
    if (dev == nullptr)
        return juce::var();

    const juce::Array<int> sizes = dev->getAvailableBufferSizes();
    if (sizes.isEmpty())
    {
        auto* o = new juce::DynamicObject();
        o->setProperty("kind", "unknown");
        return o;
    }

    int mn = sizes.getFirst();
    int mx = sizes.getFirst();
    for (int s : sizes)
    {
        mn = juce::jmin(mn, s);
        mx = juce::jmax(mx, s);
    }
    auto* o = new juce::DynamicObject();
    o->setProperty("kind", "range");
    o->setProperty("min", mn);
    o->setProperty("max", mx);
    return o;
}

static juce::String uniqueDeviceId(const juce::String& name, std::unordered_map<juce::String, uint32_t>& seen)
{
    const auto it = seen.find(name);
    if (it == seen.end())
    {
        seen[name] = 1;
        return name;
    }
    it->second += 1;
    return name + "#" + juce::String((int) it->second);
}

static void enumerateOutputIds(juce::StringArray& outIds, juce::StringArray& outNames)
{
    outIds.clear();
    outNames.clear();
    juce::AudioDeviceManager dm;
    dm.initialise(0, 2, nullptr, true);
    juce::AudioIODeviceType* t = dm.getCurrentDeviceTypeObject();
    if (t == nullptr)
        return;
    const juce::StringArray names = t->getDeviceNames(false);
    std::unordered_map<juce::String, uint32_t> seen;
    for (int i = 0; i < names.size(); ++i)
    {
        const juce::String& n = names[i];
        outNames.add(n);
        outIds.add(uniqueDeviceId(n, seen));
    }
}

static void enumerateInputIds(juce::StringArray& outIds, juce::StringArray& outNames)
{
    outIds.clear();
    outNames.clear();
    juce::AudioDeviceManager dm;
    dm.initialise(2, 0, nullptr, true);
    juce::AudioIODeviceType* t = dm.getCurrentDeviceTypeObject();
    if (t == nullptr)
        return;
    const juce::StringArray names = t->getDeviceNames(true);
    std::unordered_map<juce::String, uint32_t> seen;
    for (int i = 0; i < names.size(); ++i)
    {
        const juce::String& n = names[i];
        outNames.add(n);
        outIds.add(uniqueDeviceId(n, seen));
    }
}

static juce::String resolveOutputDeviceName(const juce::String& id)
{
    juce::StringArray ids, names;
    enumerateOutputIds(ids, names);
    if (id.isEmpty())
    {
        juce::AudioDeviceManager dm;
        dm.initialise(0, 2, nullptr, true);
        juce::AudioIODevice* dev = dm.getCurrentAudioDevice();
        return dev != nullptr ? dev->getName() : juce::String();
    }
    for (int i = 0; i < ids.size(); ++i)
        if (ids[i] == id)
            return names[i];
    if (id.containsOnly("0123456789"))
    {
        const int idx = id.getIntValue();
        if (idx >= 0 && idx < names.size())
            return names[idx];
    }
    return {};
}

static juce::String resolveInputDeviceName(const juce::String& id)
{
    juce::StringArray ids, names;
    enumerateInputIds(ids, names);
    if (id.isEmpty())
    {
        juce::AudioDeviceManager dm;
        dm.initialise(2, 0, nullptr, true);
        juce::AudioIODevice* dev = dm.getCurrentAudioDevice();
        return dev != nullptr ? dev->getName() : juce::String();
    }
    for (int i = 0; i < ids.size(); ++i)
        if (ids[i] == id)
            return names[i];
    if (id.containsOnly("0123456789"))
    {
        const int idx = id.getIntValue();
        if (idx >= 0 && idx < names.size())
            return names[idx];
    }
    return {};
}

static juce::String outputIdForDeviceName(const juce::String& deviceName)
{
    juce::StringArray ids, names;
    enumerateOutputIds(ids, names);
    for (int i = 0; i < names.size(); ++i)
        if (names[i] == deviceName)
            return ids[i];
    return deviceName;
}

static juce::String inputIdForDeviceName(const juce::String& deviceName)
{
    juce::StringArray ids, names;
    enumerateInputIds(ids, names);
    for (int i = 0; i < names.size(); ++i)
        if (names[i] == deviceName)
            return ids[i];
    return deviceName;
}

struct DspAtomics
{
    std::atomic<uint32_t> gainBits{std::bit_cast<uint32_t>(1.0f)};
    std::atomic<uint32_t> panBits{std::bit_cast<uint32_t>(0.0f)};
    std::atomic<uint32_t> eqLowBits{std::bit_cast<uint32_t>(0.0f)};
    std::atomic<uint32_t> eqMidBits{std::bit_cast<uint32_t>(0.0f)};
    std::atomic<uint32_t> eqHighBits{std::bit_cast<uint32_t>(0.0f)};
};

static float loadF(const std::atomic<uint32_t>& a)
{
    return std::bit_cast<float>(a.load());
}

static void applyDspFrame(float& l, float& r, double sr, const DspAtomics& dsp, juce::dsp::IIR::Filter<float>& lowL,
                          juce::dsp::IIR::Filter<float>& lowR, juce::dsp::IIR::Filter<float>& midL,
                          juce::dsp::IIR::Filter<float>& midR, juce::dsp::IIR::Filter<float>& hiL,
                          juce::dsp::IIR::Filter<float>& hiR)
{
    const float g = juce::jlimit(0.0f, 4.0f, loadF(dsp.gainBits));
    const float pan = juce::jlimit(-1.0f, 1.0f, loadF(dsp.panBits));
    const float lowDb = loadF(dsp.eqLowBits);
    const float midDb = loadF(dsp.eqMidBits);
    const float highDb = loadF(dsp.eqHighBits);

    auto lowCoef = juce::dsp::IIR::Coefficients<float>::makeLowShelf(sr, 200.0, 0.707f, juce::Decibels::decibelsToGain(lowDb));
    auto midCoef = juce::dsp::IIR::Coefficients<float>::makePeakFilter(sr, 1000.0, 1.0f, juce::Decibels::decibelsToGain(midDb));
    auto hiCoef = juce::dsp::IIR::Coefficients<float>::makeHighShelf(sr, 8000.0, 0.707f, juce::Decibels::decibelsToGain(highDb));
    *lowL.coefficients = *lowCoef;
    *lowR.coefficients = *lowCoef;
    *midL.coefficients = *midCoef;
    *midR.coefficients = *midCoef;
    *hiL.coefficients = *hiCoef;
    *hiR.coefficients = *hiCoef;

    double dl = (double) l;
    double dr = (double) r;
    dl = (double) lowL.processSample((float) dl);
    dr = (double) lowR.processSample((float) dr);
    dl = (double) midL.processSample((float) dl);
    dr = (double) midR.processSample((float) dr);
    dl = (double) hiL.processSample((float) dl);
    dr = (double) hiR.processSample((float) dr);
    dl *= (double) g;
    dr *= (double) g;
    const double ang = ((double) pan + 1.0) * juce::MathConstants<double>::halfPi / 2.0;
    l = (float) (dl * std::cos(ang));
    r = (float) (dr * std::sin(ang));
}

class ToneAudioSource final : public juce::AudioSource
{
public:
    std::atomic<bool> toneOn{false};
    std::atomic<uint64_t> phase{0};

    void prepareToPlay(int, double sampleRate) override { sr = sampleRate; }
    void releaseResources() override {}

    void getNextAudioBlock(const juce::AudioSourceChannelInfo& bufferToFill) override
    {
        if (bufferToFill.buffer == nullptr)
            return;
        const int ch = bufferToFill.buffer->getNumChannels();
        const int n = bufferToFill.numSamples;
        if (!toneOn.load())
        {
            for (int c = 0; c < ch; ++c)
                bufferToFill.buffer->clear(c, bufferToFill.startSample, n);
            phase.fetch_add((uint64_t) n);
            return;
        }
        uint64_t p = phase.load();
        const double twoPi = juce::MathConstants<double>::twoPi;
        for (int i = 0; i < n; ++i)
        {
            const float s = (float) (std::sin((double) p * twoPi * (double) kTestToneHz / sr) * (double) kTestToneGain);
            for (int c = 0; c < ch; ++c)
                bufferToFill.buffer->setSample(c, bufferToFill.startSample + i, s);
            ++p;
        }
        phase.store(p);
    }

private:
    double sr = 44100.0;
};

class DspStereoFileSource final : public juce::PositionableAudioSource
{
public:
    std::unique_ptr<juce::AudioFormatReaderSource> readerSource;
    juce::AudioBuffer<float> reverseStereo;
    bool reverseMode = false;
    int reverseFrame = 0;
    DspAtomics* dsp = nullptr;
    std::atomic<float>* peak = nullptr;
    juce::dsp::IIR::Filter<float> lowL, lowR, midL, midR, hiL, hiR;
    double processRate = 44100.0;

    void prepareToPlay(int samplesPerBlockExpected, double sampleRate) override
    {
        processRate = sampleRate;
        juce::dsp::ProcessSpec spec;
        spec.maximumBlockSize = (juce::uint32) juce::jmax(1, samplesPerBlockExpected);
        spec.sampleRate = sampleRate;
        spec.numChannels = 2;
        lowL.prepare(spec);
        lowR.prepare(spec);
        midL.prepare(spec);
        midR.prepare(spec);
        hiL.prepare(spec);
        hiR.prepare(spec);
        if (readerSource != nullptr)
            readerSource->prepareToPlay(samplesPerBlockExpected, sampleRate);
    }

    void releaseResources() override
    {
        if (readerSource != nullptr)
            readerSource->releaseResources();
    }

    void setNextReadPosition(juce::int64 newPosition) override
    {
        if (reverseMode)
        {
            const int frames = reverseStereo.getNumSamples();
            if (frames <= 0)
                reverseFrame = 0;
            else
                reverseFrame = (int) juce::jlimit<juce::int64>(0, (juce::int64) frames - 1, newPosition);
        }
        else if (readerSource != nullptr)
        {
            readerSource->setNextReadPosition(newPosition);
        }
    }

    juce::int64 getNextReadPosition() const override
    {
        if (reverseMode)
            return (juce::int64) reverseFrame;
        if (readerSource != nullptr)
            return readerSource->getNextReadPosition();
        return 0;
    }

    juce::int64 getTotalLength() const override
    {
        if (reverseMode)
            return (juce::int64) reverseStereo.getNumSamples();
        if (readerSource != nullptr)
            return readerSource->getTotalLength();
        return 0;
    }

    bool isLooping() const override
    {
        if (readerSource != nullptr)
            return readerSource->isLooping();
        return false;
    }

    void setLooping(bool shouldLoop) override
    {
        if (readerSource != nullptr)
            readerSource->setLooping(shouldLoop);
    }

    void getNextAudioBlock(const juce::AudioSourceChannelInfo& bufferToFill) override
    {
        if (bufferToFill.buffer == nullptr || dsp == nullptr)
            return;

        const int n = bufferToFill.numSamples;
        if (reverseMode && reverseStereo.getNumChannels() >= 2 && reverseStereo.getNumSamples() > 0)
        {
            const int frames = reverseStereo.getNumSamples();
            for (int i = 0; i < n; ++i)
            {
                if (reverseFrame >= frames)
                {
                    bufferToFill.buffer->clear(bufferToFill.startSample, n - i);
                    break;
                }
                const int fi = frames - 1 - reverseFrame;
                float l = reverseStereo.getSample(0, fi);
                float r = reverseStereo.getSample(1, fi);
                ++reverseFrame;
                applyDspFrame(l, r, processRate, *dsp, lowL, lowR, midL, midR, hiL, hiR);
                bufferToFill.buffer->setSample(0, bufferToFill.startSample + i, l);
                bufferToFill.buffer->setSample(1, bufferToFill.startSample + i, r);
                if (peak != nullptr)
                {
                    float pk = peak->load();
                    pk = juce::jmax(pk, std::abs(l), std::abs(r));
                    peak->store(pk);
                }
            }
            return;
        }

        if (readerSource == nullptr)
        {
            bufferToFill.clearActiveBufferRegion();
            return;
        }

        readerSource->getNextAudioBlock(bufferToFill);

        if (readerSource->getAudioFormatReader() != nullptr && readerSource->getAudioFormatReader()->numChannels == 1)
        {
            for (int i = 0; i < n; ++i)
            {
                const float x = bufferToFill.buffer->getSample(0, bufferToFill.startSample + i);
                bufferToFill.buffer->setSample(1, bufferToFill.startSample + i, x);
            }
        }

        for (int i = 0; i < n; ++i)
        {
            float l = bufferToFill.buffer->getSample(0, bufferToFill.startSample + i);
            float r = bufferToFill.buffer->getSample(1, bufferToFill.startSample + i);
            applyDspFrame(l, r, processRate, *dsp, lowL, lowR, midL, midR, hiL, hiR);
            bufferToFill.buffer->setSample(0, bufferToFill.startSample + i, l);
            bufferToFill.buffer->setSample(1, bufferToFill.startSample + i, r);
            if (peak != nullptr)
            {
                float pk = peak->load();
                pk = juce::jmax(pk, std::abs(l), std::abs(r));
                peak->store(pk);
            }
        }
    }
};

class InputPeakCallback final : public juce::AudioIODeviceCallback
{
public:
    std::atomic<float> peak{0.0f};

    void audioDeviceIOCallbackWithContext(const float* const* inputChannelData, int numInputChannels, float* const* outputChannelData,
                                          int numOutputChannels, int numSamples, const juce::AudioIODeviceCallbackContext&) override
    {
        juce::ignoreUnused(outputChannelData, numOutputChannels);
        float m = 0.0f;
        if (inputChannelData != nullptr && numInputChannels > 0 && numSamples > 0)
        {
            for (int ch = 0; ch < numInputChannels; ++ch)
            {
                const float* row = inputChannelData[ch];
                if (row == nullptr)
                    continue;
                for (int i = 0; i < numSamples; ++i)
                    m = juce::jmax(m, std::abs(row[i]));
            }
        }
        const float old = peak.load();
        const float next = m > old ? m : old * kInputPeakDecay;
        peak.store(juce::jmin(1.0f, next));
    }

    void audioDeviceAboutToStart(juce::AudioIODevice*) override {}
    void audioDeviceStopped() override {}
};

} // namespace

struct Engine::Impl
{
    std::mutex mutex;
    juce::AudioDeviceManager outputManager;
    juce::AudioDeviceManager inputManager;
    juce::AudioSourcePlayer sourcePlayer;
    juce::AudioTransportSource transport;
    juce::AudioFormatManager formatManager;
    ToneAudioSource toneSource;
    std::unique_ptr<DspStereoFileSource> fileSource;
    std::atomic<float> playbackPeak{0.0f};
    DspAtomics dsp;

    InputPeakCallback inputCb;
    bool outputRunning = false;
    bool inputRunning = false;
    bool playbackMode = false;
    bool toneMode = false;

    juce::String outDeviceId;
    juce::String outDeviceName;
    int outSampleRate = 0;
    int outChannels = 2;
    juce::var outBufferSizeJson;
    std::optional<int> outStreamBufferFrames;

    juce::String inDeviceId;
    juce::String inDeviceName;
    int inSampleRate = 0;
    int inChannels = 2;
    juce::var inBufferSizeJson;
    std::optional<int> inStreamBufferFrames;

    juce::String sessionPath;
    double sessionDurationSec = 0.0;
    uint32_t sessionSrcRate = 44100;
    std::atomic<uint32_t> deviceRate{0};
    bool reverseWanted = false;
    bool paused = false;

    juce::KnownPluginList pluginList;
    juce::VST3PluginFormat vst3;
#if JUCE_MAC
    juce::AudioUnitPluginFormat auFormat;
#endif
    bool pluginScanDone = false;

    Impl()
    {
        formatManager.registerBasicFormats();
        outputManager.initialise(0, 2, nullptr, true);
        inputManager.initialise(2, 0, nullptr, true);
    }

    void scanPluginsOnce()
    {
        if (pluginScanDone)
            return;
        juce::String name;
        {
            const juce::FileSearchPath dirs = vst3.getDefaultLocationsToSearch();
            juce::PluginDirectoryScanner scanner(pluginList, vst3, dirs, true, {});
            while (scanner.scanNextFile(true, name))
            {
            }
        }
#if JUCE_MAC
        {
            const juce::FileSearchPath auDirs = auFormat.getDefaultLocationsToSearch();
            juce::PluginDirectoryScanner scanner(pluginList, auFormat, auDirs, true, {});
            while (scanner.scanNextFile(true, name))
            {
            }
        }
#endif
        pluginScanDone = true;
    }

    void stopOutputLocked()
    {
        outputManager.removeAudioCallback(&sourcePlayer);
        sourcePlayer.setSource(nullptr);
        transport.setSource(nullptr);
        transport.stop();
        transport.releaseResources();
        fileSource.reset();
        outputManager.closeAudioDevice();
        outputRunning = false;
        playbackMode = false;
        toneMode = false;
        playbackPeak.store(0.0f);
    }

    void stopInputLocked()
    {
        inputManager.removeAudioCallback(&inputCb);
        inputManager.closeAudioDevice();
        inputRunning = false;
        inputCb.peak.store(0.0f);
    }

    juce::var playbackLoad(const juce::var& req)
    {
        const juce::String path = req["path"].toString();
        if (path.isEmpty())
            return errObj("path required");
        const juce::File f(path);
        if (!f.existsAsFile())
            return errObj("not a file: " + path);
        std::unique_ptr<juce::AudioFormatReader> reader(formatManager.createReaderFor(f));
        if (reader == nullptr)
            return errObj("unsupported or unreadable file");
        sessionPath = path;
        sessionSrcRate = (uint32_t) reader->sampleRate;
        sessionDurationSec = (double) reader->lengthInSamples / juce::jmax(1.0, reader->sampleRate);
        reverseWanted = false;
        paused = false;
        playbackPeak.store(0.0f);
        auto* o = okObj().getDynamicObject();
        o->setProperty("duration_sec", sessionDurationSec);
        o->setProperty("sample_rate_hz", (int) sessionSrcRate);
        o->setProperty("track_id", 0);
        return o;
    }

    juce::var playbackStopLocked()
    {
        transport.stop();
        transport.setSource(nullptr);
        transport.releaseResources();
        fileSource.reset();
        playbackMode = false;
        sessionPath.clear();
        sessionDurationSec = 0.0;
        if (outputRunning)
        {
            toneSource.toneOn.store(false);
            sourcePlayer.setSource(&toneSource);
        }
        return okObj();
    }

    juce::var startOutputStreamLocked(const juce::var& req)
    {
        const bool startPlayback = req.hasProperty("start_playback") && (bool) req["start_playback"];
        const bool tone = req.hasProperty("tone") && (bool) req["tone"];
        const juce::String deviceId = req["device_id"].toString();
        uint32_t bf = 0;
        if (req.hasProperty("buffer_frames") && !req["buffer_frames"].isVoid())
            bf = (uint32_t) (int) req["buffer_frames"];
        if (bf > kMaxBufferFrames)
            bf = kMaxBufferFrames;

        stopOutputLocked();

        if (startPlayback && sessionPath.isEmpty())
            return errObj("playback_load required before start_playback");

        juce::String devName = resolveOutputDeviceName(deviceId);
        if (devName.isEmpty() && !deviceId.isEmpty())
            return errObj("unknown device_id: " + deviceId);

        juce::AudioDeviceManager::AudioDeviceSetup setup;
        if (devName.isNotEmpty())
            setup.outputDeviceName = devName;
        setup.inputDeviceName = "";
        if (bf > 0)
            setup.bufferSize = (int) bf;

        if (startPlayback)
        {
            std::unique_ptr<juce::AudioFormatReader> probe(formatManager.createReaderFor(juce::File(sessionPath)));
            if (probe == nullptr)
                return errObj("open file failed");
            setup.sampleRate = probe->sampleRate;
        }

        outputManager.setAudioDeviceSetup(setup, true);
        juce::AudioIODevice* dev = outputManager.getCurrentAudioDevice();
        if (dev == nullptr)
            return errObj("no output device");

        deviceRate.store((uint32_t) dev->getCurrentSampleRate());

        outDeviceId = outputIdForDeviceName(dev->getName());
        outDeviceName = dev->getName();
        outSampleRate = (int) dev->getCurrentSampleRate();
        outChannels = juce::jmax(1, dev->getActiveOutputChannels().countNumberOfSetBits());
        outBufferSizeJson = bufferSizeJson(dev);
        outStreamBufferFrames = (bf > 0) ? std::optional<int>((int) bf) : std::nullopt;

        if (startPlayback)
        {
            fileSource = std::make_unique<DspStereoFileSource>();
            fileSource->dsp = &dsp;
            fileSource->peak = &playbackPeak;
            fileSource->reverseMode = reverseWanted;
            fileSource->reverseFrame = 0;

            if (reverseWanted)
            {
                std::unique_ptr<juce::AudioFormatReader> reader(formatManager.createReaderFor(juce::File(sessionPath)));
                if (reader == nullptr)
                    return errObj("open file failed");
                const int nFrames = (int) reader->lengthInSamples;
                if (nFrames <= 0)
                    return errObj("empty audio");
                fileSource->reverseStereo.setSize(2, nFrames);
                if (reader->numChannels >= 2)
                {
                    reader->read(&fileSource->reverseStereo, 0, nFrames, 0, true, true);
                }
                else
                {
                    juce::AudioBuffer<float> m(1, nFrames);
                    reader->read(&m, 0, nFrames, 0, true, true);
                    fileSource->reverseStereo.copyFrom(0, 0, m, 0, 0, nFrames);
                    fileSource->reverseStereo.copyFrom(1, 0, m, 0, 0, nFrames);
                }
                for (int i = 0; i < nFrames / 2; ++i)
                {
                    const int j = nFrames - 1 - i;
                    for (int c = 0; c < 2; ++c)
                    {
                        const float a = fileSource->reverseStereo.getSample(c, i);
                        const float b = fileSource->reverseStereo.getSample(c, j);
                        fileSource->reverseStereo.setSample(c, i, b);
                        fileSource->reverseStereo.setSample(c, j, a);
                    }
                }
            }
            else
            {
                std::unique_ptr<juce::AudioFormatReader> reader(formatManager.createReaderFor(juce::File(sessionPath)));
                if (reader == nullptr)
                    return errObj("open file failed");
                auto* raw = reader.release();
                fileSource->readerSource = std::make_unique<juce::AudioFormatReaderSource>(raw, true);
            }

            transport.setSource(fileSource.get(), 0, nullptr, (double) sessionSrcRate);
            sourcePlayer.setSource(&transport);
            outputManager.addAudioCallback(&sourcePlayer);
            transport.start();
            playbackMode = true;
            toneMode = false;
        }
        else
        {
            toneMode = true;
            toneSource.toneOn.store(tone);
            toneSource.phase.store(0);
            sourcePlayer.setSource(&toneSource);
            outputManager.addAudioCallback(&sourcePlayer);
            playbackMode = false;
        }

        outputRunning = true;

        auto* o = okObj().getDynamicObject();
        o->setProperty("device_id", outDeviceId);
        o->setProperty("device_name", outDeviceName);
        o->setProperty("sample_rate_hz", outSampleRate);
        o->setProperty("channels", outChannels);
        o->setProperty("sample_format", juce::String("F32"));
        o->setProperty("buffer_size", outBufferSizeJson);
        o->setProperty("stream_buffer_frames", outStreamBufferFrames.has_value() ? juce::var(*outStreamBufferFrames) : juce::var());
        o->setProperty("tone_supported", true);
        o->setProperty("tone_on", !startPlayback && tone);
        o->setProperty("note", startPlayback ? juce::String("file playback via JUCE") : juce::String("silence or test tone"));
        return o;
    }

    juce::var startInputStreamLocked(const juce::var& req)
    {
        uint32_t bf = 0;
        if (req.hasProperty("buffer_frames") && !req["buffer_frames"].isVoid())
            bf = (uint32_t) (int) req["buffer_frames"];
        if (bf > kMaxBufferFrames)
            bf = kMaxBufferFrames;

        stopInputLocked();

        const juce::String deviceId = req["device_id"].toString();
        juce::String devName = resolveInputDeviceName(deviceId);
        if (devName.isEmpty() && !deviceId.isEmpty())
            return errObj("unknown device_id: " + deviceId);

        juce::AudioDeviceManager::AudioDeviceSetup setup;
        if (devName.isNotEmpty())
            setup.inputDeviceName = devName;
        setup.outputDeviceName = "";
        if (bf > 0)
            setup.bufferSize = (int) bf;

        inputManager.setAudioDeviceSetup(setup, true);
        juce::AudioIODevice* dev = inputManager.getCurrentAudioDevice();
        if (dev == nullptr)
            return errObj("no input device");

        inDeviceId = inputIdForDeviceName(dev->getName());
        inDeviceName = dev->getName();
        inSampleRate = (int) dev->getCurrentSampleRate();
        inChannels = juce::jmax(1, dev->getActiveInputChannels().countNumberOfSetBits());
        inBufferSizeJson = bufferSizeJson(dev);
        inStreamBufferFrames = (bf > 0) ? std::optional<int>((int) bf) : std::nullopt;

        inputCb.peak.store(0.0f);
        inputManager.addAudioCallback(&inputCb);

        inputRunning = true;

        auto* o = okObj().getDynamicObject();
        o->setProperty("device_id", inDeviceId);
        o->setProperty("device_name", inDeviceName);
        o->setProperty("sample_rate_hz", inSampleRate);
        o->setProperty("channels", inChannels);
        o->setProperty("sample_format", juce::String("F32"));
        o->setProperty("buffer_size", inBufferSizeJson);
        o->setProperty("stream_buffer_frames", inStreamBufferFrames.has_value() ? juce::var(*inStreamBufferFrames) : juce::var());
        o->setProperty("input_peak", 0.0);
        o->setProperty("note", "input capture running; samples discarded; input_peak is block peak with decay");
        return o;
    }

    juce::var playbackStatusLocked()
    {
        if (sessionPath.isEmpty())
        {
            auto* o = okObj().getDynamicObject();
            o->setProperty("loaded", false);
            return o;
        }
        auto* o = okObj().getDynamicObject();
        o->setProperty("loaded", true);
        o->setProperty("duration_sec", sessionDurationSec);
        o->setProperty("sample_rate_hz", (int) deviceRate.load());
        o->setProperty("src_rate_hz", (int) sessionSrcRate);
        o->setProperty("reverse", reverseWanted);
        if (!playbackMode)
        {
            o->setProperty("position_sec", 0.0);
            o->setProperty("peak", playbackPeak.load());
            o->setProperty("paused", false);
            o->setProperty("eof", false);
            return o;
        }
        const double posSrc = transport.getCurrentPosition();
        double pos = reverseWanted ? (sessionDurationSec - posSrc) : posSrc;
        pos = juce::jlimit(0.0, sessionDurationSec, pos);
        o->setProperty("position_sec", pos);
        o->setProperty("peak", playbackPeak.load());
        o->setProperty("paused", paused);
        o->setProperty("eof", transport.hasStreamFinished());
        return o;
    }

    juce::var outputStreamStatusLocked()
    {
        if (!outputRunning)
        {
            auto* o = new juce::DynamicObject();
            o->setProperty("ok", true);
            o->setProperty("running", false);
            o->setProperty("device_id", juce::var());
            o->setProperty("device_name", juce::var());
            o->setProperty("sample_rate_hz", juce::var());
            o->setProperty("channels", juce::var());
            o->setProperty("sample_format", juce::var());
            o->setProperty("buffer_size", juce::var());
            o->setProperty("stream_buffer_frames", juce::var());
            o->setProperty("tone_supported", juce::var());
            o->setProperty("tone_on", juce::var());
            return o;
        }

        auto* o = new juce::DynamicObject();
        o->setProperty("ok", true);
        o->setProperty("running", true);
        o->setProperty("device_id", outDeviceId);
        o->setProperty("device_name", outDeviceName);
        o->setProperty("sample_rate_hz", outSampleRate);
        o->setProperty("channels", outChannels);
        o->setProperty("sample_format", juce::String("F32"));
        o->setProperty("buffer_size", outBufferSizeJson);
        o->setProperty("stream_buffer_frames", outStreamBufferFrames.has_value() ? juce::var(*outStreamBufferFrames) : juce::var());
        o->setProperty("tone_supported", true);
        o->setProperty("tone_on", toneMode && toneSource.toneOn.load());
        return o;
    }

    juce::var inputStreamStatusLocked()
    {
        if (!inputRunning)
        {
            auto* o = new juce::DynamicObject();
            o->setProperty("ok", true);
            o->setProperty("running", false);
            o->setProperty("device_id", juce::var());
            o->setProperty("device_name", juce::var());
            o->setProperty("sample_rate_hz", juce::var());
            o->setProperty("channels", juce::var());
            o->setProperty("sample_format", juce::var());
            o->setProperty("buffer_size", juce::var());
            o->setProperty("stream_buffer_frames", juce::var());
            o->setProperty("input_peak", juce::var());
            return o;
        }

        auto* o = new juce::DynamicObject();
        o->setProperty("ok", true);
        o->setProperty("running", true);
        o->setProperty("device_id", inDeviceId);
        o->setProperty("device_name", inDeviceName);
        o->setProperty("sample_rate_hz", inSampleRate);
        o->setProperty("channels", inChannels);
        o->setProperty("sample_format", juce::String("F32"));
        o->setProperty("buffer_size", inBufferSizeJson);
        o->setProperty("stream_buffer_frames", inStreamBufferFrames.has_value() ? juce::var(*inStreamBufferFrames) : juce::var());
        o->setProperty("input_peak", inputCb.peak.load());
        return o;
    }

    juce::var engineStateLocked()
    {
        auto* o = new juce::DynamicObject();
        o->setProperty("ok", true);
        o->setProperty("version", juce::String(AUDIO_ENGINE_VERSION_STRING));
        o->setProperty("host", juce::String("juce"));
        o->setProperty("stream", outputStreamStatusLocked());
        o->setProperty("input_stream", inputStreamStatusLocked());
        return o;
    }
};

Engine::Engine() : impl(std::make_unique<Impl>()) {}
Engine::~Engine() = default;

juce::var Engine::dispatch(const juce::var& req)
{
    std::lock_guard<std::mutex> lock(impl->mutex);
    const juce::String cmd = cmdKey(req);

    if (cmd == "ping")
    {
        auto* o = new juce::DynamicObject();
        o->setProperty("ok", true);
        o->setProperty("version", juce::String(AUDIO_ENGINE_VERSION_STRING));
        o->setProperty("host", juce::String("juce"));
        return o;
    }

    if (cmd == "engine_state")
        return impl->engineStateLocked();

    if (cmd == "output_stream_status")
        return impl->outputStreamStatusLocked();

    if (cmd == "input_stream_status")
        return impl->inputStreamStatusLocked();

    if (cmd == "list_output_devices")
    {
        juce::StringArray ids, names;
        enumerateOutputIds(ids, names);
        juce::String defaultId;
        juce::AudioDeviceManager dm;
        dm.initialise(0, 2, nullptr, true);
        juce::AudioIODevice* cur = dm.getCurrentAudioDevice();
        const juce::String curName = cur != nullptr ? cur->getName() : juce::String();
        for (int i = 0; i < names.size(); ++i)
            if (names[i] == curName)
                defaultId = ids[i];
        juce::Array<juce::var> rows;
        for (int i = 0; i < ids.size(); ++i)
        {
            auto* row = new juce::DynamicObject();
            row->setProperty("id", ids[i]);
            row->setProperty("name", names[i]);
            row->setProperty("is_default", ids[i] == defaultId);
            rows.add(row);
        }
        auto* o = okObj().getDynamicObject();
        o->setProperty("default_device_id", defaultId.isEmpty() ? juce::var() : juce::var(defaultId));
        o->setProperty("devices", juce::var(rows));
        return o;
    }

    if (cmd == "list_input_devices")
    {
        juce::StringArray ids, names;
        enumerateInputIds(ids, names);
        juce::String defaultId;
        juce::AudioDeviceManager dm;
        dm.initialise(2, 0, nullptr, true);
        juce::AudioIODevice* cur = dm.getCurrentAudioDevice();
        const juce::String curName = cur != nullptr ? cur->getName() : juce::String();
        for (int i = 0; i < names.size(); ++i)
            if (names[i] == curName)
                defaultId = ids[i];
        juce::Array<juce::var> rows;
        for (int i = 0; i < ids.size(); ++i)
        {
            auto* row = new juce::DynamicObject();
            row->setProperty("id", ids[i]);
            row->setProperty("name", names[i]);
            row->setProperty("is_default", ids[i] == defaultId);
            rows.add(row);
        }
        auto* o = okObj().getDynamicObject();
        o->setProperty("default_device_id", defaultId.isEmpty() ? juce::var() : juce::var(defaultId));
        o->setProperty("devices", juce::var(rows));
        return o;
    }

    if (cmd == "get_output_device_info")
    {
        const juce::String id = req["device_id"].toString();
        const juce::String name = resolveOutputDeviceName(id);
        if (name.isEmpty() && !id.isEmpty())
            return errObj("unknown device_id: " + id);
        juce::AudioDeviceManager dm;
        dm.initialise(0, 2, nullptr, true);
        juce::AudioDeviceManager::AudioDeviceSetup setup;
        setup.outputDeviceName = name.isEmpty() ? (dm.getCurrentAudioDevice() != nullptr ? dm.getCurrentAudioDevice()->getName() : juce::String()) : name;
        setup.inputDeviceName = "";
        dm.setAudioDeviceSetup(setup, true);
        juce::AudioIODevice* dev = dm.getCurrentAudioDevice();
        if (dev == nullptr)
            return errObj("no output device");
        auto* o = okObj().getDynamicObject();
        o->setProperty("device_name", dev->getName());
        o->setProperty("sample_rate_hz", dev->getCurrentSampleRate());
        o->setProperty("channels", dev->getActiveOutputChannels().countNumberOfSetBits());
        o->setProperty("sample_format", juce::String("F32"));
        o->setProperty("buffer_size", bufferSizeJson(dev));
        return o;
    }

    if (cmd == "get_input_device_info")
    {
        const juce::String id = req["device_id"].toString();
        const juce::String name = resolveInputDeviceName(id);
        if (name.isEmpty() && !id.isEmpty())
            return errObj("unknown device_id: " + id);
        juce::AudioDeviceManager dm;
        dm.initialise(2, 0, nullptr, true);
        juce::AudioDeviceManager::AudioDeviceSetup setup;
        setup.inputDeviceName = name.isEmpty() ? (dm.getCurrentAudioDevice() != nullptr ? dm.getCurrentAudioDevice()->getName() : juce::String()) : name;
        setup.outputDeviceName = "";
        dm.setAudioDeviceSetup(setup, true);
        juce::AudioIODevice* dev = dm.getCurrentAudioDevice();
        if (dev == nullptr)
            return errObj("no input device");
        auto* o = okObj().getDynamicObject();
        o->setProperty("device_name", dev->getName());
        o->setProperty("sample_rate_hz", dev->getCurrentSampleRate());
        o->setProperty("channels", dev->getActiveInputChannels().countNumberOfSetBits());
        o->setProperty("sample_format", juce::String("F32"));
        o->setProperty("buffer_size", bufferSizeJson(dev));
        return o;
    }

    if (cmd == "set_output_device")
    {
        const juce::String id = req["device_id"].toString();
        if (id.isEmpty())
            return errObj("device_id required");
        if (resolveOutputDeviceName(id).isEmpty())
            return errObj("unknown device_id: " + id);
        auto* o = okObj().getDynamicObject();
        o->setProperty("device_id", id);
        o->setProperty("note", "validated; use start_output_stream to open the device");
        return o;
    }

    if (cmd == "set_input_device")
    {
        const juce::String id = req["device_id"].toString();
        if (id.isEmpty())
            return errObj("device_id required");
        if (resolveInputDeviceName(id).isEmpty())
            return errObj("unknown device_id: " + id);
        auto* o = okObj().getDynamicObject();
        o->setProperty("device_id", id);
        o->setProperty("note", "validated; use start_input_stream to open capture");
        return o;
    }

    if (cmd == "start_output_stream")
        return impl->startOutputStreamLocked(req);

    if (cmd == "start_input_stream")
        return impl->startInputStreamLocked(req);

    if (cmd == "playback_load")
        return impl->playbackLoad(req);

    if (cmd == "playback_stop")
        return impl->playbackStopLocked();

    if (cmd == "playback_pause")
    {
        const bool p = req["paused"].isVoid() ? true : (bool) req["paused"];
        if (impl->playbackMode)
        {
            if (p)
                impl->transport.stop();
            else
                impl->transport.start();
        }
        impl->paused = p;
        auto* o = okObj().getDynamicObject();
        o->setProperty("paused", p);
        return o;
    }

    if (cmd == "playback_seek")
    {
        const double pos = req["position_sec"].isVoid() ? 0.0 : (double) req["position_sec"];
        if (!impl->playbackMode)
            return errObj("no active player");
        const double t = juce::jlimit(0.0, impl->sessionDurationSec, pos);
        const double seekInSource = impl->reverseWanted ? (impl->sessionDurationSec - t) : t;
        impl->transport.setPosition(juce::jmax(0.0, seekInSource));
        return okObj();
    }

    if (cmd == "playback_set_dsp")
    {
        const float g = req["gain"].isVoid() ? 1.0f : (float) req["gain"];
        const float pan = req["pan"].isVoid() ? 0.0f : (float) req["pan"];
        const float eqL = req["eq_low_db"].isVoid() ? 0.0f : (float) req["eq_low_db"];
        const float eqM = req["eq_mid_db"].isVoid() ? 0.0f : (float) req["eq_mid_db"];
        const float eqH = req["eq_high_db"].isVoid() ? 0.0f : (float) req["eq_high_db"];
        impl->dsp.gainBits.store(std::bit_cast<uint32_t>(g));
        impl->dsp.panBits.store(std::bit_cast<uint32_t>(pan));
        impl->dsp.eqLowBits.store(std::bit_cast<uint32_t>(eqL));
        impl->dsp.eqMidBits.store(std::bit_cast<uint32_t>(eqM));
        impl->dsp.eqHighBits.store(std::bit_cast<uint32_t>(eqH));
        return okObj();
    }

    if (cmd == "playback_set_speed")
    {
        float s = req["speed"].isVoid() ? 1.0f : (float) req["speed"];
        s = juce::jlimit(0.25f, 2.0f, s);
        // JUCE AudioTransportSource has no setSpeed; rodio-style pitch-change would need a ResamplingAudioSource wrapper.
        auto* o = okObj().getDynamicObject();
        o->setProperty("speed", s);
        o->setProperty("note", "speed stored; playback rate change not yet wired in JUCE engine");
        return o;
    }

    if (cmd == "playback_set_reverse")
    {
        const bool en = req["reverse"].isVoid() ? false : (bool) req["reverse"];
        impl->reverseWanted = en;
        auto* o = okObj().getDynamicObject();
        o->setProperty("reverse", en);
        return o;
    }

    if (cmd == "playback_status")
        return impl->playbackStatusLocked();

    if (cmd == "set_output_tone")
    {
        const bool t = req["tone"].isVoid() ? false : (bool) req["tone"];
        if (!impl->outputRunning)
            return errObj("no output stream");
        impl->toneSource.toneOn.store(t);
        auto* o = okObj().getDynamicObject();
        o->setProperty("tone", t);
        return o;
    }

    if (cmd == "stop_output_stream")
    {
        const bool was = impl->outputRunning;
        impl->stopOutputLocked();
        auto* o = okObj().getDynamicObject();
        o->setProperty("was_running", was);
        return o;
    }

    if (cmd == "stop_input_stream")
    {
        const bool was = impl->inputRunning;
        impl->stopInputLocked();
        auto* o = okObj().getDynamicObject();
        o->setProperty("was_running", was);
        return o;
    }

    if (cmd == "plugin_chain")
    {
        impl->scanPluginsOnce();
        const juce::Array<juce::PluginDescription> types = impl->pluginList.getTypes();
        juce::Array<juce::var> plugins;
        for (const auto& t : types)
        {
            auto* row = new juce::DynamicObject();
            row->setProperty("name", t.name);
            row->setProperty("format", t.pluginFormatName);
            row->setProperty("path", t.fileOrIdentifier);
            plugins.add(row);
        }
        auto* o = okObj().getDynamicObject();
        o->setProperty("phase", "juce");
        o->setProperty("api_version", 1);
        o->setProperty("slots", juce::Array<juce::var>());
        juce::Array<juce::var> fmts;
        fmts.add("VST3");
#if JUCE_MAC
        fmts.add("AU");
#endif
        o->setProperty("formats_planned", juce::var(fmts));
        o->setProperty("plugins", juce::var(plugins));
        o->setProperty("plugin_count", plugins.size());
        o->setProperty("note", "JUCE KnownPluginList scan; real-time insert chain next.");
        return o;
    }

    return errObj("unknown cmd: " + cmd);
}

} // namespace audio_haxor
