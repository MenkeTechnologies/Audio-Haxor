//! Library file playback via **[rodio](https://github.com/RustAudio/rodio)** (`Decoder` → `Source` →
//! `Player` / OS mixer) with the same **3-band EQ**, **gain**, and **constant-power pan** as the
//! previous symphonia→ring→cpal path. Device selection and buffer size use **cpal** configs passed
//! through `rodio::stream::DeviceSinkBuilder` (same stack as rodio’s examples).

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::num::NonZero;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rodio::buffer::SamplesBuffer;
use rodio::source::Source;
use rodio::stream::DeviceSinkBuilder;
use rodio::{Decoder, Player};
use rodio::{ChannelCount, SampleRate};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

// ── Biquad (direct form I, coeffs normalized by a0) ─────────────────────────

#[derive(Clone, Copy, Debug)]
struct BiquadCoeffs {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct Biquad {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    fn process(&mut self, x: f64, c: &BiquadCoeffs) -> f64 {
        let y = c.b0 * x + c.b1 * self.x1 + c.b2 * self.x2 - c.a1 * self.y1 - c.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

fn db_to_linear(db: f64) -> f64 {
    10f64.powf(db / 20.0)
}

/// RBJ lowshelf (musicdsp cookbook).
fn coeffs_lowshelf(freq_hz: f64, gain_db: f64, sr: f64) -> BiquadCoeffs {
    let a = db_to_linear(gain_db);
    let w0 = 2.0 * std::f64::consts::PI * freq_hz / sr;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / 2.0 * ((2.0 * a).sqrt());
    let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
    let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
    let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);
    let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
    let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
    let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;
    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

fn coeffs_peaking(freq_hz: f64, q: f64, gain_db: f64, sr: f64) -> BiquadCoeffs {
    let a = 10f64.powf(gain_db / 40.0);
    let w0 = 2.0 * std::f64::consts::PI * freq_hz / sr;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);
    let b0 = 1.0 + alpha * a;
    let b1 = -2.0 * cos_w0;
    let b2 = 1.0 - alpha * a;
    let a0 = 1.0 + alpha / a;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha / a;
    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

fn coeffs_highshelf(freq_hz: f64, gain_db: f64, sr: f64) -> BiquadCoeffs {
    let a = db_to_linear(gain_db);
    let w0 = 2.0 * std::f64::consts::PI * freq_hz / sr;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / 2.0 * ((2.0 * a).sqrt());
    let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
    let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
    let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);
    let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
    let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
    let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;
    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

pub struct DspAtomic {
    pub gain: AtomicU32,
    pub pan: AtomicU32,
    pub eq_low_db: AtomicU32,
    pub eq_mid_db: AtomicU32,
    pub eq_high_db: AtomicU32,
}

impl DspAtomic {
    pub fn new() -> Self {
        Self {
            gain: AtomicU32::new(1.0f32.to_bits()),
            pan: AtomicU32::new(0.0f32.to_bits()),
            eq_low_db: AtomicU32::new(0.0f32.to_bits()),
            eq_mid_db: AtomicU32::new(0.0f32.to_bits()),
            eq_high_db: AtomicU32::new(0.0f32.to_bits()),
        }
    }

    fn snapshot(&self, _sr: f64) -> (f64, f64, f64, f64, f64) {
        (
            f32::from_bits(self.gain.load(Ordering::Relaxed)).clamp(0.0, 4.0) as f64,
            f32::from_bits(self.pan.load(Ordering::Relaxed)).clamp(-1.0, 1.0) as f64,
            f32::from_bits(self.eq_low_db.load(Ordering::Relaxed)) as f64,
            f32::from_bits(self.eq_mid_db.load(Ordering::Relaxed)) as f64,
            f32::from_bits(self.eq_high_db.load(Ordering::Relaxed)) as f64,
        )
    }
}

impl Default for DspAtomic {
    fn default() -> Self {
        Self::new()
    }
}

struct DspChain {
    low_l: Biquad,
    low_r: Biquad,
    mid_l: Biquad,
    mid_r: Biquad,
    high_l: Biquad,
    high_r: Biquad,
}

impl DspChain {
    fn new() -> Self {
        Self {
            low_l: Biquad::default(),
            low_r: Biquad::default(),
            mid_l: Biquad::default(),
            mid_r: Biquad::default(),
            high_l: Biquad::default(),
            high_r: Biquad::default(),
        }
    }

