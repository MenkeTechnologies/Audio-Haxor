/**
 * Off-main-thread fetch + decodeAudioData + peak / spectrogram / PCM extraction.
 * Keeps the UI responsive for large WAVs and long MP3s.
 */
/* global self, OfflineAudioContext, fetch */

function computePeaksFromChannel(raw, bars) {
    const nBars = Math.max(1, Math.floor(Number(bars)) || 1);
    const step = Math.max(1, Math.floor(raw.length / nBars));
    const peaks = [];
    for (let i = 0; i < nBars; i++) {
        let max = 0;
        let min = 0;
        const start = i * step;
        const end = Math.min(start + step, raw.length);
        for (let j = start; j < end; j++) {
            const v = raw[j];
            if (v > max) max = v;
            if (v < min) min = v;
        }
        peaks.push({ max, min });
    }
    return peaks;
}

function computeSpectrogramData(raw) {
    const w = 800;
    const h = 80;
    const fftSize = 1024;
    const hop = fftSize / 2;
    const numBins = fftSize / 2;
    const minLen = fftSize + hop;
    let pcm = raw;
    if (raw.length < minLen) {
        const p = new Float32Array(minLen);
        p.set(raw);
        pcm = p;
    }
    const numFrames = Math.floor((pcm.length - fftSize) / hop);
    if (numFrames <= 0) return [];

    const cols = Math.min(w, numFrames);
    const frameStep = Math.max(1, Math.floor(numFrames / cols));
    const freqBins = 64;

    const hannWindow = new Float32Array(fftSize);
    for (let i = 0; i < fftSize; i++) {
        hannWindow[i] = 0.5 * (1 - Math.cos((2 * Math.PI * i) / (fftSize - 1)));
    }

    const bitRev = new Uint32Array(fftSize);
    const bits = Math.log2(fftSize);
    for (let i = 0; i < fftSize; i++) {
        let reversed = 0;
        for (let b = 0; b < bits; b++) {
            reversed = (reversed << 1) | ((i >> b) & 1);
        }
        bitRev[i] = reversed;
    }

    const twiddleRe = new Float64Array(fftSize / 2);
    const twiddleIm = new Float64Array(fftSize / 2);
    for (let i = 0; i < fftSize / 2; i++) {
        const angle = (-2 * Math.PI * i) / fftSize;
        twiddleRe[i] = Math.cos(angle);
        twiddleIm[i] = Math.sin(angle);
    }

    const re = new Float64Array(fftSize);
    const im = new Float64Array(fftSize);
    const sgData = [];

    for (let col = 0; col < cols; col++) {
        const frameIdx = col * frameStep;
        const offset = frameIdx * hop;
        if (offset + fftSize > pcm.length) break;

        for (let i = 0; i < fftSize; i++) {
            re[bitRev[i]] = pcm[offset + i] * hannWindow[i];
            im[bitRev[i]] = 0;
        }

        for (let size = 2; size <= fftSize; size *= 2) {
            const halfSize = size / 2;
            const step = fftSize / size;
            for (let i = 0; i < fftSize; i += size) {
                for (let j = 0; j < halfSize; j++) {
                    const idx = j * step;
                    const tRe =
                        twiddleRe[idx] * re[i + j + halfSize] - twiddleIm[idx] * im[i + j + halfSize];
                    const tIm =
                        twiddleRe[idx] * im[i + j + halfSize] + twiddleIm[idx] * re[i + j + halfSize];
                    re[i + j + halfSize] = re[i + j] - tRe;
                    im[i + j + halfSize] = im[i + j] - tIm;
                    re[i + j] += tRe;
                    im[i + j] += tIm;
                }
            }
        }

        const mags = new Array(freqBins);
        for (let bin = 0; bin < freqBins; bin++) {
            const freqLo = Math.pow(bin / freqBins, 2) * numBins;
            const freqHi = Math.pow((bin + 1) / freqBins, 2) * numBins;
            const lo = Math.floor(freqLo);
            const hi = Math.max(lo + 1, Math.floor(freqHi));
            let energy = 0;
            for (let k = lo; k < hi && k < numBins; k++) {
                energy += Math.sqrt(re[k] * re[k] + im[k] * im[k]);
            }
            mags[bin] = Math.round((energy / Math.max(1, hi - lo)) * 100) / 100;
        }
        sgData.push(mags);
    }

    return sgData;
}

