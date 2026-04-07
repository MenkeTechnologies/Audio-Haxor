#include "VisualPreview.hpp"

#include <algorithm>
#include <cmath>
#include <memory>
#include <vector>

#include <juce_dsp/juce_dsp.h>

namespace audio_haxor {
namespace {

static juce::var errPrev(const juce::String& msg)
{
    auto* o = new juce::DynamicObject();
    o->setProperty("ok", false);
    o->setProperty("error", msg);
    return o;
}

static juce::var okPrev()
{
    auto* o = new juce::DynamicObject();
    o->setProperty("ok", true);
    return o;
}

static int clampInt(const juce::var& v, int lo, int hi, int dflt)
{
    if (v.isVoid())
        return dflt;
    if (v.isInt() || v.isInt64() || v.isDouble())
    {
        const int x = (int) v;
        return juce::jlimit(lo, hi, x);
    }
    return dflt;
}

static double clampDouble(const juce::var& v, double lo, double hi, double dflt)
{
    if (v.isVoid())
        return dflt;
    if (v.isInt() || v.isInt64() || v.isDouble())
    {
        const double x = (double) v;
        return juce::jlimit(lo, hi, x);
    }
    return dflt;
}

/** Largest power of two ≤ `n` (minimum 4). */
static int highestPowerOf2AtMost(int n)
{
    if (n < 4)
        return 0;
    unsigned int p = 1U;
    while (p * 2U <= (unsigned int) n)
        p *= 2U;
    return (int) p;
}

static float magnitudeToDb(float mag)
{
    const float m = juce::jmax(mag, 1.0e-10f);
    return 20.0f * std::log10(m);
}

} // namespace

juce::var waveformPreview(juce::AudioFormatManager& fm, const juce::var& req)
{
    const juce::String path = req["path"].toString();
    if (path.isEmpty())
        return errPrev("path required");

    const juce::File f(path);
    if (!f.existsAsFile())
        return errPrev("not a file: " + path);

    std::unique_ptr<juce::AudioFormatReader> reader(fm.createReaderFor(f));
    if (reader == nullptr)
        return errPrev("unsupported or unreadable file");

    constexpr double kMaxDurationSec = 300.0;
    constexpr int kMaxWidthPx = 8192;

    const int widthPx = clampInt(req["width_px"], 32, kMaxWidthPx, 800);
    const double sr = reader->sampleRate;
    const int64_t totalSamples = reader->lengthInSamples;
    const int numCh = juce::jmax(1, (int) reader->numChannels);
    const double fileDurSec = (double) totalSamples / juce::jmax(1.0, sr);

    double startSec = 0.0;
    if (req.hasProperty("start_sec") && !req["start_sec"].isVoid())
        startSec = clampDouble(req["start_sec"], 0.0, fileDurSec, 0.0);

    double durationSec = fileDurSec - startSec;
    if (req.hasProperty("duration_sec") && !req["duration_sec"].isVoid())
        durationSec = clampDouble(req["duration_sec"], 0.0, kMaxDurationSec, durationSec);
    durationSec = juce::jmin(durationSec, kMaxDurationSec);
    durationSec = juce::jmin(durationSec, fileDurSec - startSec);

    int64_t startSample = (int64_t) std::llround(startSec * sr);
    {
        const int64_t maxStart = totalSamples > 0 ? totalSamples - 1 : 0;
        startSample = std::clamp(startSample, (int64_t) 0, maxStart);
    }
    int64_t numSamples = (int64_t) std::llround(durationSec * sr);
    numSamples = juce::jmin(numSamples, totalSamples - startSample);
    if (numSamples < 1)
        return errPrev("empty segment");

    std::vector<float> mono((size_t) numSamples, 0.f);

    constexpr int kChunk = 65536;
    int64_t readOff = 0;
    while (readOff < numSamples)
    {
        const int n = (int) juce::jmin<int64_t>(kChunk, numSamples - readOff);
        std::vector<std::vector<float>> ch((size_t) numCh);
        for (int c = 0; c < numCh; ++c)
            ch[(size_t) c].resize((size_t) n);
        std::vector<float*> ptrs((size_t) numCh);
        for (int c = 0; c < numCh; ++c)
            ptrs[(size_t) c] = ch[(size_t) c].data();

        if (!reader->read(ptrs.data(), numCh, startSample + readOff, n))
            return errPrev("read failed");

        for (int i = 0; i < n; ++i)
        {
            float s = 0.f;
            for (int c = 0; c < numCh; ++c)
                s += ch[(size_t) c][(size_t) i];
            mono[(size_t) (readOff + (int64_t) i)] = s / (float) numCh;
        }
        readOff += (int64_t) n;
    }

    const double samplesPerCol = (double) numSamples / (double) widthPx;
    juce::Array<juce::var> peaks;
    peaks.ensureStorageAllocated(widthPx);

    for (int col = 0; col < widthPx; ++col)
    {
        const int64_t a = (int64_t) std::floor((double) col * samplesPerCol);
        const int64_t bEx = (int64_t) std::floor((double) (col + 1) * samplesPerCol);
        const int64_t b = juce::jmax(a + 1, bEx);
        float mn = 1.f;
        float mx = -1.f;
        for (int64_t i = a; i < b && i < numSamples; ++i)
        {
            const float v = mono[(size_t) i];
            mn = juce::jmin(mn, v);
            mx = juce::jmax(mx, v);
        }
        auto* pair = new juce::DynamicObject();
        pair->setProperty("min", mn);
        pair->setProperty("max", mx);
        peaks.add(pair);
    }

    juce::var out = okPrev();
    if (auto* o = out.getDynamicObject())
    {
        o->setProperty("sample_rate_hz", (int) std::lround(sr));
        o->setProperty("channels", numCh);
        o->setProperty("file_duration_sec", fileDurSec);
        o->setProperty("start_sec", startSec);
        o->setProperty("duration_sec", (double) numSamples / sr);
        o->setProperty("width_px", widthPx);
        o->setProperty("peaks", juce::var(peaks));
    }
    return out;
}

juce::var spectrogramPreview(juce::AudioFormatManager& fm, const juce::var& req)
{
    const juce::String path = req["path"].toString();
    if (path.isEmpty())
        return errPrev("path required");

    const juce::File f(path);
    if (!f.existsAsFile())
        return errPrev("not a file: " + path);

    std::unique_ptr<juce::AudioFormatReader> reader(fm.createReaderFor(f));
    if (reader == nullptr)
        return errPrev("unsupported or unreadable file");

    constexpr double kMaxDurationSec = 120.0;
    constexpr int kMaxWidthPx = 512;
    constexpr int kMaxHeightPx = 512;

    const int widthPx = clampInt(req["width_px"], 16, kMaxWidthPx, 256);
    const int heightPx = clampInt(req["height_px"], 16, kMaxHeightPx, 128);
    int fftOrder = clampInt(req["fft_order"], 8, 15, 11);
    int fftSize = 1 << fftOrder;

    const double sr = reader->sampleRate;
    const int64_t totalSamples = reader->lengthInSamples;
    const int numCh = juce::jmax(1, (int) reader->numChannels);
    const double fileDurSec = (double) totalSamples / juce::jmax(1.0, sr);

    double startSec = 0.0;
    if (req.hasProperty("start_sec") && !req["start_sec"].isVoid())
        startSec = clampDouble(req["start_sec"], 0.0, fileDurSec, 0.0);

    double durationSec = fileDurSec - startSec;
    if (req.hasProperty("duration_sec") && !req["duration_sec"].isVoid())
        durationSec = clampDouble(req["duration_sec"], 0.0, kMaxDurationSec, durationSec);
    durationSec = juce::jmin(durationSec, kMaxDurationSec);
    durationSec = juce::jmin(durationSec, fileDurSec - startSec);

    int64_t startSample = (int64_t) std::llround(startSec * sr);
    {
        const int64_t maxStart = totalSamples > 0 ? totalSamples - 1 : 0;
        startSample = std::clamp(startSample, (int64_t) 0, maxStart);
    }
    int64_t numSamples = (int64_t) std::llround(durationSec * sr);
    numSamples = juce::jmin(numSamples, totalSamples - startSample);
    if (numSamples < 8)
        return errPrev("segment too short for spectrogram");

    if (fftSize > (int) numSamples)
    {
        fftSize = highestPowerOf2AtMost((int) numSamples);
        fftOrder = 0;
        for (int x = fftSize; x > 1; x >>= 1)
            ++fftOrder;
    }

    std::vector<float> mono((size_t) numSamples, 0.f);
    constexpr int kChunk = 65536;
    int64_t readOff = 0;
    while (readOff < numSamples)
    {
        const int n = (int) juce::jmin<int64_t>(kChunk, numSamples - readOff);
        std::vector<std::vector<float>> ch((size_t) numCh);
        for (int c = 0; c < numCh; ++c)
            ch[(size_t) c].resize((size_t) n);
        std::vector<float*> ptrs((size_t) numCh);
        for (int c = 0; c < numCh; ++c)
            ptrs[(size_t) c] = ch[(size_t) c].data();

        if (!reader->read(ptrs.data(), numCh, startSample + readOff, n))
            return errPrev("read failed");

        for (int i = 0; i < n; ++i)
        {
            float s = 0.f;
            for (int c = 0; c < numCh; ++c)
                s += ch[(size_t) c][(size_t) i];
            mono[(size_t) (readOff + (int64_t) i)] = s / (float) numCh;
        }
        readOff += (int64_t) n;
    }

    juce::dsp::FFT fft(fftOrder);
    const int fftBins = fft.getSize() / 2 + 1;
    std::vector<float> window((size_t) fftSize);
    for (int i = 0; i < fftSize; ++i)
    {
        window[(size_t) i] = 0.5f * (1.0f - std::cos((float) (2.0 * juce::MathConstants<double>::pi * (double) i / (double) juce::jmax(1, fftSize - 1))));
    }

    std::vector<std::vector<float>> grid((size_t) heightPx, std::vector<float>((size_t) widthPx, -100.f));

    const int hopClamped = juce::jmax(1, (int) ((numSamples - (int64_t) fftSize) / (int64_t) juce::jmax(1, widthPx - 1)));

    for (int t = 0; t < widthPx; ++t)
    {
        const int64_t frameStart = juce::jmin<int64_t>((int64_t) t * (int64_t) hopClamped, numSamples - (int64_t) fftSize);
        std::vector<float> fftBuf((size_t) (fft.getSize() * 2), 0.f);
        for (int i = 0; i < fftSize; ++i)
            fftBuf[(size_t) i] = mono[(size_t) (frameStart + (int64_t) i)] * window[(size_t) i];

        fft.performFrequencyOnlyForwardTransform(fftBuf.data(), true);

        for (int freqRow = 0; freqRow < heightPx; ++freqRow)
        {
            const int binLo = (freqRow * fftBins) / heightPx;
            const int binHi = juce::jmax(binLo + 1, ((freqRow + 1) * fftBins) / heightPx);
            float magMax = 0.f;
            for (int b = binLo; b < binHi && b < fftBins; ++b)
                magMax = juce::jmax(magMax, fftBuf[(size_t) b]);

            const float db = magnitudeToDb(magMax);
            grid[(size_t) freqRow][(size_t) t] = juce::jlimit(-100.0f, 0.0f, db);
        }
    }

    juce::Array<juce::var> rowsOut;
    rowsOut.ensureStorageAllocated(heightPx);
    for (int r = 0; r < heightPx; ++r)
    {
        juce::Array<juce::var> rowVals;
        rowVals.ensureStorageAllocated(widthPx);
        for (int t = 0; t < widthPx; ++t)
            rowVals.add(grid[(size_t) r][(size_t) t]);
        rowsOut.add(juce::var(rowVals));
    }

    juce::var out = okPrev();
    if (auto* o = out.getDynamicObject())
    {
        o->setProperty("sample_rate_hz", (int) std::lround(sr));
        o->setProperty("channels", numCh);
        o->setProperty("file_duration_sec", fileDurSec);
        o->setProperty("start_sec", startSec);
        o->setProperty("duration_sec", (double) numSamples / sr);
        o->setProperty("fft_size", fftSize);
        o->setProperty("hop", hopClamped);
        o->setProperty("width_px", widthPx);
        o->setProperty("height_px", heightPx);
        o->setProperty("db_min", -100);
        o->setProperty("db_max", 0);
        o->setProperty("rows", juce::var(rowsOut));
    }
    return out;
}

} // namespace audio_haxor