    fn process_stereo_with_coeffs(
        &mut self,
        l_in: f64,
        r_in: f64,
        cl: &BiquadCoeffs,
        cm: &BiquadCoeffs,
        ch: &BiquadCoeffs,
        g: f64,
        pan: f64,
    ) -> (f64, f64) {
        let mut l = self.low_l.process(l_in, cl);
        let mut r = self.low_r.process(r_in, cl);
        l = self.mid_l.process(l, cm);
        r = self.mid_r.process(r, cm);
        l = self.high_l.process(l, ch);
        r = self.high_r.process(r, ch);
        l *= g;
        r *= g;
        let x = (pan + 1.0) * std::f64::consts::PI / 4.0;
        (l * x.cos(), r * x.sin())
    }
}

/// Shared DSP + metadata for IPC (`playback_status`, etc.).
pub struct PlaybackShared {
    pub peak: AtomicU32,
    pub duration_sec: AtomicU64,
    #[allow(dead_code)]
    pub src_rate: AtomicU32,
    pub device_rate: AtomicU32,
    pub eof: AtomicBool,
    pub dsp: Arc<DspAtomic>,
}

impl PlaybackShared {
    pub fn new(dsp: Arc<DspAtomic>, duration_sec: f64, src_rate: u32) -> Arc<Self> {
        Arc::new(Self {
            peak: AtomicU32::new(0.0f32.to_bits()),
            duration_sec: AtomicU64::new(duration_sec.to_bits()),
            src_rate: AtomicU32::new(src_rate),
            device_rate: AtomicU32::new(0),
            eof: AtomicBool::new(false),
            dsp,
        })
    }

    pub fn duration_sec_f64(&self) -> f64 {
        f64::from_bits(self.duration_sec.load(Ordering::Relaxed))
    }
}

fn open_format(path: &Path) -> Result<Box<dyn FormatReader>, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| e.to_string())?;
    Ok(probed.format)
}

fn probe_first_decoded_sample_rate(format: &mut Box<dyn FormatReader>, track_id: u32) -> Option<u32> {
    let track = format
        .tracks()
        .iter()
        .find(|t| t.id == track_id)
        .or_else(|| format.default_track())?;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .ok()?;
    let tid = track.id;
    loop {
        let packet = format.next_packet().ok()?;
        if packet.track_id() != tid {
            continue;
        }
        let decoded = decoder.decode(&packet).ok()?;
        let rate = decoded.spec().rate;
        if rate > 0 {
            return Some(rate);
        }
    }
}

static SESSION: Mutex<Option<PlaybackSession>> = Mutex::new(None);
static RODIO_PLAYER: Mutex<Option<Player>> = Mutex::new(None);

struct PlaybackSession {
    path: String,
    shared: Arc<PlaybackShared>,
    #[allow(dead_code)]
    track_id: u32,
    /// When true, [`Self::reverse_pcm`] is played instead of streaming from disk.
    reverse_mode: bool,
    /// Stereo interleaved f32 at file decode rate (reversed in time).
    reverse_pcm: Option<Arc<Vec<f32>>>,
    reverse_pcm_rate: u32,
}

/// Wraps a [`Source`] and applies EQ / gain / pan for each stereo frame (interleaved samples).
struct DspStereoSource<S: Source<Item = f32>> {
    inner: S,
    shared: Arc<PlaybackShared>,
    chain: DspChain,
    out: [f32; 2],
    out_i: u8,
    ch: ChannelCount,
}

impl<S: Source<Item = f32>> DspStereoSource<S> {
    fn new(inner: S, shared: Arc<PlaybackShared>) -> Self {
        let ch = inner.channels();
        Self {
            inner,
            shared,
            chain: DspChain::new(),
            out: [0.0, 0.0],
            out_i: 2,
            ch,
        }
    }

