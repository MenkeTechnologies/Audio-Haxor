#include "AppLog.hpp"
#include "Engine.hpp"
#include "VisualPreview.hpp"

#include <atomic>
#include <bit>
#include <chrono>
#include <cmath>
#include <cstdlib>
#include <deque>
#include <functional>
#include <future>
#include <memory>
#include <mutex>
#include <optional>
#include <thread>
#include <unordered_map>
#include <vector>

#include <juce_events/juce_events.h>
#include <juce_gui_basics/juce_gui_basics.h>

#include <juce_audio_devices/juce_audio_devices.h>
#include <juce_audio_formats/juce_audio_formats.h>
#include <juce_audio_processors/juce_audio_processors.h>
#include <juce_audio_utils/juce_audio_utils.h>
#include <juce_dsp/juce_dsp.h>
#include <juce_core/juce_core.h>

namespace audio_haxor {
namespace {

#ifndef AUDIO_ENGINE_VERSION_STRING
#define AUDIO_ENGINE_VERSION_STRING "2.4.2"
#endif

static constexpr float kTestToneHz = 440.0f;
static constexpr float kTestToneGain = 0.05f;
static constexpr float kInputPeakDecay = 0.95f;
static constexpr uint32_t kMaxBufferFrames = 8192;

/** `setenv` is POSIX; MSVC exposes `_putenv_s` instead. */
static void setProcessEnv(const char* name, const char* value)
{
#if defined(_WIN32)
    if (_putenv_s(name, value) != 0)
        appLogLine(juce::String("setProcessEnv failed: ") + name);
#else
    if (::setenv(name, value, 1) != 0)
        appLogLine(juce::String("setProcessEnv failed: ") + name);
#endif
}

static juce::var errObj(const juce::String& msg)
{
    appLogLine("error: " + msg);
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

/** JUCE dead-man's-pedal: plugins that crash during scan are deferred to the end on later runs. */
static juce::File deadMansPedalFilePath()
{
    juce::File dir = audio_haxor::appDataDirectoryForAudioEngine();
    (void) dir.createDirectory();
    return dir.getChildFile("plugin-scan-dead-mans-pedal.txt");
}

/** Persisted `KnownPluginList` so restarts skip unchanged modules (paired with `dontRescanIfAlreadyInList`). */
static juce::File knownPluginListCacheFilePath()
{
    return audio_haxor::appDataDirectoryForAudioEngine().getChildFile("known-plugin-list.xml");
}

/** Called after each scan step so a hang or crash mid-scan still leaves a useful cache on disk. */
static void persistKnownPluginListCache(const juce::KnownPluginList& list)
{
    const juce::File cacheFile = knownPluginListCacheFilePath();
    if (auto xml = list.createXml())
    {
        (void) cacheFile.getParentDirectory().createDirectory();
        xml->writeTo(cacheFile, {});
    }
}

/** Never throws — a disk/XML error must not abort the whole scan (user would have to restart the engine). */
static void persistKnownPluginListCacheSafe(const juce::KnownPluginList& list)
{
    try
    {
        persistKnownPluginListCache(list);
    }
    catch (const std::exception& e)
    {
        appLogLine("plugin scan: known-plugin-list.xml write failed (continuing): " + juce::String(e.what()));
    }
    catch (...)
    {
        appLogLine("plugin scan: known-plugin-list.xml write failed (continuing, non-std)");
    }
}

/** Loads `known-plugin-list.xml` into `out`. Returns false if missing or invalid. */
static bool loadKnownPluginListFromCacheFile(juce::KnownPluginList& out)
{
    const juce::File cacheFile = knownPluginListCacheFilePath();
    if (!cacheFile.existsAsFile())
        return false;
    juce::XmlDocument doc(cacheFile);
    std::unique_ptr<juce::XmlElement> root = doc.getDocumentElement();
    if (root == nullptr)
        return false;
    try
    {
        out.recreateFromXml(*root);
    }
    catch (...)
    {
        return false;
    }
    return true;
}

/** How many prior scans to keep as `known-plugin-list.backup.N.xml` when `plugin_rescan` runs (N = 1 is most recent). */
static constexpr int kKnownPluginListBackupSlots = 4;

/** Before removing `known-plugin-list.xml` on `plugin_rescan`, rotate rolling backups for manual diff / parity checks. */
static void rotateKnownPluginListBackupsBeforeWipe()
{
    const juce::File cache = knownPluginListCacheFilePath();
    if (!cache.existsAsFile())
        return;

    const juce::File dir = cache.getParentDirectory();
    (void) dir.createDirectory();

    const juce::File oldest = dir.getChildFile("known-plugin-list.backup." + juce::String(kKnownPluginListBackupSlots) + ".xml");
    (void) oldest.deleteFile();

    for (int i = kKnownPluginListBackupSlots - 1; i >= 1; --i)
    {
        const juce::File from = dir.getChildFile("known-plugin-list.backup." + juce::String(i) + ".xml");
        const juce::File to = dir.getChildFile("known-plugin-list.backup." + juce::String(i + 1) + ".xml");
        if (from.existsAsFile())
        {
            (void) to.deleteFile();
            if (!from.moveFileTo(to))
                appLogLine("plugin_rescan: backup rotate failed (" + from.getFileName() + " -> " + to.getFileName() + ")");
        }
    }

    const juce::File b1 = dir.getChildFile("known-plugin-list.backup.1.xml");
    (void) b1.deleteFile();
    if (!cache.moveFileTo(b1))
        appLogLine("plugin_rescan: could not move known-plugin-list.xml to backup.1 (parity copy lost)");
    else
        appLogLine("plugin_rescan: previous known-plugin-list.xml saved as known-plugin-list.backup.1.xml (rolling backups "
                   + juce::String(kKnownPluginListBackupSlots) + " deep)");
}

static juce::StringArray readDeadMansPedalLines(const juce::File& deadMans)
{
    juce::StringArray lines;
    if (deadMans.getFullPathName().isNotEmpty())
        deadMans.readLines(lines);
    lines.removeEmptyStrings();
    return lines;
}

/** One plugin identifier per line — modules that threw during `scanNextFile` or were manually listed; never rescanned. */
static juce::File pluginScanSkipFilePath()
{
    return audio_haxor::appDataDirectoryForAudioEngine().getChildFile("plugin-scan-skip.txt");
}

/** File lines + built-in IDs (hangs that never throw) + `AUDIO_HAXOR_PLUGIN_SCAN_SKIP` (comma-separated). */
static juce::StringArray mergePluginScanSkipList(const juce::File& skipFile)
{
    juce::StringArray skips = readDeadMansPedalLines(skipFile);
    /* Apple system AUs that often block `PluginDirectoryScanner::scanNextFile` indefinitely (no throw →
     * FAIL_SKIP never runs). Identifiers match JUCE `AudioUnitFormatHelpers::createPluginIdentifier`
     * (see `juce_AudioUnitPluginFormat.mm`: AudioUnit:<Category>/type4,subtype4,manu4). */
    static const char* const kBuiltin[] = {
        "AudioUnit:Panners/aupn,vbas,appl",   // AUVectorPanner / AU Panner
        "AudioUnit:Mixers/mixr,aumx,appl",    // AUMixer
        "AudioUnit:Mixers/mixr,mxmx,appl",    // AUMatrixMixer
        "AudioUnit:Mixers/mixr,mcmx,appl",    // AUMultiChannelMixer
        "AudioUnit:Mixers/mixr,spmx,appl",    // AUSpatialMixer — often blocks JUCE scanNextFile (no throw)
    };
    for (const char* s : kBuiltin)
    {
        const juce::String id(s);
        if (!skips.contains(id))
            skips.add(id);
    }
    const juce::String env = juce::SystemStats::getEnvironmentVariable("AUDIO_HAXOR_PLUGIN_SCAN_SKIP", {}).trim();
    if (env.isNotEmpty())
    {
        juce::StringArray parts;
        parts.addTokens(env, ",", {});
        for (const juce::String& t : parts)
        {
            const juce::String u = t.trim();
            if (u.isNotEmpty() && !skips.contains(u))
                skips.add(u);
        }
    }
    return skips;
}

static juce::StringArray filterOutSkippedPluginIdentifiers(const juce::StringArray& files, const juce::File& skipFile)
{
    const juce::StringArray skips = mergePluginScanSkipList(skipFile);
    juce::StringArray out;
    for (const juce::String& f : files)
    {
        /* Apple AUSpatialMixer (subtype 'spmx'); category path can differ by OS/JUCE — match subtype+manu. */
        if (f.endsWithIgnoreCase(",spmx,appl"))
        {
            appLogLine("plugin scan: SKIP_LIST apple_spatial_mixer file=\"" + f + "\"");
            continue;
        }
        if (skips.contains(f))
        {
            appLogLine("plugin scan: SKIP_LIST file=\"" + f + "\"");
            continue;
        }
        out.add(f);
    }
    return out;
}

static void appendPluginScanSkipIfNew(const juce::String& fileId)
{
    if (fileId.isEmpty())
        return;
    const juce::File path = pluginScanSkipFilePath();
    (void) path.getParentDirectory().createDirectory();
    juce::StringArray lines = readDeadMansPedalLines(path);
    if (lines.contains(fileId))
        return;
    lines.add(fileId);
    (void) path.replaceWithText(lines.joinIntoString("\n") + "\n", false, false);
}

static void appendPluginScanSkipAndBlacklistSafe(juce::KnownPluginList& list, const juce::String& fileId)
{
    if (fileId.isEmpty())
        return;
    try
    {
        appendPluginScanSkipIfNew(fileId);
    }
    catch (const std::exception& e)
    {
        appLogLine("plugin scan: plugin-scan-skip.txt append failed: " + juce::String(e.what()));
    }
    catch (...)
    {
        appLogLine("plugin scan: plugin-scan-skip.txt append failed (non-std)");
    }
    try
    {
        list.addToBlacklist(fileId);
    }
    catch (const std::exception& e)
    {
        appLogLine("plugin scan: addToBlacklist failed: " + juce::String(e.what()));
    }
    catch (...)
    {
        appLogLine("plugin scan: addToBlacklist failed (non-std)");
    }
}

/** Same file order as `juce::PluginDirectoryScanner` (searchPaths + dead-man's-pedal reorder). */
static juce::StringArray buildOrderedPluginScanFiles(juce::AudioPluginFormat& format,
                                                     const juce::FileSearchPath& dirs,
                                                     bool recursive,
                                                     const juce::File& deadMans)
{
    juce::FileSearchPath p = dirs;
    p.removeRedundantPaths();
    juce::StringArray files = format.searchPathsForPlugins(p, recursive, false);
    for (const auto& crashed : readDeadMansPedalLines(deadMans))
        for (int j = files.size(); --j >= 0;)
            if (crashed == files[j])
                files.move(j, -1);
    return files;
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

/** Smallest supported buffer size >= minFrames; if none, largest available. */
static int pickNearestSupportedBufferAtLeast(juce::AudioIODevice* dev, int minFrames)
{
    if (dev == nullptr || minFrames <= 0)
        return 0;
    const juce::Array<int> sizes = dev->getAvailableBufferSizes();
    if (sizes.isEmpty())
        return juce::jmax(minFrames, dev->getCurrentBufferSizeSamples());
    int best = -1;
    for (int s : sizes)
    {
        if (s >= minFrames && (best < 0 || s < best))
            best = s;
    }
    if (best >= 0)
        return best;
    int mx = sizes.getFirst();
    for (int s : sizes)
        mx = juce::jmax(mx, s);
    return mx;
}

/** When the user omits `buffer_frames`, some drivers default to very small buffers (high xrun risk). */
static constexpr int kStablePlaybackMinFrames = 512;

static void maybeBumpBufferForStablePlayback(juce::AudioDeviceManager& mgr, juce::AudioIODevice*& dev, uint32_t userBf)
{
    if (userBf > 0 || dev == nullptr)
        return;
    const int cur = dev->getCurrentBufferSizeSamples();
    const int want = pickNearestSupportedBufferAtLeast(dev, kStablePlaybackMinFrames);
    if (want <= cur || want <= 0)
        return;
    juce::AudioDeviceManager::AudioDeviceSetup s = mgr.getAudioDeviceSetup();
    s.bufferSize = want;
    mgr.setAudioDeviceSetup(s, true);
    dev = mgr.getCurrentAudioDevice();
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

static void copyDeviceType(const juce::AudioDeviceManager& src, juce::AudioDeviceManager& dst)
{
    const juce::String t = src.getCurrentAudioDeviceType();
    if (t.isNotEmpty())
        dst.setCurrentAudioDeviceType(t, false);
}

/** Platform device-type objects not wired into `dm`'s internal list — avoids `getAvailableDeviceTypes()` → `scanDevicesIfNeeded()` (fragile with two managers in one process). */
static void createFreshDeviceTypes(juce::AudioDeviceManager& dm, juce::OwnedArray<juce::AudioIODeviceType>& out)
{
    out.clear();
    dm.createAudioDeviceTypes(out);
}

static void enumerateOutputIds(juce::AudioDeviceManager& dm, juce::StringArray& outIds, juce::StringArray& outNames)
{
    outIds.clear();
    outNames.clear();
    std::unordered_map<juce::String, uint32_t> seen;
    if (auto* t = dm.getCurrentDeviceTypeObject())
    {
        const juce::StringArray names = t->getDeviceNames(false);
        for (int i = 0; i < names.size(); ++i)
        {
            const juce::String& n = names[i];
            outNames.add(n);
            outIds.add(uniqueDeviceId(n, seen));
        }
    }
    if (outNames.isEmpty())
    {
        juce::OwnedArray<juce::AudioIODeviceType> types;
        createFreshDeviceTypes(dm, types);
        for (auto* t : types)
        {
            if (t == nullptr)
                continue;
            t->scanForDevices();
            const juce::StringArray names = t->getDeviceNames(false);
            for (int i = 0; i < names.size(); ++i)
            {
                const juce::String& n = names[i];
                if (outNames.contains(n))
                    continue;
                outNames.add(n);
                outIds.add(uniqueDeviceId(n, seen));
            }
        }
    }
}

static void enumerateInputIds(juce::AudioDeviceManager& dm, juce::StringArray& outIds, juce::StringArray& outNames)
{
    outIds.clear();
    outNames.clear();
    std::unordered_map<juce::String, uint32_t> seen;
    if (auto* t = dm.getCurrentDeviceTypeObject())
    {
        const juce::StringArray names = t->getDeviceNames(true);
        for (int i = 0; i < names.size(); ++i)
        {
            const juce::String& n = names[i];
            outNames.add(n);
            outIds.add(uniqueDeviceId(n, seen));
        }
    }
    if (outNames.isEmpty())
    {
        juce::OwnedArray<juce::AudioIODeviceType> types;
        createFreshDeviceTypes(dm, types);
        for (auto* t : types)
        {
            if (t == nullptr)
                continue;
            t->scanForDevices();
            const juce::StringArray names = t->getDeviceNames(true);
            for (int i = 0; i < names.size(); ++i)
            {
                const juce::String& n = names[i];
                if (outNames.contains(n))
                    continue;
                outNames.add(n);
                outIds.add(uniqueDeviceId(n, seen));
            }
        }
    }
}

static juce::String resolveOutputDeviceName(juce::AudioDeviceManager& dm, const juce::String& id)
{
    juce::StringArray ids, names;
    enumerateOutputIds(dm, ids, names);
    if (id.isEmpty())
    {
        juce::AudioIODevice* dev = dm.getCurrentAudioDevice();
        if (dev != nullptr)
            return dev->getName();
        if (names.size() > 0)
            return names[0];
        return {};
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

static juce::String resolveInputDeviceName(juce::AudioDeviceManager& dm, const juce::String& id)
{
    juce::StringArray ids, names;
    enumerateInputIds(dm, ids, names);
    if (id.isEmpty())
    {
        juce::AudioIODevice* dev = dm.getCurrentAudioDevice();
        if (dev != nullptr)
            return dev->getName();
        if (names.size() > 0)
            return names[0];
        return {};
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

static juce::String outputIdForDeviceName(juce::AudioDeviceManager& dm, const juce::String& deviceName)
{
    juce::StringArray ids, names;
    enumerateOutputIds(dm, ids, names);
    for (int i = 0; i < names.size(); ++i)
        if (names[i] == deviceName)
            return ids[i];
    return deviceName;
}

static juce::String inputIdForDeviceName(juce::AudioDeviceManager& dm, const juce::String& deviceName)
{
    juce::StringArray ids, names;
    enumerateInputIds(dm, ids, names);
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

class InsertChainRunner;

struct InsertChainPreparePayload
{
    InsertChainRunner* self = nullptr;
    double sr = 44100.0;
    int maxBlock = 512;
};

static void* insertChainPrepareOnMessageThreadFn(void* userData);

/** Stereo insert chain (VST3 / AU) after file decode + built-in DSP. Not applied in reverse-playback mode (sample-at-a-time path). */
class InsertChainRunner
{
    friend void* insertChainPrepareOnMessageThreadFn(void*);

public:
    std::vector<std::unique_ptr<juce::AudioPluginInstance>> instances;
    std::vector<juce::String> paths;

    bool isActive() const { return !instances.empty(); }

    void release()
    {
        for (auto& p : instances)
            if (p != nullptr)
                p->releaseResources();
    }

    void clear()
    {
        release();
        instances.clear();
        paths.clear();
    }

    /** VST3 `prepareToPlay` must run on the JUCE message thread (`JUCE_ASSERT_MESSAGE_THREAD` in JUCE).
        `AudioSource::prepareToPlay` is invoked from the device graph (often not the message thread), so we
        marshal here. Also applies `setPlayConfigDetails(2,2,…)` + `disableNonMainBuses()` so sidechain/aux
        buses do not leave garbage in `processBlock`. */
    void prepare(double sr, int maxBlock)
    {
        InsertChainPreparePayload payload { this, sr, maxBlock };
        juce::MessageManager::getInstance()->callFunctionOnMessageThread(insertChainPrepareOnMessageThreadFn,
                                                                         &payload);
    }

    void process(juce::AudioBuffer<float>& buf, int start, int n)
    {
        if (instances.empty() || n <= 0)
            return;
        const int nc = scratchChannels;
        /* `processBlock` uses `buffer.getNumSamples()` — the buffer must be exactly `n`, not a larger
           scratch left over from a previous (longer) block; otherwise plugins process stale samples
           (garbled audio). */
        scratch.setSize(nc, n, false, false, true);
        scratch.clear();
        scratch.copyFrom(0, 0, buf, 0, start, n);
        scratch.copyFrom(1, 0, buf, 1, start, n);
        juce::MidiBuffer midi;
        for (auto& p : instances)
        {
            if (p == nullptr || p->isSuspended())
                continue;
            const juce::ScopedLock sl(p->getCallbackLock());
            p->processBlock(scratch, midi);
        }
        buf.copyFrom(0, start, scratch, 0, 0, n);
        buf.copyFrom(1, start, scratch, 1, 0, n);
    }

private:
    juce::AudioBuffer<float> scratch;
    int scratchChannels = 2;
};

static void* insertChainPrepareOnMessageThreadFn(void* userData)
{
    auto* p = static_cast<InsertChainPreparePayload*>(userData);
    InsertChainRunner* self = p->self;
    if (self == nullptr)
        return nullptr;
    const double sr = p->sr;
    const int block = juce::jmax(1, p->maxBlock);
    int maxCh = 2;
    for (auto& inst : self->instances)
    {
        if (inst == nullptr)
            continue;
        inst->releaseResources();
        inst->setPlayConfigDetails(2, 2, sr, block);
        inst->setProcessingPrecision(juce::AudioProcessor::singlePrecision);
        inst->prepareToPlay(sr, block);
        maxCh = juce::jmax(maxCh, inst->getTotalNumInputChannels(), inst->getTotalNumOutputChannels());
    }
    self->scratchChannels = juce::jmax(2, maxCh);
    self->scratch.setSize(self->scratchChannels, block, false, false, true);
    return nullptr;
}

/** Native VST3/AU editor window; must be created/destroyed on the JUCE message thread. */
class PluginEditorHostWindow : public juce::DocumentWindow
{
public:
    PluginEditorHostWindow(int chainSlot, std::function<void(int)> onClose, juce::AudioPluginInstance& inst)
        : DocumentWindow(inst.getName() + " (insert)", juce::Colours::lightgrey, DocumentWindow::allButtons),
          slot(chainSlot),
          closeFn(std::move(onClose))
    {
        /* Native title bar + wrong outer size often yields blank VST3 (IPlugView::attached needs a peer)
         * and mis-sized AU Cocoa views. Use JUCE title bar and size the *content* via setContentComponentSize. */
        setUsingNativeTitleBar(false);
        juce::AudioProcessorEditor* ed = inst.createEditorIfNeeded();
        if (ed != nullptr)
        {
            const int w = juce::jmax(200, ed->getWidth());
            const int h = juce::jmax(150, ed->getHeight());
            ed->setSize(w, h);
            setContentOwned(ed, true);
            setContentComponentSize(w, h);
        }
        setResizable(true, true);
        setAlwaysOnTop(true);
    }

    /** VST3 defers view attach until visibility + valid peer; AU sometimes needs a second layout tick. */
    void schedulePostShowLayout()
    {
        juce::Component::SafePointer<PluginEditorHostWindow> safe(this);
        auto bump = [safe]() {
            if (safe == nullptr)
                return;
            safe->resized();
            if (auto* c = safe->getContentComponent())
            {
                c->resized();
                c->repaint();
            }
        };
        juce::MessageManager::callAsync(bump);
        juce::Timer::callAfterDelay(50, bump);
    }

    bool hasEditorContent() const { return getContentComponent() != nullptr; }

    void closeButtonPressed() override
    {
        if (!closeFn)
            return;
        const int s = slot;
        std::function<void(int)> fn = closeFn;
        juce::MessageManager::callAsync([fn, s]() {
            if (fn)
                fn(s);
        });
    }

private:
    int slot = -1;
    std::function<void(int)> closeFn;
};

class ToneAudioSource final : public juce::AudioSource
{
public:
    std::atomic<bool> toneOn{false};
    std::atomic<uint64_t> phase{0};
    /** Optional: tap mono spectrum (test tone path; inserts apply to file playback only). */
    std::function<void(const float*, int)> spectrumPushBatch;

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
        thread_local std::vector<float> toneSpectrumMono;
        if (toneSpectrumMono.size() < (size_t)n)
            toneSpectrumMono.resize((size_t)n);
        for (int i = 0; i < n; ++i)
        {
            const float s = (float) (std::sin((double) p * twoPi * (double) kTestToneHz / sr) * (double) kTestToneGain);
            for (int c = 0; c < ch; ++c)
                bufferToFill.buffer->setSample(c, bufferToFill.startSample + i, s);
            toneSpectrumMono[(size_t)i] = s;
            ++p;
        }
        phase.store(p);
        if (spectrumPushBatch)
            spectrumPushBatch(toneSpectrumMono.data(), n);
    }

private:
    double sr = 44100.0;
};

class DspStereoFileSource final : public juce::PositionableAudioSource
{
public:
    std::unique_ptr<juce::AudioFormatReaderSource> readerSource;
    /** Forward playback only: wraps `readerSource` for tape-style speed (pitch follows rate). */
    std::unique_ptr<juce::ResamplingAudioSource> speedResampler;
    std::atomic<float>* playbackSpeed = nullptr;
    /** When non-null, reverse path wraps when `load()` is true (same flag as forward `playback_set_loop`). */
    std::atomic<bool>* playbackLoop = nullptr;
    juce::AudioBuffer<float> reverseStereo;
    bool reverseMode = false;
    int reverseFrame = 0;
    DspAtomics* dsp = nullptr;
    std::atomic<float>* peak = nullptr;
    /** Filled after DSP + inserts (what reaches the device). */
    std::function<void(const float*, int)> spectrumPushBatch;
    juce::dsp::IIR::Filter<float> lowL, lowR, midL, midR, hiL, hiR;
    double processRate = 44100.0;
    InsertChainRunner* insertChain = nullptr;

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
        if (speedResampler != nullptr)
            speedResampler->prepareToPlay(samplesPerBlockExpected, sampleRate);
        if (insertChain != nullptr)
            insertChain->prepare(sampleRate, samplesPerBlockExpected);
    }

    void releaseResources() override
    {
        if (speedResampler != nullptr)
            speedResampler->releaseResources();
        if (readerSource != nullptr)
            readerSource->releaseResources();
        if (insertChain != nullptr)
            insertChain->release();
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
            if (speedResampler != nullptr)
                speedResampler->flushBuffers();
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
        if (reverseMode && playbackLoop != nullptr)
            return playbackLoop->load();
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
            thread_local std::vector<float> revSpectrumMono;
            if (revSpectrumMono.size() < (size_t)n)
                revSpectrumMono.resize((size_t)n);
            const int frames = reverseStereo.getNumSamples();
            int specCount = 0;
            for (int i = 0; i < n; ++i)
            {
                if (reverseFrame >= frames)
                {
                    if (playbackLoop != nullptr && playbackLoop->load())
                        reverseFrame = 0;
                    else
                    {
                        bufferToFill.buffer->clear(bufferToFill.startSample + i, n - i);
                        break;
                    }
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
                revSpectrumMono[(size_t)specCount++] = (l + r) * 0.5f;
            }
            if (spectrumPushBatch && specCount > 0)
                spectrumPushBatch(revSpectrumMono.data(), specCount);
            /* Reverse path is sample-wise; VST block processing skipped. */
            return;
        }

        if (readerSource == nullptr)
        {
            bufferToFill.clearActiveBufferRegion();
            return;
        }

        if (speedResampler != nullptr)
        {
            if (playbackSpeed != nullptr)
            {
                const double r = (double) playbackSpeed->load();
                speedResampler->setResamplingRatio(juce::jlimit(0.25, 2.0, r));
            }
            speedResampler->getNextAudioBlock(bufferToFill);
        }
        else
        {
            readerSource->getNextAudioBlock(bufferToFill);
        }

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

        if (insertChain != nullptr && insertChain->isActive())
            insertChain->process(*bufferToFill.buffer, bufferToFill.startSample, n);

        if (spectrumPushBatch)
        {
            thread_local std::vector<float> fwdSpectrumMono;
            if (fwdSpectrumMono.size() < (size_t)n)
                fwdSpectrumMono.resize((size_t)n);
            for (int i = 0; i < n; ++i)
            {
                const float l = bufferToFill.buffer->getSample(0, bufferToFill.startSample + i);
                const float r = bufferToFill.buffer->getSample(1, bufferToFill.startSample + i);
                fwdSpectrumMono[(size_t)i] = (l + r) * 0.5f;
            }
            spectrumPushBatch(fwdSpectrumMono.data(), n);
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

enum class PluginScanOopResult
{
    Ok,
    Timeout,
    ChildFailed,
    StartFailed
};

static int pluginScanTimeoutMsFromEnv()
{
    const char* e = std::getenv("AUDIO_HAXOR_PLUGIN_SCAN_TIMEOUT_SEC");
    if (e == nullptr || e[0] == '\0')
        return 30 * 1000;
    char* end = nullptr;
    const long sec = std::strtol(e, &end, 10);
    if (end == e || sec <= 0)
        return 30 * 1000;
    return (int) juce::jlimit(5L, 3600L, sec) * 1000;
}

static PluginScanOopResult spawnPluginScanChildMerge(juce::KnownPluginList& list, const juce::String& formatLabel,
                                                     const juce::String& fileId, int timeoutMs)
{
    persistKnownPluginListCacheSafe(list);
    const juce::File outFile =
        juce::File::getSpecialLocation(juce::File::tempDirectory)
            .getChildFile("ahx_ps_" + juce::Uuid().toString() + ".xml");

    const juce::File exe = juce::File::getSpecialLocation(juce::File::currentExecutableFile);
    juce::StringArray args;
    args.add(exe.getFullPathName());
    args.add("--plugin-scan-one");
    args.add(formatLabel);
    args.add(juce::Base64::toBase64(fileId.toRawUTF8(), fileId.getNumBytesAsUTF8()));
    args.add(juce::Base64::toBase64(outFile.getFullPathName().toRawUTF8(), outFile.getFullPathName().getNumBytesAsUTF8()));

    juce::ChildProcess child;
    if (!child.start(args))
    {
        appLogLine("plugin scan: OOP start_failed file=\"" + fileId + "\"");
        return PluginScanOopResult::StartFailed;
    }

    if (!child.waitForProcessToFinish(timeoutMs))
    {
        child.kill();
        appLogLine("plugin scan: TIMEOUT_KILL timeout_ms=" + juce::String(timeoutMs) + " file=\"" + fileId + "\"");
        (void) outFile.deleteFile();
        return PluginScanOopResult::Timeout;
    }

    const int exitCode = (int) child.getExitCode();
    if (exitCode != 0)
    {
        appLogLine("plugin scan: OOP child_exit=" + juce::String(exitCode) + " file=\"" + fileId + "\"");
        (void) outFile.deleteFile();
        return PluginScanOopResult::ChildFailed;
    }

    if (!outFile.existsAsFile())
    {
        appLogLine("plugin scan: OOP missing_output file=\"" + fileId + "\"");
        return PluginScanOopResult::ChildFailed;
    }

    juce::XmlDocument doc(outFile);
    std::unique_ptr<juce::XmlElement> root = doc.getDocumentElement();
    (void) outFile.deleteFile();
    if (root == nullptr)
    {
        appLogLine("plugin scan: OOP bad_xml file=\"" + fileId + "\"");
        return PluginScanOopResult::ChildFailed;
    }

    try
    {
        list.recreateFromXml(*root);
    }
    catch (const std::exception& e)
    {
        appLogLine("plugin scan: OOP merge_xml failed: " + juce::String(e.what()));
        return PluginScanOopResult::ChildFailed;
    }
    catch (...)
    {
        appLogLine("plugin scan: OOP merge_xml failed (non-std)");
        return PluginScanOopResult::ChildFailed;
    }

    return PluginScanOopResult::Ok;
}

struct Engine::Impl
{
    /** Serializes `waveform_preview` / `spectrogram_preview` decode (not the main `mutex`). */
    std::mutex previewMutex;
    std::mutex mutex;
    juce::AudioDeviceManager outputManager;
    juce::AudioDeviceManager inputManager;
    juce::AudioSourcePlayer sourcePlayer;
    juce::AudioTransportSource transport;
    juce::AudioFormatManager formatManager;
    ToneAudioSource toneSource;
    std::unique_ptr<DspStereoFileSource> fileSource;
    std::atomic<float> playbackPeak{0.0f};
    /** 0.25–2.0, tape-style playback (`juce::ResamplingAudioSource`); ignored in reverse mode. */
    std::atomic<float> playbackSpeed{1.0f};
    DspAtomics dsp;

    std::mutex spectrumRingMutex;
    std::deque<float> spectrumRing;
    static constexpr size_t kSpectrumRingMax = 32768;
    std::unique_ptr<juce::dsp::FFT> spectrumFft;

    /** One lock per callback block — avoids per-sample mutex traffic on the audio thread. */
    void pushSpectrumMonoBatch(const float* mono, int count)
    {
        if (count <= 0 || mono == nullptr)
            return;
        std::lock_guard<std::mutex> lk(spectrumRingMutex);
        for (int i = 0; i < count; ++i)
        {
            spectrumRing.push_back(mono[i]);
            while (spectrumRing.size() > kSpectrumRingMax)
                spectrumRing.pop_front();
        }
    }

    void clearSpectrumRing()
    {
        std::lock_guard<std::mutex> lk(spectrumRingMutex);
        spectrumRing.clear();
    }

    void wireSpectrumCallbacks()
    {
        auto fn = [this](const float* m, int c) { pushSpectrumMonoBatch(m, c); };
        toneSource.spectrumPushBatch = fn;
        if (fileSource != nullptr)
            fileSource->spectrumPushBatch = fn;
    }

    void clearSpectrumCallbacks()
    {
        toneSource.spectrumPushBatch = {};
        if (fileSource != nullptr)
            fileSource->spectrumPushBatch = {};
    }

    /** Hann + real FFT → 1024 magnitudes (0–255) for WebView (np FFT strip, EQ canvas, visualizer). */
    void appendPlaybackSpectrumJson(juce::DynamicObject* o)
    {
        if (o == nullptr)
            return;
        constexpr int fftOrder = 11;
        constexpr int fftSize = 1 << fftOrder;
        constexpr int fftBinsOut = 1024;
        const int srOut = outSampleRate > 0 ? outSampleRate : (int) deviceRate.load();
        if (!outputRunning)
        {
            o->setProperty("spectrum", juce::var());
            o->setProperty("spectrum_fft_size", fftSize);
            o->setProperty("spectrum_bins", fftBinsOut);
            o->setProperty("spectrum_sr_hz", srOut > 0 ? srOut : 44100);
            return;
        }
        std::vector<float> snap;
        {
            std::lock_guard<std::mutex> lk(spectrumRingMutex);
            if (spectrumRing.size() < (size_t) fftSize)
            {
                o->setProperty("spectrum", juce::var());
                o->setProperty("spectrum_fft_size", fftSize);
                o->setProperty("spectrum_bins", fftBinsOut);
                o->setProperty("spectrum_sr_hz", srOut > 0 ? srOut : 44100);
                return;
            }
            snap.assign(spectrumRing.end() - (ptrdiff_t) fftSize, spectrumRing.end());
        }
        std::vector<float> window((size_t) fftSize);
        for (int i = 0; i < fftSize; ++i)
            window[(size_t) i] =
                0.5f * (1.0f - std::cos((float) (2.0 * juce::MathConstants<double>::pi * (double) i / (double) juce::jmax(1, fftSize - 1))));
        std::vector<float> fftBuf((size_t) (fftSize * 2), 0.f);
        for (int i = 0; i < fftSize; ++i)
            fftBuf[(size_t) i] = snap[(size_t) i] * window[(size_t) i];
        if (!spectrumFft)
            spectrumFft = std::make_unique<juce::dsp::FFT>(fftOrder);
        spectrumFft->performFrequencyOnlyForwardTransform(fftBuf.data(), true);
        float mx = 1.0e-9f;
        for (int i = 1; i <= fftBinsOut; ++i)
            mx = juce::jmax(mx, fftBuf[(size_t) i]);
        juce::Array<juce::var> arr;
        arr.ensureStorageAllocated(fftBinsOut);
        for (int i = 0; i < fftBinsOut; ++i)
        {
            const int bi = 1 + i;
            const float mag = fftBuf[(size_t) bi] / mx;
            arr.add((int) juce::jlimit(0, 255, (int) std::lround(mag * 255.0f)));
        }
        o->setProperty("spectrum", juce::var(arr));
        o->setProperty("spectrum_fft_size", fftSize);
        o->setProperty("spectrum_bins", fftBinsOut);
        o->setProperty("spectrum_sr_hz", srOut > 0 ? srOut : 44100);
    }

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
    /** Forward: `AudioFormatReaderSource::setLooping`. Reverse: `DspStereoFileSource` wraps RAM buffer. */
    std::atomic<bool> playbackLoopWanted{false};
    bool paused = false;

    juce::VST3PluginFormat vst3;
#if JUCE_MAC
    juce::AudioUnitPluginFormat auFormat;
#endif
    juce::AudioPluginFormatManager pluginFormatManager;
    std::unique_ptr<InsertChainRunner> insertRunner;
    std::vector<std::unique_ptr<PluginEditorHostWindow>> insertEditorWindows;

    enum class PluginScanPhase
    {
        Idle,
        Running,
        Done,
        Failed
    };

    std::mutex pluginScanMutex;
    std::thread pluginScanThread;
    juce::Array<juce::PluginDescription> pluginScanCache;
    PluginScanPhase pluginScanPhase = PluginScanPhase::Idle;
    juce::String pluginScanLastError;
    std::atomic<bool> pluginScanCancel{false};

    struct PluginScanProgressState
    {
        std::atomic<int> done{0};
        std::atomic<int> total{0};
        std::atomic<int> skipped{0};
        std::atomic<bool> cacheLoaded{false};
        std::mutex mutex;
        juce::String currentFormat;
        juce::String currentName;

        void resetForNewScan()
        {
            done.store(0, std::memory_order_relaxed);
            skipped.store(0, std::memory_order_relaxed);
            total.store(0, std::memory_order_relaxed);
            std::lock_guard<std::mutex> lock(mutex);
            currentFormat.clear();
            currentName.clear();
        }
    };
    PluginScanProgressState pluginScanProgress;

    /** Deferred so `ping` / stdin smoke tests do not block on CoreAudio before any line is read. */
    bool audioDeviceManagersInitialised = false;

    void ensureAudioDeviceManagersInitialised()
    {
        if (audioDeviceManagersInitialised)
            return;
        appLogLine("AudioDeviceManager: initialising output + input (first non-ping cmd)");
        audioDeviceManagersInitialised = true;
        outputManager.initialise(0, 2, nullptr, true);
        inputManager.initialise(2, 0, nullptr, true);
        appLogLine("AudioDeviceManager: initialised");
    }

    Impl()
    {
        formatManager.registerBasicFormats();
        pluginFormatManager.addDefaultFormats();
        hydratePluginScanCacheFromDiskIfAvailable();
    }

    ~Impl()
    {
        if (pluginScanThread.joinable())
            pluginScanThread.join();
    }

    void scanPluginFormatWithProgress(juce::KnownPluginList& list, juce::AudioPluginFormat& format,
                                      const juce::FileSearchPath& dirs, const juce::File& deadMans,
                                      const juce::String& formatLabel)
    {
        /* Isolate failures so one bad identifier / disk error does not set plugin_scan phase to failed. */
        try
        {
        juce::StringArray files = buildOrderedPluginScanFiles(format, dirs, true, deadMans);
        files = filterOutSkippedPluginIdentifiers(files, pluginScanSkipFilePath());
        const int n = files.size();
        int processed = 0;
        const int timeoutMs = pluginScanTimeoutMsFromEnv();
        /* Each module is scanned in a separate process so a hang or crash cannot block the main engine. */
        auto yieldIfPlayback = [this]() {
            if (outputRunning && playbackMode)
                std::this_thread::sleep_for(std::chrono::milliseconds(2));
        };
        while (processed < n)
        {
            if (pluginScanCancel.load(std::memory_order_relaxed))
            {
                appLogLine("plugin scan: " + formatLabel + " cancelled at " + juce::String(processed) + "/" + juce::String(n));
                return;
            }
            const int fileIdx = n - 1 - processed;
            const juce::String fileId = files[fileIdx];
            juce::String displayName = fileId;
            try
            {
                displayName = format.getNameOfPluginFromIdentifier(fileId);
            }
            catch (...)
            {
                displayName = fileId;
            }
            bool cacheListingUpToDate = false;
            try
            {
                cacheListingUpToDate = list.isListingUpToDate(fileId, format);
            }
            catch (...)
            {
                cacheListingUpToDate = false;
            }
            if (cacheListingUpToDate)
                pluginScanProgress.skipped.fetch_add(1, std::memory_order_relaxed);
            {
                std::lock_guard<std::mutex> lk(pluginScanProgress.mutex);
                pluginScanProgress.currentFormat = formatLabel;
                pluginScanProgress.currentName = displayName;
            }
            const int doneBefore = pluginScanProgress.done.load(std::memory_order_relaxed);
            const int scanSeq = doneBefore + 1;
            const int totalCandidates = pluginScanProgress.total.load(std::memory_order_relaxed);
            try
            {
                appLogLine("plugin scan: START " + formatLabel + " scan_seq=" + juce::String(scanSeq)
                           + " total_candidates=" + juce::String(totalCandidates) + " format_pos=" + juce::String(processed + 1)
                           + "/" + juce::String(n) + " cache_listing_up_to_date=" + juce::String(cacheListingUpToDate ? "yes" : "no")
                           + " name=\"" + displayName + "\" file=\"" + fileId + "\"");
            }
            catch (...)
            {
            }

            const PluginScanOopResult oop = spawnPluginScanChildMerge(list, formatLabel, fileId, timeoutMs);
            if (oop == PluginScanOopResult::Ok)
            {
                processed++;
                pluginScanProgress.done.fetch_add(1, std::memory_order_relaxed);
                persistKnownPluginListCacheSafe(list);
                yieldIfPlayback();
                continue;
            }

            appendPluginScanSkipAndBlacklistSafe(list, fileId);
            if (oop == PluginScanOopResult::Timeout)
                appLogLine("plugin scan: TIMEOUT_SKIP " + formatLabel + " scan_seq=" + juce::String(scanSeq) + " file=\"" + fileId
                           + "\"");
            else if (oop == PluginScanOopResult::StartFailed)
                appLogLine("plugin scan: OOP_START_FAIL_SKIP " + formatLabel + " scan_seq=" + juce::String(scanSeq) + " file=\"" + fileId
                           + "\"");
            else
                appLogLine("plugin scan: OOP_FAIL_SKIP " + formatLabel + " scan_seq=" + juce::String(scanSeq) + " file=\"" + fileId
                           + "\"");
            processed++;
            pluginScanProgress.done.fetch_add(1, std::memory_order_relaxed);
            persistKnownPluginListCacheSafe(list);
            yieldIfPlayback();
        }
        }
        catch (const std::exception& e)
        {
            appLogLine("plugin scan: " + formatLabel + " phase error (skipping rest of phase): " + juce::String(e.what()));
        }
        catch (...)
        {
            appLogLine("plugin scan: " + formatLabel + " phase error (skipping rest of phase, non-std)");
        }
        {
            std::lock_guard<std::mutex> lk(pluginScanProgress.mutex);
            pluginScanProgress.currentName.clear();
        }
    }

    void runPluginScanWorker()
    {
        juce::KnownPluginList list;
        if (loadKnownPluginListFromCacheFile(list))
        {
            pluginScanProgress.cacheLoaded.store(true, std::memory_order_relaxed);
        }
        else
        {
            const juce::File cacheFile = knownPluginListCacheFilePath();
            if (cacheFile.existsAsFile())
                appLogLine("plugin scan: known-plugin-list.xml missing or invalid; scanning from empty list");
            pluginScanProgress.cacheLoaded.store(false, std::memory_order_relaxed);
        }

        const juce::File deadMans = deadMansPedalFilePath();

        int vst3Total = 0;
        int auTotal = 0;
        {
            juce::FileSearchPath p = vst3.getDefaultLocationsToSearch();
            p.removeRedundantPaths();
            vst3Total = vst3.searchPathsForPlugins(p, true, false).size();
        }
#if JUCE_MAC
        {
            juce::FileSearchPath p = auFormat.getDefaultLocationsToSearch();
            p.removeRedundantPaths();
            auTotal = auFormat.searchPathsForPlugins(p, true, false).size();
        }
#endif
        pluginScanProgress.total.store(vst3Total + auTotal, std::memory_order_relaxed);
        appLogLine("plugin scan: started total_candidates=" + juce::String(vst3Total + auTotal) + " vst3=" + juce::String(vst3Total)
#if JUCE_MAC
                   + " au=" + juce::String(auTotal)
#endif
        );

        try
        {
            {
                const juce::FileSearchPath dirs = vst3.getDefaultLocationsToSearch();
                scanPluginFormatWithProgress(list, vst3, dirs, deadMans, "VST3");
            }
        }
        catch (const std::exception& e)
        {
            persistKnownPluginListCacheSafe(list);
            appLogLine("plugin scan: VST3 phase unexpected error (continuing): " + juce::String(e.what()));
        }
        catch (...)
        {
            persistKnownPluginListCacheSafe(list);
            appLogLine("plugin scan: VST3 phase unexpected error (continuing, non-std)");
        }
#if JUCE_MAC
        if (pluginScanCancel.load(std::memory_order_relaxed))
        {
            appLogLine("plugin scan: finished cancelled (before AU phase)");
            std::lock_guard<std::mutex> lock(pluginScanMutex);
            pluginScanCache = list.getTypes();
            pluginScanPhase = PluginScanPhase::Idle;
            return;
        }
        try
        {
            appLogLine("plugin scan: VST3 phase complete; starting AU");
            {
                const juce::FileSearchPath auDirs = auFormat.getDefaultLocationsToSearch();
                scanPluginFormatWithProgress(list, auFormat, auDirs, deadMans, "AU");
            }
        }
        catch (const std::exception& e)
        {
            persistKnownPluginListCacheSafe(list);
            appLogLine("plugin scan: AU phase unexpected error (continuing): " + juce::String(e.what()));
        }
        catch (...)
        {
            persistKnownPluginListCacheSafe(list);
            appLogLine("plugin scan: AU phase unexpected error (continuing, non-std)");
        }
#endif

        const juce::Array<juce::PluginDescription> types = list.getTypes();
        const bool scanCancelled = pluginScanCancel.load(std::memory_order_relaxed);
        if (scanCancelled)
            appLogLine("plugin scan: finished cancelled plugin_count=" + juce::String(types.size()));
        else
            appLogLine("plugin scan: finished ok plugin_count=" + juce::String(types.size()));
        std::lock_guard<std::mutex> lock(pluginScanMutex);
        pluginScanCache = types;
        pluginScanPhase = PluginScanPhase::Done;
    }

    /** Load `known-plugin-list.xml` into `pluginScanCache` when non-empty; used to skip a full rescan. */
    bool hydratePluginScanCacheFromDiskIfAvailable()
    {
        juce::KnownPluginList list;
        if (!loadKnownPluginListFromCacheFile(list))
            return false;
        pluginScanCache = list.getTypes();
        pluginScanProgress.cacheLoaded.store(true, std::memory_order_relaxed);
        return pluginScanCache.size() > 0;
    }

    /** Resolve a UI path from the scan cache only.  Never call findAllTypesForFile on the
        message thread — WaveShell and heavy plugins can block indefinitely during introspection. */
    bool resolvePluginDescriptionForInsert(const juce::String& path, juce::PluginDescription& out)
    {
        std::lock_guard<std::mutex> scanLock(pluginScanMutex);
        for (const auto& t : pluginScanCache)
        {
            if (t.fileOrIdentifier == path)
            {
                out = t;
                return true;
            }
        }
        return false;
    }

    juce::var pluginChainLocked()
    {
        juce::Array<juce::var> slotRows;
        juce::Array<juce::var> insertPathVars;
        if (insertRunner != nullptr)
        {
            for (const auto& p : insertRunner->paths)
            {
                insertPathVars.add(p);
                auto* slot = new juce::DynamicObject();
                slot->setProperty("path", p);
                slotRows.add(slot);
            }
        }

        juce::Array<juce::var> fmts;
        fmts.add("VST3");
#if JUCE_MAC
        fmts.add("AU");
#endif

        std::lock_guard<std::mutex> scanLock(pluginScanMutex);
        if (pluginScanPhase == PluginScanPhase::Failed)
        {
            const juce::String err = pluginScanLastError;
            pluginScanPhase = PluginScanPhase::Idle;
            pluginScanLastError.clear();
            juce::var out = okObj();
            if (auto* o = out.getDynamicObject())
            {
                o->setProperty("phase", "failed");
                o->setProperty("api_version", 2);
                o->setProperty("slots", juce::var(slotRows));
                o->setProperty("insert_paths", juce::var(insertPathVars));
                o->setProperty("formats_planned", juce::var(fmts));
                o->setProperty("plugins", juce::Array<juce::var>());
                o->setProperty("plugin_count", 0);
                o->setProperty("error", err);
                o->setProperty("note", err.isNotEmpty() ? err : juce::String("plugin scan failed"));
            }
            return out;
        }

        if (pluginScanPhase == PluginScanPhase::Idle)
        {
            pluginScanProgress.resetForNewScan();
            if (!hydratePluginScanCacheFromDiskIfAvailable())
            {
                pluginScanPhase = PluginScanPhase::Running;
                if (pluginScanThread.joinable())
                    pluginScanThread.join();
                pluginScanThread = std::thread([this]() { runPluginScanWorker(); });
                juce::var out = okObj();
                if (auto* o = out.getDynamicObject())
                {
                    o->setProperty("phase", "scanning");
                    o->setProperty("api_version", 2);
                    o->setProperty("slots", juce::var(slotRows));
                    o->setProperty("insert_paths", juce::var(insertPathVars));
                    o->setProperty("formats_planned", juce::var(fmts));
                    o->setProperty("plugins", juce::Array<juce::var>());
                    o->setProperty("plugin_count", 0);
                    o->setProperty("note", "JUCE plugin scan running on background thread; poll plugin_chain until phase is not scanning.");
                    o->setProperty("scan_done", pluginScanProgress.done.load(std::memory_order_relaxed));
                    o->setProperty("scan_total", pluginScanProgress.total.load(std::memory_order_relaxed));
                    o->setProperty("scan_skipped", pluginScanProgress.skipped.load(std::memory_order_relaxed));
                    o->setProperty("scan_cache_loaded", pluginScanProgress.cacheLoaded.load(std::memory_order_relaxed));
                    {
                        std::lock_guard<std::mutex> lk(pluginScanProgress.mutex);
                        o->setProperty("scan_current_format", pluginScanProgress.currentFormat);
                        o->setProperty("scan_current_name", pluginScanProgress.currentName);
                    }
                }
                return out;
            }
            pluginScanPhase = PluginScanPhase::Done;
        }

        if (pluginScanPhase == PluginScanPhase::Running)
        {
            juce::var out = okObj();
            if (auto* o = out.getDynamicObject())
            {
                o->setProperty("phase", "scanning");
                o->setProperty("api_version", 2);
                o->setProperty("slots", juce::var(slotRows));
                o->setProperty("insert_paths", juce::var(insertPathVars));
                o->setProperty("formats_planned", juce::var(fmts));
                o->setProperty("plugins", juce::Array<juce::var>());
                o->setProperty("plugin_count", 0);
                o->setProperty("note", "JUCE plugin scan running on background thread; poll plugin_chain until phase is not scanning.");
                o->setProperty("scan_done", pluginScanProgress.done.load(std::memory_order_relaxed));
                o->setProperty("scan_total", pluginScanProgress.total.load(std::memory_order_relaxed));
                o->setProperty("scan_skipped", pluginScanProgress.skipped.load(std::memory_order_relaxed));
                o->setProperty("scan_cache_loaded", pluginScanProgress.cacheLoaded.load(std::memory_order_relaxed));
                {
                    std::lock_guard<std::mutex> lk(pluginScanProgress.mutex);
                    o->setProperty("scan_current_format", pluginScanProgress.currentFormat);
                    o->setProperty("scan_current_name", pluginScanProgress.currentName);
                }
            }
            return out;
        }

        juce::Array<juce::var> plugins;
        for (const auto& t : pluginScanCache)
        {
            auto* row = new juce::DynamicObject();
            row->setProperty("name", t.name);
            row->setProperty("format", t.pluginFormatName);
            row->setProperty("path", t.fileOrIdentifier);
            row->setProperty("category", t.category);
            row->setProperty("isInstrument", t.isInstrument);
            plugins.add(row);
        }
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
        {
            o->setProperty("phase", "juce");
            o->setProperty("api_version", 2);
            o->setProperty("slots", juce::var(slotRows));
            o->setProperty("insert_paths", juce::var(insertPathVars));
            o->setProperty("formats_planned", juce::var(fmts));
            o->setProperty("plugins", juce::var(plugins));
            o->setProperty("plugin_count", plugins.size());
            o->setProperty(
                "note",
                "playback_set_inserts loads VST3 bundles, AU .components, or cached fileOrIdentifier strings (e.g. AudioUnit:…); "
                "order is serial before device. Stop output stream first. "
                "playback_open_insert_editor opens native plug-in UIs (chain slot index).");
        }
        return out;
    }

    juce::var pluginRescanLocked(const juce::var& req)
    {
        if (req.hasProperty("timeout_sec"))
        {
            const int sec = (int) req["timeout_sec"];
            if (sec >= 5 && sec <= 3600)
            {
                const juce::String val(sec);
                setProcessEnv("AUDIO_HAXOR_PLUGIN_SCAN_TIMEOUT_SEC", val.toRawUTF8());
                appLogLine("plugin_rescan: timeout set to " + val + "s");
            }
        }
        {
            std::lock_guard<std::mutex> scanLock(pluginScanMutex);
            if (pluginScanPhase == PluginScanPhase::Running)
            {
                pluginScanCancel.store(true, std::memory_order_relaxed);
            }
        }

        if (pluginScanThread.joinable())
            pluginScanThread.join();
        pluginScanCancel.store(false, std::memory_order_relaxed);

        rotateKnownPluginListBackupsBeforeWipe();
        (void) pluginScanSkipFilePath().deleteFile();
        (void) deadMansPedalFilePath().deleteFile();

        {
            std::lock_guard<std::mutex> scanLock(pluginScanMutex);
            pluginScanCache.clear();
            pluginScanPhase = PluginScanPhase::Idle;
            pluginScanLastError.clear();
        }

        appLogLine("plugin_rescan: wiped cache, skip-list, dead-man's-pedal; phase reset to Idle");
        return okObj();
    }

    juce::var playbackSetInserts(const juce::var& req)
    {
        appLogLine("playback_set_inserts: ENTER");

        // ── Phase 1: under lock — validate request, close editors, resolve descriptions ──
        struct SlotInfo { juce::String path; juce::PluginDescription desc; };
        std::vector<SlotInfo> slots;
        {
            std::lock_guard<std::mutex> lock(mutex);
            closeAllInsertEditorsLocked();
            appLogLine("playback_set_inserts: editors closed");
            if (outputRunning)
                return errObj("stop_output_stream before changing inserts");
            const juce::var pathsVar = req["paths"];
            if (!pathsVar.isArray())
                return errObj("paths must be an array");
            auto* arr = pathsVar.getArray();
            if (arr == nullptr)
                return errObj("paths must be an array");
            if (arr->isEmpty())
            {
                insertRunner.reset();
                juce::var out = okObj();
                if (auto* o = out.getDynamicObject())
                {
                    juce::Array<juce::var> empty;
                    o->setProperty("insert_paths", juce::var(empty));
                }
                return out;
            }
            constexpr int kMaxSlots = 32;
            for (int i = 0; i < arr->size() && i < kMaxSlots; ++i)
            {
                const juce::String path = (*arr)[i].toString();
                if (path.isEmpty())
                    continue;
                SlotInfo s;
                s.path = path;
                appLogLine("playback_set_inserts: resolving " + path);
                if (!resolvePluginDescriptionForInsert(path, s.desc))
                    return errObj("unknown plugin (not on disk and not in scan cache): " + path);
                appLogLine("playback_set_inserts: resolved " + s.desc.name);
                slots.push_back(std::move(s));
            }
        }
        // mutex released — message thread free to process AU/system callbacks

        // ── Phase 2: no lock — create plugin instances ──
        //
        // JUCE's `createPluginInstanceAsync` posts a message that calls
        // `AudioComponentInstanceNew` ON the message thread.  We pump messages
        // from the main thread while a background thread waits on the sync
        // `createPluginInstance` (which internally posts + waits on the
        // message-thread callback).  Because impl->mutex is NOT held,
        // message-delivered callbacks that need the mutex won't deadlock.
        auto next = std::make_unique<InsertChainRunner>();
        for (auto& s : slots)
        {
            appLogLine("playback_set_inserts: creating instance " + s.desc.name + "...");
            std::atomic<bool> done{false};
            std::unique_ptr<juce::AudioPluginInstance> inst;
            juce::String createErr;
            auto descCopy = s.desc;

            std::thread worker([&done, &inst, &createErr, &descCopy, this]() {
                inst = pluginFormatManager.createPluginInstance(descCopy, 44100.0, 512, createErr);
                done.store(true, std::memory_order_release);
            });

            const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(30);
            while (!done.load(std::memory_order_acquire))
            {
                if (std::chrono::steady_clock::now() > deadline)
                {
                    appLogLine("playback_set_inserts: TIMEOUT creating " + s.desc.name);
                    worker.detach();
                    return errObj("plugin creation timed out: " + s.desc.name);
                }
                std::this_thread::sleep_for(std::chrono::milliseconds(10));
            }
            worker.join();

            if (inst == nullptr)
            {
                appLogLine("playback_set_inserts: FAILED " + s.desc.name + ": " + createErr);
                return errObj("plugin load failed for " + s.desc.name + ": " + createErr);
            }
            appLogLine("playback_set_inserts: loaded " + s.desc.name);
            next->paths.push_back(s.path);
            next->instances.push_back(std::move(inst));
        }

        // ── Phase 3: re-lock — install the chain ──
        juce::var out;
        {
            std::lock_guard<std::mutex> lock(mutex);
            if (outputRunning)
                return errObj("output stream started while loading plugins — stop it first");
            insertRunner = std::move(next);
            out = okObj();
            if (auto* o = out.getDynamicObject())
            {
                juce::Array<juce::var> pathVars;
                for (const auto& p : insertRunner->paths)
                    pathVars.add(p);
                o->setProperty("insert_paths", juce::var(pathVars));
            }
        }
        /* `InsertChainRunner::prepare` normally runs from `DspStereoFileSource::prepareToPlay` when the
         * output device opens. If the user applies inserts with the stream stopped then opens the native
         * editor, instances were never prepared — many VST3/AU UIs stay blank until `prepareToPlay`.
         * Re-prepare when playback starts (same `prepare` path). */
        if (insertRunner != nullptr && insertRunner->isActive())
        {
            insertRunner->prepare(44100.0, 512);
            appLogLine("playback_set_inserts: prepared insert chain (44100 Hz, 512 samples) for editor / device");
        }
        return out;
    }

    void closeAllInsertEditorsLocked()
    {
        insertEditorWindows.clear();
    }

    void requestCloseInsertEditor(int slot)
    {
        juce::MessageManager::callAsync([this, slot]() {
            std::lock_guard<std::mutex> lock(mutex);
            if (slot >= 0 && slot < (int) insertEditorWindows.size())
                insertEditorWindows[(size_t) slot].reset();
        });
    }

    /** DocumentWindow + AU/VST editor must be created/destroyed on the JUCE message thread. */
    static void* openInsertEditorMessageThreadFn(void* userData)
    {
        struct Payload
        {
            Impl* self = nullptr;
            int slot = -1;
            juce::var result;
        };
        auto* p = static_cast<Payload*>(userData);
        try
        {
            juce::AudioPluginInstance* inst = nullptr;
            {
                std::lock_guard<std::mutex> lock(p->self->mutex);
                if (p->self->insertRunner == nullptr || p->slot < 0
                    || p->slot >= (int) p->self->insertRunner->instances.size())
                {
                    p->result = errObj("invalid insert slot");
                    return nullptr;
                }
                inst = p->self->insertRunner->instances[(size_t) p->slot].get();
                if (inst == nullptr)
                {
                    p->result = errObj("empty insert slot");
                    return nullptr;
                }
                p->self->insertEditorWindows.resize(p->self->insertRunner->instances.size());
                p->self->insertEditorWindows[(size_t) p->slot].reset();
            }
            auto w = std::make_unique<PluginEditorHostWindow>(
                p->slot,
                [self = p->self](int s) { self->requestCloseInsertEditor(s); },
                *inst);
            if (!w->hasEditorContent())
            {
                p->result = errObj("plugin has no editor");
                return nullptr;
            }
            w->setVisible(true);
            w->toFront(true);
            w->schedulePostShowLayout();
            {
                std::lock_guard<std::mutex> lock(p->self->mutex);
                if (p->self->insertRunner == nullptr || p->slot < 0
                    || p->slot >= (int) p->self->insertRunner->instances.size())
                {
                    p->result = errObj("insert chain changed while opening editor");
                    return nullptr;
                }
                p->self->insertEditorWindows.resize(p->self->insertRunner->instances.size());
                p->self->insertEditorWindows[(size_t) p->slot] = std::move(w);
            }
            p->result = okObj();
        }
        catch (...)
        {
            p->result = errObj("plugin editor failed");
        }
        return nullptr;
    }

    juce::var openInsertEditorOnMessageThread(const juce::var& req)
    {
        struct Payload
        {
            Impl* self = nullptr;
            int slot = -1;
            juce::var result;
        };
        Payload p;
        p.self = this;
        p.slot = (int) req["slot"];
        juce::MessageManager::getInstance()->callFunctionOnMessageThread(openInsertEditorMessageThreadFn, &p);
        return p.result;
    }

    static void* closeInsertEditorMessageThreadFn(void* userData)
    {
        struct Payload
        {
            Impl* self = nullptr;
            int slot = -1;
            juce::var result;
        };
        auto* p = static_cast<Payload*>(userData);
        try
        {
            std::lock_guard<std::mutex> lock(p->self->mutex);
            if (p->self->insertRunner == nullptr || p->slot < 0)
            {
                p->result = errObj("invalid insert slot");
                return nullptr;
            }
            p->self->insertEditorWindows.resize(p->self->insertRunner->instances.size());
            if (p->slot >= (int) p->self->insertEditorWindows.size())
            {
                p->result = errObj("invalid insert slot");
                return nullptr;
            }
            p->self->insertEditorWindows[(size_t) p->slot].reset();
            p->result = okObj();
        }
        catch (...)
        {
            p->result = errObj("close editor failed");
        }
        return nullptr;
    }

    juce::var closeInsertEditorOnMessageThread(const juce::var& req)
    {
        struct Payload
        {
            Impl* self = nullptr;
            int slot = -1;
            juce::var result;
        };
        Payload p;
        p.self = this;
        p.slot = (int) req["slot"];
        juce::MessageManager::getInstance()->callFunctionOnMessageThread(closeInsertEditorMessageThreadFn, &p);
        return p.result;
    }

    void stopOutputLocked()
    {
        const bool wasRunning = outputRunning;
        const juce::String prevName = outDeviceName;
        const juce::String prevId = outDeviceId;
        outputManager.removeAudioCallback(&sourcePlayer);
        clearSpectrumCallbacks();
        sourcePlayer.setSource(nullptr);
        transport.setSource(nullptr);
        transport.stop();
        transport.releaseResources();
        fileSource.reset();
        outputManager.closeAudioDevice();
        clearSpectrumRing();
        outputRunning = false;
        playbackMode = false;
        toneMode = false;
        playbackPeak.store(0.0f);
        if (wasRunning)
            appLogLine("audio device: output stream stopped name=\"" + prevName + "\" id=\"" + prevId + "\"");
    }

    void stopInputLocked()
    {
        const bool wasRunning = inputRunning;
        const juce::String prevName = inDeviceName;
        const juce::String prevId = inDeviceId;
        inputManager.removeAudioCallback(&inputCb);
        inputManager.closeAudioDevice();
        inputRunning = false;
        inputCb.peak.store(0.0f);
        if (wasRunning)
            appLogLine("audio device: input stream stopped name=\"" + prevName + "\" id=\"" + prevId + "\"");
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
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
        {
            o->setProperty("duration_sec", sessionDurationSec);
            o->setProperty("sample_rate_hz", (int) sessionSrcRate);
            o->setProperty("track_id", 0);
        }
        return out;
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
            clearSpectrumRing();
            wireSpectrumCallbacks();
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

        juce::String devName = resolveOutputDeviceName(outputManager, deviceId);
        if (devName.isEmpty() && !deviceId.isEmpty())
            return errObj("unknown device_id: " + deviceId);

        juce::AudioDeviceManager::AudioDeviceSetup setup;
        if (devName.isNotEmpty())
            setup.outputDeviceName = devName;
        setup.inputDeviceName = "";
        if (bf > 0)
            setup.bufferSize = (int) bf;

        const bool userSr = req.hasProperty("sample_rate_hz") && !req["sample_rate_hz"].isVoid();
        if (userSr)
        {
            const double sr = (double) (int) req["sample_rate_hz"];
            if (sr > 1000.0)
                setup.sampleRate = sr;
        }
        else if (startPlayback)
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

        maybeBumpBufferForStablePlayback(outputManager, dev, bf);
        if (dev == nullptr)
            return errObj("no output device");

        deviceRate.store((uint32_t) dev->getCurrentSampleRate());

        outDeviceId = outputIdForDeviceName(outputManager, dev->getName());
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
                fileSource->speedResampler =
                    std::make_unique<juce::ResamplingAudioSource>(fileSource->readerSource.get(), false, 2);
                fileSource->speedResampler->setResamplingRatio((double) juce::jlimit(0.25f, 2.0f, playbackSpeed.load()));
                fileSource->playbackSpeed = &playbackSpeed;
            }

            fileSource->playbackLoop = &playbackLoopWanted;
            transport.setSource(fileSource.get(), 0, nullptr, (double) sessionSrcRate);
            fileSource->insertChain = insertRunner.get();
            if (!reverseWanted)
                fileSource->setLooping(playbackLoopWanted.load());
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
        clearSpectrumRing();
        wireSpectrumCallbacks();

        {
            const char* mode = startPlayback ? "playback" : (tone ? "tone" : "silence");
            appLogLine("audio device: output stream started name=\"" + outDeviceName + "\" id=\"" + outDeviceId
                       + "\" sample_rate_hz=" + juce::String(outSampleRate) + " channels=" + juce::String(outChannels)
                       + " mode=" + juce::String(mode));
        }

        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
        {
            o->setProperty("device_id", outDeviceId);
            o->setProperty("device_name", outDeviceName);
            o->setProperty("sample_rate_hz", outSampleRate);
            o->setProperty("channels", outChannels);
            o->setProperty("sample_format", juce::String("F32"));
            o->setProperty("buffer_size", outBufferSizeJson);
            o->setProperty("stream_buffer_frames", outStreamBufferFrames.has_value() ? juce::var(*outStreamBufferFrames) : juce::var());
            o->setProperty("current_buffer_frames", dev != nullptr ? dev->getCurrentBufferSizeSamples() : 0);
            o->setProperty("tone_supported", true);
            o->setProperty("tone_on", !startPlayback && tone);
            o->setProperty("note", startPlayback ? juce::String("file playback via JUCE") : juce::String("silence or test tone"));
        }
        return out;
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
        juce::String devName = resolveInputDeviceName(inputManager, deviceId);
        if (devName.isEmpty() && !deviceId.isEmpty())
            return errObj("unknown device_id: " + deviceId);

        juce::AudioDeviceManager::AudioDeviceSetup setup;
        if (devName.isNotEmpty())
            setup.inputDeviceName = devName;
        setup.outputDeviceName = "";
        if (bf > 0)
            setup.bufferSize = (int) bf;
        if (req.hasProperty("sample_rate_hz") && !req["sample_rate_hz"].isVoid())
        {
            const double sr = (double) (int) req["sample_rate_hz"];
            if (sr > 1000.0)
                setup.sampleRate = sr;
        }

        inputManager.setAudioDeviceSetup(setup, true);
        juce::AudioIODevice* dev = inputManager.getCurrentAudioDevice();
        if (dev == nullptr)
            return errObj("no input device");

        maybeBumpBufferForStablePlayback(inputManager, dev, bf);
        if (dev == nullptr)
            return errObj("no input device");

        inDeviceId = inputIdForDeviceName(inputManager, dev->getName());
        inDeviceName = dev->getName();
        inSampleRate = (int) dev->getCurrentSampleRate();
        inChannels = juce::jmax(1, dev->getActiveInputChannels().countNumberOfSetBits());
        inBufferSizeJson = bufferSizeJson(dev);
        inStreamBufferFrames = (bf > 0) ? std::optional<int>((int) bf) : std::nullopt;

        inputCb.peak.store(0.0f);
        inputManager.addAudioCallback(&inputCb);

        inputRunning = true;

        appLogLine("audio device: input stream started name=\"" + inDeviceName + "\" id=\"" + inDeviceId
                   + "\" sample_rate_hz=" + juce::String(inSampleRate) + " channels=" + juce::String(inChannels));

        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
        {
            o->setProperty("device_id", inDeviceId);
            o->setProperty("device_name", inDeviceName);
            o->setProperty("sample_rate_hz", inSampleRate);
            o->setProperty("channels", inChannels);
            o->setProperty("sample_format", juce::String("F32"));
            o->setProperty("buffer_size", inBufferSizeJson);
            o->setProperty("stream_buffer_frames", inStreamBufferFrames.has_value() ? juce::var(*inStreamBufferFrames) : juce::var());
            o->setProperty("current_buffer_frames", dev != nullptr ? dev->getCurrentBufferSizeSamples() : 0);
            o->setProperty("input_peak", 0.0);
            o->setProperty("note", "input capture running; samples discarded; input_peak is block peak with decay");
        }
        return out;
    }

    juce::var playbackStatusLocked()
    {
        juce::var out = okObj();
        auto* o = out.getDynamicObject();
        if (o == nullptr)
            return out;
        if (sessionPath.isEmpty())
        {
            o->setProperty("loaded", false);
            appendPlaybackSpectrumJson(o);
            return out;
        }
        o->setProperty("loaded", true);
        o->setProperty("duration_sec", sessionDurationSec);
        o->setProperty("sample_rate_hz", (int) deviceRate.load());
        o->setProperty("src_rate_hz", (int) sessionSrcRate);
        o->setProperty("reverse", reverseWanted);
        o->setProperty("speed", (double) juce::jlimit(0.25f, 2.0f, playbackSpeed.load()));
        if (!playbackMode)
        {
            o->setProperty("position_sec", 0.0);
            o->setProperty("peak", playbackPeak.load());
            o->setProperty("paused", false);
            o->setProperty("eof", false);
            appendPlaybackSpectrumJson(o);
            return out;
        }
        /* Forward + resampler: timeline from reader samples (transport time drifts vs. ResamplingAudioSource). */
        double posSrc = transport.getCurrentPosition();
        if (!reverseWanted && fileSource != nullptr && fileSource->readerSource != nullptr)
        {
            const juce::int64 sp = fileSource->readerSource->getNextReadPosition();
            posSrc = (double) sp / juce::jmax(1.0e-9, (double) sessionSrcRate);
        }
        double pos = reverseWanted ? (sessionDurationSec - posSrc) : posSrc;
        pos = juce::jlimit(0.0, sessionDurationSec, pos);
        o->setProperty("position_sec", pos);
        o->setProperty("peak", playbackPeak.load());
        o->setProperty("paused", paused);
        o->setProperty("eof", transport.hasStreamFinished());
        appendPlaybackSpectrumJson(o);
        return out;
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
        {
            int curBf = 0;
            if (juce::AudioIODevice* d = outputManager.getCurrentAudioDevice())
                curBf = d->getCurrentBufferSizeSamples();
            o->setProperty("current_buffer_frames", curBf);
        }
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
        {
            int curBf = 0;
            if (juce::AudioIODevice* d = inputManager.getCurrentAudioDevice())
                curBf = d->getCurrentBufferSizeSamples();
            o->setProperty("current_buffer_frames", curBf);
        }
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
    const juce::String cmd = cmdKey(req);
    if (cmd == "waveform_preview" || cmd == "spectrogram_preview")
    {
        // High-frequency IPC: omit from engine.log (same as ping / playback_status polls).
        if (cmd.isNotEmpty() && cmd != "ping" && cmd != "playback_status" && cmd != "playback_seek")
            appLogLine("cmd " + cmd);
        // Decode off the stdin thread; do not hold `impl->mutex` during heavy IIR/FFT work.
        auto fut = std::async(std::launch::async, [this, req, cmd]() -> juce::var {
            std::lock_guard<std::mutex> lk(impl->previewMutex);
            if (cmd == "waveform_preview")
                return waveformPreview(impl->formatManager, req);
            return spectrogramPreview(impl->formatManager, req);
        });
        return fut.get();
    }

    if (cmd == "playback_set_inserts")
    {
        appLogLine("cmd " + cmd);
        return impl->playbackSetInserts(req);
    }

    if (cmd == "playback_open_insert_editor")
    {
        appLogLine("cmd " + cmd);
        return impl->openInsertEditorOnMessageThread(req);
    }

    if (cmd == "playback_close_insert_editor")
    {
        appLogLine("cmd " + cmd);
        return impl->closeInsertEditorOnMessageThread(req);
    }

    std::lock_guard<std::mutex> lock(impl->mutex);
    // High-frequency IPC: omit from engine.log (same as ping / playback_status polls).
    if (cmd.isNotEmpty() && cmd != "ping" && cmd != "playback_status" && cmd != "playback_seek")
        appLogLine("cmd " + cmd);

    if (cmd == "ping")
    {
        auto* o = new juce::DynamicObject();
        o->setProperty("ok", true);
        o->setProperty("version", juce::String(AUDIO_ENGINE_VERSION_STRING));
        o->setProperty("host", juce::String("juce"));
        return o;
    }

    /** Only reads the file / session fields — no `AudioDeviceManager` needed; defer CoreAudio init to `start_output_stream`. */
    if (cmd == "playback_load")
        return impl->playbackLoad(req);

    impl->ensureAudioDeviceManagersInitialised();

    if (cmd == "engine_state")
        return impl->engineStateLocked();

    if (cmd == "output_stream_status")
        return impl->outputStreamStatusLocked();

    if (cmd == "input_stream_status")
        return impl->inputStreamStatusLocked();

    if (cmd == "list_output_devices")
    {
        juce::StringArray ids, names;
        enumerateOutputIds(impl->outputManager, ids, names);
        juce::String defaultId;
        juce::AudioIODevice* cur = impl->outputManager.getCurrentAudioDevice();
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
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
        {
            o->setProperty("default_device_id", defaultId.isEmpty() ? juce::var() : juce::var(defaultId));
            o->setProperty("devices", juce::var(rows));
        }
        appLogLine("audio device: output discovery type=\"" + impl->outputManager.getCurrentAudioDeviceType() + "\" count="
                    + juce::String(names.size()) + " default_id=\"" + defaultId + "\" default_name=\"" + curName + "\"");
        return out;
    }

    if (cmd == "list_input_devices")
    {
        juce::StringArray ids, names;
        enumerateInputIds(impl->inputManager, ids, names);
        juce::String defaultId;
        juce::AudioIODevice* cur = impl->inputManager.getCurrentAudioDevice();
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
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
        {
            o->setProperty("default_device_id", defaultId.isEmpty() ? juce::var() : juce::var(defaultId));
            o->setProperty("devices", juce::var(rows));
        }
        appLogLine("audio device: input discovery type=\"" + impl->inputManager.getCurrentAudioDeviceType() + "\" count="
                    + juce::String(names.size()) + " default_id=\"" + defaultId + "\" default_name=\"" + curName + "\"");
        return out;
    }

    if (cmd == "get_output_device_info")
    {
        const juce::String id = req["device_id"].toString();
        juce::AudioDeviceManager probe;
        probe.initialise(0, 2, nullptr, true);
        copyDeviceType(impl->outputManager, probe);
        juce::String name = resolveOutputDeviceName(probe, id);
        if (name.isEmpty() && !id.isEmpty())
            return errObj("unknown device_id: " + id);
        if (name.isEmpty())
        {
            juce::StringArray ids, names;
            enumerateOutputIds(probe, ids, names);
            if (names.size() > 0)
                name = names[0];
        }
        juce::AudioDeviceManager::AudioDeviceSetup setup;
        setup.outputDeviceName = name;
        setup.inputDeviceName = "";
        probe.setAudioDeviceSetup(setup, true);
        juce::AudioIODevice* dev = probe.getCurrentAudioDevice();
        if (dev == nullptr)
            return errObj("no output device");
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
        {
            o->setProperty("device_name", dev->getName());
            o->setProperty("sample_rate_hz", dev->getCurrentSampleRate());
            o->setProperty("channels", dev->getActiveOutputChannels().countNumberOfSetBits());
            o->setProperty("sample_format", juce::String("F32"));
            o->setProperty("buffer_size", bufferSizeJson(dev));
            {
                juce::Array<juce::var> rates;
                for (double r : dev->getAvailableSampleRates())
                    rates.add(r);
                o->setProperty("sample_rates", juce::var(rates));
            }
            {
                juce::Array<juce::var> bufs;
                for (int b : dev->getAvailableBufferSizes())
                    bufs.add(b);
                o->setProperty("buffer_sizes", juce::var(bufs));
            }
            o->setProperty("audio_device_type", impl->outputManager.getCurrentAudioDeviceType());
        }
        appLogLine("audio device: output info probe name=\"" + dev->getName() + "\" sample_rate_hz="
                    + juce::String(dev->getCurrentSampleRate()) + " type=\"" + impl->outputManager.getCurrentAudioDeviceType() + "\"");
        return out;
    }

    if (cmd == "get_input_device_info")
    {
        const juce::String id = req["device_id"].toString();
        juce::AudioDeviceManager probe;
        probe.initialise(2, 0, nullptr, true);
        copyDeviceType(impl->inputManager, probe);
        juce::String name = resolveInputDeviceName(probe, id);
        if (name.isEmpty() && !id.isEmpty())
            return errObj("unknown device_id: " + id);
        if (name.isEmpty())
        {
            juce::StringArray ids, names;
            enumerateInputIds(probe, ids, names);
            if (names.size() > 0)
                name = names[0];
        }
        juce::AudioDeviceManager::AudioDeviceSetup setup;
        setup.inputDeviceName = name;
        setup.outputDeviceName = "";
        probe.setAudioDeviceSetup(setup, true);
        juce::AudioIODevice* dev = probe.getCurrentAudioDevice();
        if (dev == nullptr)
            return errObj("no input device");
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
        {
            o->setProperty("device_name", dev->getName());
            o->setProperty("sample_rate_hz", dev->getCurrentSampleRate());
            o->setProperty("channels", dev->getActiveInputChannels().countNumberOfSetBits());
            o->setProperty("sample_format", juce::String("F32"));
            o->setProperty("buffer_size", bufferSizeJson(dev));
            {
                juce::Array<juce::var> rates;
                for (double r : dev->getAvailableSampleRates())
                    rates.add(r);
                o->setProperty("sample_rates", juce::var(rates));
            }
            {
                juce::Array<juce::var> bufs;
                for (int b : dev->getAvailableBufferSizes())
                    bufs.add(b);
                o->setProperty("buffer_sizes", juce::var(bufs));
            }
            o->setProperty("audio_device_type", impl->inputManager.getCurrentAudioDeviceType());
        }
        appLogLine("audio device: input info probe name=\"" + dev->getName() + "\" sample_rate_hz="
                    + juce::String(dev->getCurrentSampleRate()) + " type=\"" + impl->inputManager.getCurrentAudioDeviceType() + "\"");
        return out;
    }

    if (cmd == "list_audio_device_types")
    {
        // createAudioDeviceTypes → CoreAudio: can block a long time or appear to hang when the
        // binary is driven by a shell pipe (no normal GUI app context); use ping for pipe smoke tests.
        juce::OwnedArray<juce::AudioIODeviceType> types;
        createFreshDeviceTypes(impl->outputManager, types);
        /* Array of string vars only — avoids nested DynamicObject::setProperty per row (SIGSEGV on macOS
           when building {type:…} objects after createAudioDeviceTypes). JSON: ["CoreAudio", …]. */
        juce::Array<juce::var> typeNames;
        juce::StringArray seenTypeNames;
        for (auto* ty : types)
        {
            if (ty == nullptr)
                continue;
            const juce::String tn = ty->getTypeName();
            if (tn.isEmpty() || seenTypeNames.contains(tn))
                continue;
            seenTypeNames.add(tn);
            typeNames.add(juce::var(tn));
        }
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
        {
            o->setProperty("types", juce::var(typeNames));
            o->setProperty("current", impl->outputManager.getCurrentAudioDeviceType());
        }
        appLogLine("audio device: types discovery count=" + juce::String(typeNames.size()) + " current=\""
                    + impl->outputManager.getCurrentAudioDeviceType() + "\"");
        return out;
    }

    if (cmd == "set_audio_device_type")
    {
        const juce::String t = req["type"].toString();
        if (t.isEmpty())
            return errObj("type required");
        impl->stopOutputLocked();
        impl->stopInputLocked();
        impl->outputManager.setCurrentAudioDeviceType(t, true);
        impl->inputManager.setCurrentAudioDeviceType(t, true);
        appLogLine("audio device: driver type set to \"" + t + "\" (streams stopped)");
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
        {
            o->setProperty("type", t);
            o->setProperty("note", "streams stopped; device lists refreshed for this driver");
        }
        return out;
    }

    if (cmd == "set_output_device")
    {
        const juce::String id = req["device_id"].toString();
        if (id.isEmpty())
            return errObj("device_id required");
        const juce::String resolvedName = resolveOutputDeviceName(impl->outputManager, id);
        if (resolvedName.isEmpty())
            return errObj("unknown device_id: " + id);
        appLogLine("audio device: output device_id selected id=\"" + id + "\" name=\"" + resolvedName + "\" (stream not open)");
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
        {
            o->setProperty("device_id", id);
            o->setProperty("note", "validated; use start_output_stream to open the device");
        }
        return out;
    }

    if (cmd == "set_input_device")
    {
        const juce::String id = req["device_id"].toString();
        if (id.isEmpty())
            return errObj("device_id required");
        const juce::String resolvedNameIn = resolveInputDeviceName(impl->inputManager, id);
        if (resolvedNameIn.isEmpty())
            return errObj("unknown device_id: " + id);
        appLogLine("audio device: input device_id selected id=\"" + id + "\" name=\"" + resolvedNameIn + "\" (stream not open)");
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
        {
            o->setProperty("device_id", id);
            o->setProperty("note", "validated; use start_input_stream to open capture");
        }
        return out;
    }

    if (cmd == "start_output_stream")
        return impl->startOutputStreamLocked(req);

    if (cmd == "start_input_stream")
        return impl->startInputStreamLocked(req);

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
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
            o->setProperty("paused", p);
        return out;
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
        impl->playbackSpeed.store(s);
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
        {
            o->setProperty("speed", s);
            if (impl->reverseWanted)
                o->setProperty("note", "speed stored; reverse path plays at 1× (resampler skipped)");
        }
        return out;
    }

    if (cmd == "playback_set_reverse")
    {
        const bool en = req["reverse"].isVoid() ? false : (bool) req["reverse"];
        impl->reverseWanted = en;
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
            o->setProperty("reverse", en);
        return out;
    }

    if (cmd == "playback_set_loop")
    {
        const bool loop = req["loop"].isVoid() ? false : (bool) req["loop"];
        impl->playbackLoopWanted.store(loop);
        if (impl->playbackMode && impl->fileSource != nullptr && !impl->reverseWanted)
            impl->fileSource->setLooping(loop);
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
            o->setProperty("loop", loop);
        return out;
    }

    if (cmd == "playback_status")
        return impl->playbackStatusLocked();

    if (cmd == "set_output_tone")
    {
        const bool t = req["tone"].isVoid() ? false : (bool) req["tone"];
        if (!impl->outputRunning)
            return errObj("no output stream");
        impl->toneSource.toneOn.store(t);
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
            o->setProperty("tone", t);
        return out;
    }

    if (cmd == "stop_output_stream")
    {
        const bool was = impl->outputRunning;
        impl->stopOutputLocked();
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
            o->setProperty("was_running", was);
        return out;
    }

    if (cmd == "stop_input_stream")
    {
        const bool was = impl->inputRunning;
        impl->stopInputLocked();
        juce::var out = okObj();
        if (auto* o = out.getDynamicObject())
            o->setProperty("was_running", was);
        return out;
    }

    if (cmd == "plugin_chain")
        return impl->pluginChainLocked();

    if (cmd == "plugin_rescan")
        return impl->pluginRescanLocked(req);

    return errObj("unknown cmd: " + cmd);
}

void Engine::shutdownEditors()
{
    std::lock_guard<std::mutex> lock(impl->mutex);
    impl->closeAllInsertEditorsLocked();
}

int runPluginScanOneChild(int argc, char* argv[])
{
    juce::ignoreUnused(argc);
    try
    {
        const juce::String formatLabel = argv[2];
        juce::MemoryOutputStream mos;
        if (!juce::Base64::convertFromBase64(mos, argv[3]))
            return 30;
        const juce::String fileId = mos.toString();
        mos.reset();
        if (!juce::Base64::convertFromBase64(mos, argv[4]))
            return 31;
        const juce::File outFile(mos.toString());

        juce::KnownPluginList list;
        const juce::File cacheFile = knownPluginListCacheFilePath();
        if (cacheFile.existsAsFile())
        {
            juce::XmlDocument doc(cacheFile);
            if (std::unique_ptr<juce::XmlElement> root = doc.getDocumentElement())
                list.recreateFromXml(*root);
        }

        juce::VST3PluginFormat vst3;
#if JUCE_MAC
        juce::AudioUnitPluginFormat auFormat;
#endif
        juce::AudioPluginFormat* fmt = nullptr;
        juce::FileSearchPath dirs;
        if (formatLabel == "VST3")
        {
            fmt = &vst3;
            dirs = vst3.getDefaultLocationsToSearch();
        }
#if JUCE_MAC
        else if (formatLabel == "AU")
        {
            fmt = &auFormat;
            dirs = auFormat.getDefaultLocationsToSearch();
        }
#endif
        else
            return 5;

        const juce::File deadMans = deadMansPedalFilePath();
        juce::StringArray single;
        single.add(fileId);
        juce::PluginDirectoryScanner scanner(list, *fmt, dirs, true, deadMans, false);
        scanner.setFilesOrIdentifiersToScan(single);
        juce::String name;
        (void) scanner.scanNextFile(true, name);

        if (auto xml = list.createXml())
        {
            (void) outFile.getParentDirectory().createDirectory();
            if (!xml->writeTo(outFile, {}))
                return 40;
        }
        else
            return 41;
        return 0;
    }
    catch (const std::exception& e)
    {
        appLogLine(juce::String("plugin-scan-one: exception ") + e.what());
        return 1;
    }
    catch (...)
    {
        appLogLine("plugin-scan-one: exception (non-std)");
        return 2;
    }
}

} // namespace audio_haxor
