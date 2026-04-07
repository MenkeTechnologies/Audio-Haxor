//! File playback: symphonia decode → stereo f32 ring → cpal output with EQ / gain / pan.

use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

const RING_CAP_SAMPLES: usize = 240_000;

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

    /// One stereo frame: EQ → gain → constant-power pan (coeffs computed once per block in caller).
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

pub struct PlaybackShared {
    pub ring: Mutex<VecDeque<f32>>,
    pub ring_cap: usize,
    pub paused: AtomicBool,
    pub stop_decoder: AtomicBool,
    pub seek_sec: Mutex<Option<f64>>,
    pub position_frames: AtomicU64,
    pub duration_sec: AtomicU64,
    #[allow(dead_code)]
    pub src_rate: AtomicU32,
    pub device_rate: AtomicU32,
    pub peak: AtomicU32,
    pub eof: AtomicBool,
    pub dsp: Arc<DspAtomic>,
    dsp_chain: Mutex<DspChain>,
}

impl PlaybackShared {
    pub fn new(dsp: Arc<DspAtomic>, duration_sec: f64, src_rate: u32) -> Arc<Self> {
        Arc::new(Self {
            ring: Mutex::new(VecDeque::with_capacity(RING_CAP_SAMPLES)),
            ring_cap: RING_CAP_SAMPLES,
            paused: AtomicBool::new(false),
            stop_decoder: AtomicBool::new(false),
            seek_sec: Mutex::new(None),
            position_frames: AtomicU64::new(0),
            duration_sec: AtomicU64::new(duration_sec.to_bits()),
            src_rate: AtomicU32::new(src_rate),
            device_rate: AtomicU32::new(0),
            peak: AtomicU32::new(0.0f32.to_bits()),
            eof: AtomicBool::new(false),
            dsp,
            dsp_chain: Mutex::new(DspChain::new()),
        })
    }

    pub fn duration_sec_f64(&self) -> f64 {
        f64::from_bits(self.duration_sec.load(Ordering::Relaxed))
    }

    pub fn position_sec(&self, device_rate: u32) -> f64 {
        let f = self.position_frames.load(Ordering::Relaxed);
        if device_rate == 0 {
            return 0.0;
        }
        f as f64 / f64::from(device_rate)
    }

    pub fn fill_interleaved_f32(&self, data: &mut [f32], ch: usize) {
        let ch = ch.max(1);
        let frames = data.len() / ch;
        let device_rate = self.device_rate.load(Ordering::Relaxed).max(1);
        let sr = device_rate as f64;
        let (g, pan, low, mid, high) = self.dsp.snapshot(sr);
        let cl = coeffs_lowshelf(200.0, low, sr);
        let cm = coeffs_peaking(1000.0, 1.0, mid, sr);
        let c_hi = coeffs_highshelf(8000.0, high, sr);
        let mut chain = self.dsp_chain.lock().unwrap();

        let mut pk = 0.0f32;
        for f in 0..frames {
            let (l, r) = {
                let mut ring = self.ring.lock().unwrap();
                let l = ring.pop_front().unwrap_or(0.0);
                let r = ring.pop_front().unwrap_or(0.0);
                (l as f64, r as f64)
            };
            let (lo, ro) = chain.process_stereo_with_coeffs(l, r, &cl, &cm, &c_hi, g, pan);
            let lo = lo as f32;
            let ro = ro as f32;
            pk = pk.max(lo.abs()).max(ro.abs());
            match ch {
                1 => data[f] = (lo + ro) * 0.5,
                2 => {
                    data[f * 2] = lo;
                    data[f * 2 + 1] = ro;
                }
                n => {
                    for c in 0..n {
                        data[f * n + c] = if c % 2 == 0 { lo } else { ro };
                    }
                }
            }
        }
        self.position_frames.fetch_add(frames as u64, Ordering::Relaxed);
        self.peak.store(pk.to_bits(), Ordering::Relaxed);
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

/// First decoded audio sample rate for `track_id` (advances `format`). MP3/LAME headers in
/// `codec_params` can disagree with the first decoded `AudioSpec` — probing here aligns
/// `playback_load` JSON, device `pick_output_config`, and initial `decode_loop` ratio before the
/// output stream opens.
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

#[inline]
fn ratio_from_rates(src_rate: u32, device_rate: u32) -> f64 {
    if src_rate == device_rate {
        1.0
    } else {
        f64::from(src_rate) / f64::from(device_rate)
    }
}

fn decode_loop(path: String, shared: Arc<PlaybackShared>, device_rate: u32, track_id: u32) -> Result<(), String> {
    let path_buf = Path::new(&path).to_path_buf();
    let mut format = open_format(&path_buf)?;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.id == track_id)
        .or_else(|| format.default_track())
        .ok_or_else(|| "no track".to_string())?;
    let tid = track.id;
    // Prefer `playback_load` probe (atomic) so initial ratio matches `pick_output_config` before
    // the first packet; fall back to container metadata from this fresh open.
    let probed = shared.src_rate.load(Ordering::Relaxed);
    let mut src_rate = if probed > 0 {
        probed
    } else {
        track.codec_params.sample_rate.ok_or_else(|| "unknown sample rate".to_string())?
    };
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| e.to_string())?;