    fn refill(&mut self) -> bool {
        let l = match self.inner.next() {
            Some(s) => s,
            None => return false,
        };
        let n = self.ch.get() as usize;
        let r = if n >= 2 {
            self.inner.next().unwrap_or(l)
        } else {
            l
        };
        let sr = f64::from(self.inner.sample_rate().get());
        let (g, pan, low, mid, high) = self.shared.dsp.snapshot(sr);
        let cl = coeffs_lowshelf(200.0, low, sr);
        let cm = coeffs_peaking(1000.0, 1.0, mid, sr);
        let c_hi = coeffs_highshelf(8000.0, high, sr);
        let (lo, ro) = self
            .chain
            .process_stereo_with_coeffs(f64::from(l), f64::from(r), &cl, &cm, &c_hi, g, pan);
        let lo = lo as f32;
        let ro = ro as f32;
        let pk = f32::from_bits(self.shared.peak.load(Ordering::Relaxed))
            .max(lo.abs())
            .max(ro.abs());
        self.shared.peak.store(pk.to_bits(), Ordering::Relaxed);
        self.out = [lo, ro];
        self.out_i = 0;
        true
    }
}

impl<S: Source<Item = f32>> Iterator for DspStereoSource<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        loop {
            if usize::from(self.out_i) < 2 {
                let s = self.out[usize::from(self.out_i)];
                self.out_i += 1;
                return Some(s);
            }
            if !self.refill() {
                return None;
            }
        }
    }
}

impl<S: Source<Item = f32>> Source for DspStereoSource<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

pub fn playback_load(path: String) -> Result<serde_json::Value, String> {
    let p = Path::new(&path);
    if !p.is_file() {
        return Err(format!("not a file: {path}"));
    }
    let mut format = open_format(p)?;
    let (track_id, n_frames, codec_sr) = {
        let track = format.default_track().ok_or_else(|| "no default track".to_string())?;
        (
            track.id,
            track.codec_params.n_frames,
            track
                .codec_params
                .sample_rate
                .ok_or_else(|| "unknown sample rate".to_string())?,
        )
    };
    let mut src_rate = codec_sr;
    if let Some(rate) = probe_first_decoded_sample_rate(&mut format, track_id) {
        src_rate = rate;
    }
    let duration_sec = n_frames
        .map(|n| n as f64 / f64::from(src_rate))
        .unwrap_or(0.0);

    let dsp = Arc::new(DspAtomic::new());
    let shared = PlaybackShared::new(dsp, duration_sec, src_rate);

    let mut g = SESSION.lock().map_err(|_| "playback mutex poisoned")?;
    *g = Some(PlaybackSession {
        path,
        shared: shared.clone(),
        track_id,
        reverse_mode: false,
        reverse_pcm: None,
        reverse_pcm_rate: 0,
    });

    Ok(serde_json::json!({
        "ok": true,
        "duration_sec": duration_sec,
        "sample_rate_hz": src_rate,
        "track_id": track_id,
    }))
}

/// Open rodio’s OS sink on `device` with `supported` (sample format / rate / channels) and optional
/// hardware buffer size. Caller keeps the returned handle alive for playback.
pub fn open_rodio_output(
    device: cpal::Device,
    supported: cpal::SupportedStreamConfig,
    buffer_frames: Option<u32>,
) -> Result<rodio::stream::MixerDeviceSink, String> {
    let mut b = DeviceSinkBuilder::from_device(device).map_err(|e| format!("rodio from_device: {e}"))?;
    b = b.with_supported_config(&supported);
    if let Some(frames) = buffer_frames {
        if frames > 0 {
            b = b.with_buffer_size(cpal::BufferSize::Fixed(frames));
        }
    }
    b.open_stream().map_err(|e| format!("rodio open_stream: {e}"))
}

/// Decode `path`, apply DSP, append to `player`, register [`Player`] for pause/seek/status.
pub fn start_rodio_playback(path: &str, player: Player, shared: Arc<PlaybackShared>) -> Result<(), String> {
    stop_rodio_player();
    let reverse_mode = {
        let sess = SESSION.lock().map_err(|_| "playback mutex poisoned")?;
        let Some(ref session) = *sess else {
            return Err("no session".to_string());
        };
        if session.path != path {
            return Err("session path mismatch".to_string());
        }
        session.reverse_mode
    };

    if reverse_mode {
        let (pcm, rate) = {
            let sess = SESSION.lock().map_err(|_| "playback mutex poisoned")?;
            let Some(ref session) = *sess else {
                return Err("no session".to_string());
            };
            let pcm = session
                .reverse_pcm
                .as_ref()
                .ok_or_else(|| "reverse pcm missing".to_string())?
                .clone();
            let rate = session.reverse_pcm_rate;
            if rate == 0 {
                return Err("reverse pcm rate missing".to_string());
            }
            (pcm, rate)
        };
        let ch = NonZero::new(2u16).ok_or_else(|| "bad channel count".to_string())?;
        let sr = NonZero::new(rate).ok_or_else(|| "bad sample rate".to_string())?;
        let buf = SamplesBuffer::new(ch, sr, pcm.to_vec());
        let src = DspStereoSource::new(buf, shared);
        player.append(src);
    } else {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let decoder = Decoder::try_from(BufReader::new(file)).map_err(|e| format!("rodio decode open: {e}"))?;
        let src = DspStereoSource::new(decoder, shared);
        player.append(src);
    }
    *RODIO_PLAYER.lock().map_err(|_| "rodio player mutex poisoned")? = Some(player);
    Ok(())
}