async function decodeToBuffer(url) {
    const resp = await fetch(url);
    const ab = await resp.arrayBuffer();
    return decodeToBufferFromAb(ab);
}

function decodeToBufferFromAb(ab) {
    const ctx = new OfflineAudioContext(1, 1, 48000);
    return ctx.decodeAudioData(ab.slice(0));
}

self.onmessage = async (e) => {
    const msg = e.data;
    const { id, type, url, bars, ab } = msg;
    try {
        if (type === 'peaksFromBuffer') {
            const audioBuf = await decodeToBufferFromAb(ab);
            const raw = audioBuf.getChannelData(0);
            const peaks = computePeaksFromChannel(raw, bars);
            self.postMessage({ id, ok: true, peaks });
            return;
        }
        if (type === 'metaFromBuffer') {
            const audioBuf = await decodeToBufferFromAb(ab);
            const raw = audioBuf.getChannelData(0);
            const peaks = computePeaksFromChannel(raw, bars);
            const sgData = computeSpectrogramData(raw);
            self.postMessage({ id, ok: true, peaks, sgData });
            return;
        }
        if (type === 'spectrogramFromBuffer') {
            const audioBuf = await decodeToBufferFromAb(ab);
            const raw = audioBuf.getChannelData(0);
            const sgData = computeSpectrogramData(raw);
            self.postMessage({ id, ok: true, sgData });
            return;
        }
        if (type === 'channelsFromBuffer') {
            const audioBuf = await decodeToBufferFromAb(ab);
            const nCh = audioBuf.numberOfChannels;
            const len = audioBuf.length;
            const sampleRate = audioBuf.sampleRate;
            const channels = [];
            const transfer = [];
            for (let c = 0; c < nCh; c++) {
                const src = audioBuf.getChannelData(c);
                const copy = new Float32Array(len);
                copy.set(src);
                channels.push(copy);
                transfer.push(copy.buffer);
            }
            self.postMessage({ id, ok: true, sampleRate, length: len, channels }, transfer);
            return;
        }
        if (type === 'peaks') {
            const audioBuf = await decodeToBuffer(url);
            const raw = audioBuf.getChannelData(0);
            const peaks = computePeaksFromChannel(raw, bars);
            self.postMessage({ id, ok: true, peaks });
            return;
        }
        if (type === 'meta') {
            const audioBuf = await decodeToBuffer(url);
            const raw = audioBuf.getChannelData(0);
            const peaks = computePeaksFromChannel(raw, bars);
            const sgData = computeSpectrogramData(raw);
            self.postMessage({ id, ok: true, peaks, sgData });
            return;
        }
        if (type === 'spectrogram') {
            const audioBuf = await decodeToBuffer(url);
            const raw = audioBuf.getChannelData(0);
            const sgData = computeSpectrogramData(raw);
            self.postMessage({ id, ok: true, sgData });
            return;
        }
        if (type === 'channels') {
            const audioBuf = await decodeToBuffer(url);
            const nCh = audioBuf.numberOfChannels;
            const len = audioBuf.length;
            const sampleRate = audioBuf.sampleRate;
            const channels = [];
            const transfer = [];
            for (let c = 0; c < nCh; c++) {
                const src = audioBuf.getChannelData(c);
                const copy = new Float32Array(len);
                copy.set(src);
                channels.push(copy);
                transfer.push(copy.buffer);
            }
            self.postMessage({ id, ok: true, sampleRate, length: len, channels }, transfer);
            return;
        }
        self.postMessage({ id, ok: false, error: 'unknown_type' });
    } catch (err) {
        self.postMessage({
            id,
            ok: false,
            error: err && err.message ? err.message : String(err),
        });
    }
};