    // `codec_params.sample_rate` can disagree with the decoded stream (notably some MP3 probes).
    // Wrong `src_rate` here makes `ratio` wrong → wrong resample speed vs device clock.
    let mut ratio = ratio_from_rates(src_rate, device_rate);
    let mut staging: Vec<f32> = Vec::new();
    // Fractional index in **stereo frames** from the start of `staging` (linear interp).
    let mut read_pos: f64 = 0.0;

    while !shared.stop_decoder.load(Ordering::Relaxed) {
        if shared.paused.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(8));
            continue;
        }

        if let Some(sec) = shared.seek_sec.lock().unwrap().take() {
            staging.clear();
            read_pos = 0.0;
            let sec = sec.max(0.0);
            let ti = sec.floor() as u64;
            let frac_t = sec - ti as f64;
            let _ = format.seek(SeekMode::Accurate, SeekTo::Time {
                time: Time::new(ti, frac_t),
                track_id: Some(tid),
            });
            let tr = format
                .tracks()
                .iter()
                .find(|t| t.id == tid)
                .ok_or_else(|| "track missing".to_string())?;
            decoder = symphonia::default::get_codecs()
                .make(&tr.codec_params, &DecoderOptions::default())
                .map_err(|e| e.to_string())?;
            shared.eof.store(false, Ordering::Relaxed);
        }

        {
            let ring = shared.ring.lock().unwrap();
            if ring.len() > shared.ring_cap * 8 / 10 {
                std::thread::sleep(std::time::Duration::from_millis(2));
                continue;
            }
        }

        let mut stereo_frames = staging.len() / 2;
        // Need enough staging for interpolation at read_pos + one step.
        while read_pos + ratio + 2.0 > stereo_frames as f64 && !shared.stop_decoder.load(Ordering::Relaxed) {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(_) => {
                    shared.eof.store(true, Ordering::Relaxed);
                    break;
                }
            };
            if packet.track_id() != tid {
                continue;
            }
            let decoded = match decoder.decode(&packet) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let spec = *decoded.spec();
            let rate = spec.rate;
            if rate > 0 && rate != src_rate {
                src_rate = rate;
                shared.src_rate.store(src_rate, Ordering::Relaxed);
                ratio = ratio_from_rates(src_rate, device_rate);
                staging.clear();
                read_pos = 0.0;
            }
            let dur = decoded.capacity() as u64;
            let mut sample_buf = SampleBuffer::<f32>::new(dur, spec);
            sample_buf.copy_interleaved_ref(decoded);
            let raw = sample_buf.samples();
            let ch_n = spec.channels.count().max(1);
            let frames = raw.len() / ch_n;
            for i in 0..frames {
                let l = raw[i * ch_n];
                let r = if ch_n > 1 { raw[i * ch_n + 1] } else { l };
                staging.push(l);
                staging.push(r);
            }
            stereo_frames = staging.len() / 2;
        }

        stereo_frames = staging.len() / 2;
        if stereo_frames < 2 {
            if shared.eof.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            continue;
        }

        let base = read_pos.floor() as usize;
        if base + 1 >= stereo_frames {
            continue;
        }
        let t = read_pos - base as f64;
        let i0 = base * 2;
        let i1 = (base + 1) * 2;
        if i1 + 1 >= staging.len() {
            continue;
        }
        let l = (staging[i0] as f64 + t * (staging[i1] as f64 - staging[i0] as f64)) as f32;
        let r = (staging[i0 + 1] as f64 + t * (staging[i1 + 1] as f64 - staging[i0 + 1] as f64)) as f32;
        {
            let mut ring = shared.ring.lock().unwrap();
            if ring.len() + 2 <= shared.ring_cap {
                ring.push_back(l);
                ring.push_back(r);
            }
        }
        read_pos += ratio;

        // Normalize read_pos into [0, 1) frame offset by dropping consumed stereo frames from the
        // front of staging (one frame = two interleaved samples). Requires ≥2 frames before drop
        // so one frame remains for the next decode pass when only two frames were buffered.
        while read_pos >= 1.0 && staging.len() >= 4 {
            staging.drain(0..2);
            read_pos -= 1.0;
        }
    }
    Ok(())
}