fn stop_rodio_player() {
    if let Ok(mut g) = RODIO_PLAYER.lock() {
        if let Some(p) = g.take() {
            p.stop();
        }
    }
}

/// Called when replacing cpal/rodio output so the previous [`Player`] does not outlive the mixer.
pub fn stop_playback_thread() {
    stop_rodio_player();
}

pub fn playback_stop() -> Result<serde_json::Value, String> {
    stop_rodio_player();
    let mut g = SESSION.lock().map_err(|_| "playback mutex poisoned")?;
    *g = None;
    Ok(serde_json::json!({ "ok": true }))
}

pub fn playback_pause(set: bool) -> Result<serde_json::Value, String> {
    let g = RODIO_PLAYER.lock().map_err(|_| "rodio player mutex poisoned")?;
    let Some(p) = g.as_ref() else {
        return Err("no active rodio player".to_string());
    };
    if set {
        p.pause();
    } else {
        p.play();
    }
    Ok(serde_json::json!({ "ok": true, "paused": set }))
}

pub fn playback_seek(position_sec: f64) -> Result<serde_json::Value, String> {
    let seek_in_source = {
        let sg = SESSION.lock().map_err(|_| "playback mutex poisoned")?;
        let Some(s) = sg.as_ref() else {
            return Err("no session".to_string());
        };
        let dur = s.shared.duration_sec_f64();
        let t = position_sec.max(0.0).min(dur);
        if s.reverse_mode {
            (dur - t).max(0.0)
        } else {
            t
        }
    };
    let g = RODIO_PLAYER.lock().map_err(|_| "rodio player mutex poisoned")?;
    let Some(p) = g.as_ref() else {
        return Err("no active rodio player".to_string());
    };
    p.try_seek(Duration::from_secs_f64(seek_in_source))
        .map_err(|e| format!("seek: {e:?}"))?;
    Ok(serde_json::json!({ "ok": true }))
}

pub fn playback_set_dsp(gain: f32, pan: f32, low: f32, mid: f32, high: f32) -> Result<serde_json::Value, String> {
    let g = SESSION.lock().map_err(|_| "playback mutex poisoned")?;
    let sess = g.as_ref().ok_or_else(|| "no session".to_string())?;
    let d = &sess.shared.dsp;
    d.gain.store(gain.to_bits(), Ordering::Relaxed);
    d.pan.store(pan.to_bits(), Ordering::Relaxed);
    d.eq_low_db.store(low.to_bits(), Ordering::Relaxed);
    d.eq_mid_db.store(mid.to_bits(), Ordering::Relaxed);
    d.eq_high_db.store(high.to_bits(), Ordering::Relaxed);
    Ok(serde_json::json!({ "ok": true }))
}

/// Matches now-playing speed UI (0.25x–2x). Rodio changes pitch with speed (same as `<audio>.playbackRate`).
pub fn playback_set_speed(speed: f32) -> Result<serde_json::Value, String> {
    let s = speed.clamp(0.25, 2.0);
    let g = RODIO_PLAYER.lock().map_err(|_| "rodio player mutex poisoned")?;
    let Some(p) = g.as_ref() else {
        return Err("no active rodio player".to_string());
    };
    p.set_speed(s);
    Ok(serde_json::json!({ "ok": true, "speed": s }))
}

fn decode_file_to_interleaved_f32(path: &str) -> Result<(Vec<f32>, u32), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let decoder = Decoder::try_from(BufReader::new(file)).map_err(|e| format!("rodio decode open: {e}"))?;
    let ch = decoder.channels().get();
    let rate = decoder.sample_rate().get();
    let mut out = Vec::new();
    match ch {
        1 => {
            for s in decoder {
                out.push(s);
                out.push(s);
            }
        }
        2 => {
            out.extend(decoder);
        }
        _ => return Err(format!("unsupported channel count {ch}")),
    }
    Ok((out, rate))
}

fn reverse_stereo_interleaved(v: &mut [f32]) {
    let frames = v.len() / 2;
    for i in 0..frames / 2 {
        let a = i * 2;
        let b = (frames - 1 - i) * 2;
        v.swap(a, b);
        v.swap(a + 1, b + 1);
    }
}

/// Prepare reversed PCM for [`start_rodio_playback`] (full-file decode; large files use significant RAM).
pub fn playback_set_reverse(enabled: bool) -> Result<serde_json::Value, String> {
    let mut g = SESSION.lock().map_err(|_| "playback mutex poisoned")?;
    let Some(sess) = g.as_mut() else {
        return Err("no session".to_string());
    };
    if enabled == sess.reverse_mode {
        return Ok(serde_json::json!({ "ok": true, "reverse": enabled }));
    }
    sess.reverse_mode = enabled;
    if enabled {
        let (mut pcm, rate) = decode_file_to_interleaved_f32(&sess.path)?;
        reverse_stereo_interleaved(&mut pcm);
        sess.reverse_pcm = Some(Arc::new(pcm));
        sess.reverse_pcm_rate = rate;
    } else {
        sess.reverse_pcm = None;
        sess.reverse_pcm_rate = 0;
    }
    Ok(serde_json::json!({ "ok": true, "reverse": enabled }))
}

pub fn playback_status() -> Result<serde_json::Value, String> {
    let g = SESSION.lock().map_err(|_| "playback mutex poisoned")?;
    let Some(sess) = g.as_ref() else {
        return Ok(serde_json::json!({
            "ok": true,
            "loaded": false,
        }));
    };
    let rev = sess.reverse_mode;
    let dr = sess.shared.device_rate.load(Ordering::Relaxed);
    let dur = sess.shared.duration_sec_f64();
    let pk = f32::from_bits(sess.shared.peak.load(Ordering::Relaxed));

    let pg = RODIO_PLAYER.lock().map_err(|_| "rodio player mutex poisoned")?;
    let Some(p) = pg.as_ref() else {
        return Ok(serde_json::json!({
            "ok": true,
            "loaded": true,
            "position_sec": 0.0,
            "duration_sec": dur,
            "peak": pk,
            "paused": false,
            "eof": false,
            "reverse": rev,
            "sample_rate_hz": dr,
            "src_rate_hz": sess.shared.src_rate.load(Ordering::Relaxed),
        }));
    };

    let pos_src = p.get_pos().as_secs_f64();
    let pos = if rev {
        (dur - pos_src).max(0.0)
    } else {
        pos_src
    };
    let eof = p.empty();
    sess.shared.eof.store(eof, Ordering::Relaxed);

    Ok(serde_json::json!({
        "ok": true,
        "loaded": true,
        "position_sec": pos,
        "duration_sec": dur,
        "peak": pk,
        "paused": p.is_paused(),
        "eof": eof,
        "reverse": rev,
        "sample_rate_hz": dr,
        "src_rate_hz": sess.shared.src_rate.load(Ordering::Relaxed),
    }))
}

pub fn current_playback_shared() -> Option<Arc<PlaybackShared>> {
    SESSION.lock().ok()?.as_ref().map(|s| {
        let sh = s.shared.clone();
        sh
    })
}

pub fn playback_session_path() -> Result<String, String> {
    let g = SESSION.lock().map_err(|_| "playback mutex poisoned")?;
    let s = g.as_ref().ok_or_else(|| "no session".to_string())?;
    Ok(s.path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dsp_gain_halves_stereo() {
        let mut c = DspChain::new();
        let sr = 48_000.0;
        let cl = coeffs_lowshelf(200.0, 0.0, sr);
        let cm = coeffs_peaking(1000.0, 1.0, 0.0, sr);
        let c_hi = coeffs_highshelf(8000.0, 0.0, sr);
        let (l, r) = c.process_stereo_with_coeffs(1.0, 1.0, &cl, &cm, &c_hi, 0.5, 0.0);
        let exp = 0.5 * std::f64::consts::FRAC_1_SQRT_2;
        assert!((l - exp).abs() < 0.02, "l={l}");
        assert!((r - exp).abs() < 0.02, "r={r}");
    }
}