static SESSION: Mutex<Option<PlaybackSession>> = Mutex::new(None);

struct PlaybackSession {
    path: String,
    shared: Arc<PlaybackShared>,
    track_id: u32,
    join: Option<JoinHandle<()>>,
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
        join: None,
    });

    Ok(serde_json::json!({
        "ok": true,
        "duration_sec": duration_sec,
        "sample_rate_hz": src_rate,
        "track_id": track_id,
    }))
}

/// Called from the audio thread after the output device rate is known.
pub fn begin_playback(device_rate: u32) -> Result<(), String> {
    let mut g = SESSION.lock().map_err(|_| "playback mutex poisoned")?;
    let sess = g.as_mut().ok_or_else(|| "playback_load first".to_string())?;
    if sess.join.is_some() {
        return Err("playback already running".to_string());
    }
    sess.shared.device_rate.store(device_rate, Ordering::Relaxed);
    sess.shared.stop_decoder.store(false, Ordering::Relaxed);
    sess.shared.paused.store(false, Ordering::Relaxed);
    sess.shared.eof.store(false, Ordering::Relaxed);
    sess.shared.position_frames.store(0, Ordering::Relaxed);
    {
        let mut ring = sess.shared.ring.lock().unwrap();
        ring.clear();
    }

    let path = sess.path.clone();
    let shared = sess.shared.clone();
    let tid = sess.track_id;
    let join = thread::spawn(move || {
        if let Err(e) = decode_loop(path, shared.clone(), device_rate, tid) {
            let _ = writeln!(std::io::stderr(), "audio-engine decode: {e}");
        }
    });
    sess.join = Some(join);

    Ok(())
}

/// Stop decoder thread when the output stream is dropped or stopped.
pub fn stop_playback_thread() {
    let Ok(mut g) = SESSION.lock() else {
        return;
    };
    let Some(ref mut sess) = *g else {
        return;
    };
    sess.shared.stop_decoder.store(true, Ordering::Relaxed);
    if let Some(j) = sess.join.take() {
        let _ = j.join();
    }
}

pub fn playback_stop() -> Result<serde_json::Value, String> {
    let mut g = SESSION.lock().map_err(|_| "playback mutex poisoned")?;
    if let Some(mut sess) = g.take() {
        sess.shared.stop_decoder.store(true, Ordering::Relaxed);
        if let Some(j) = sess.join.take() {
            let _ = j.join();
        }
    }
    Ok(serde_json::json!({ "ok": true }))
}

pub fn playback_pause(set: bool) -> Result<serde_json::Value, String> {
    let g = SESSION.lock().map_err(|_| "playback mutex poisoned")?;
    let sess = g.as_ref().ok_or_else(|| "no session".to_string())?;
    sess.shared.paused.store(set, Ordering::Relaxed);
    Ok(serde_json::json!({ "ok": true, "paused": set }))
}

pub fn playback_seek(position_sec: f64) -> Result<serde_json::Value, String> {
    let g = SESSION.lock().map_err(|_| "playback mutex poisoned")?;
    let sess = g.as_ref().ok_or_else(|| "no session".to_string())?;
    *sess.shared.seek_sec.lock().unwrap() = Some(position_sec.max(0.0));
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

pub fn playback_status() -> Result<serde_json::Value, String> {
    let g = SESSION.lock().map_err(|_| "playback mutex poisoned")?;
    let Some(sess) = g.as_ref() else {
        return Ok(serde_json::json!({
            "ok": true,
            "loaded": false,
        }));
    };
    let dr = sess.shared.device_rate.load(Ordering::Relaxed);
    let pos = sess.shared.position_sec(dr);
    let dur = sess.shared.duration_sec_f64();
    let pk = f32::from_bits(sess.shared.peak.load(Ordering::Relaxed));
    Ok(serde_json::json!({
        "ok": true,
        "loaded": true,
        "position_sec": pos,
        "duration_sec": dur,
        "peak": pk,
        "paused": sess.shared.paused.load(Ordering::Relaxed),
        "eof": sess.shared.eof.load(Ordering::Relaxed),
        "sample_rate_hz": dr,
        "src_rate_hz": sess.shared.src_rate.load(Ordering::Relaxed),
    }))
}

pub fn current_playback_shared() -> Option<Arc<PlaybackShared>> {
    SESSION.lock().ok()?.as_ref().map(|s| s.shared.clone())
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
