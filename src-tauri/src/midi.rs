//! MIDI file parser — extracts metadata from Standard MIDI Files (.mid/.midi).
//!
//! Parses the MThd header and MTrk track chunks to extract:
//! - Format type (0=single track, 1=multi-track, 2=independent tracks)
//! - Track count
//! - Tempo (BPM from meta event 0x51)
//! - Time signature (from meta event 0x58)
//! - Note count (total note-on events)
//! - Duration (in seconds, computed from tempo + tick count)
//! - Key signature (from meta event 0x59)
//! - Track names (from meta event 0x03)

use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Default)]
pub struct MidiInfo {
    pub format: u16,
    #[serde(rename = "trackCount")]
    pub track_count: u16,
    pub ppqn: u16,
    pub tempo: f64,
    #[serde(rename = "timeSignature")]
    pub time_signature: String,
    #[serde(rename = "keySignature")]
    pub key_signature: String,
    #[serde(rename = "noteCount")]
    pub note_count: u32,
    pub duration: f64,
    #[serde(rename = "trackNames")]
    pub track_names: Vec<String>,
    #[serde(rename = "channelsUsed")]
    pub channels_used: u16,
}

/// Parse a MIDI file and return metadata.
pub fn parse_midi(path: &Path) -> Option<MidiInfo> {
    let data = std::fs::read(path).ok()?;
    if data.len() < 14 {
        return None;
    }

    // MThd header: "MThd" + 4-byte length + format + tracks + division
    if &data[0..4] != b"MThd" {
        return None;
    }
    let header_len = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
    if data.len() < 8 + header_len {
        return None;
    }

    let format = u16::from_be_bytes([data[8], data[9]]);
    let track_count = u16::from_be_bytes([data[10], data[11]]);
    let division = u16::from_be_bytes([data[12], data[13]]);

    // Division: if bit 15 is 0, it's ticks per quarter note (PPQN)
    let ppqn = if division & 0x8000 == 0 {
        division
    } else {
        480
    }; // default 480 for SMPTE

    let mut info = MidiInfo {
        format,
        track_count,
        ppqn,
        tempo: 120.0, // default BPM
        time_signature: "4/4".into(),
        ..Default::default()
    };

    let mut channel_mask = 0u16;
    let mut total_ticks = 0u32;
    let mut tempo_us = 500_000u32; // default: 120 BPM = 500000 µs/beat

    // Parse track chunks
    let mut pos = 8 + header_len;
    for _ in 0..track_count {
        if pos + 8 > data.len() {
            break;
        }
        if &data[pos..pos + 4] != b"MTrk" {
            break;
        }
        let track_len =
            u32::from_be_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                as usize;
        let track_end = (pos + 8 + track_len).min(data.len());
        let mut tp = pos + 8;
        let mut track_ticks = 0u32;
        let mut running_status = 0u8;

        while tp < track_end {
            // Read delta time (variable-length)
            let (delta, bytes_read) = read_var_len(&data, tp);
            tp += bytes_read;
            track_ticks += delta;

            if tp >= track_end {
                break;
            }

            let status = data[tp];

            if status == 0xFF {
                // Meta event
                tp += 1;
                if tp >= track_end {
                    break;
                }
                let meta_type = data[tp];
                tp += 1;
                let (meta_len, vl_bytes) = read_var_len(&data, tp);
                tp += vl_bytes;
                let meta_end = (tp + meta_len as usize).min(track_end);

                match meta_type {
                    0x03 => {
                        // Track name
                        if let Ok(name) = std::str::from_utf8(&data[tp..meta_end]) {
                            let name = name.trim();
                            if !name.is_empty() {
                                info.track_names.push(name.to_string());
                            }
                        }
                    }
                    0x51 => {
                        // Tempo: 3 bytes, microseconds per quarter note
                        if meta_end - tp >= 3 {
                            tempo_us = ((data[tp] as u32) << 16)
                                | ((data[tp + 1] as u32) << 8)
                                | (data[tp + 2] as u32);
                            if tempo_us > 0 {
                                info.tempo = 60_000_000.0 / tempo_us as f64;
                            }
                        }
                    }
                    0x58 => {
                        // Time signature: nn/2^dd
                        if meta_end - tp >= 2 {
                            let nn = data[tp];
                            let dd = data[tp + 1];
                            let denom = 1u32 << dd;
                            info.time_signature = format!("{nn}/{denom}");
                        }
                    }
                    0x59 => {
                        // Key signature: sf mi (sf=sharps/flats, mi=0 major/1 minor)
                        if meta_end - tp >= 2 {
                            let sf = data[tp] as i8;
                            let mi = data[tp + 1];
                            let key_names_major = [
                                "Cb", "Gb", "Db", "Ab", "Eb", "Bb", "F", "C", "G", "D", "A", "E",
                                "B", "F#", "C#",
                            ];
                            let key_names_minor = [
                                "Ab", "Eb", "Bb", "F", "C", "G", "D", "A", "E", "B", "F#", "C#",
                                "G#", "D#", "A#",
                            ];
                            let idx = (sf + 7) as usize;
                            if idx < 15 {
                                let name = if mi == 0 {
                                    key_names_major[idx]
                                } else {
                                    key_names_minor[idx]
                                };
                                let mode = if mi == 0 { "major" } else { "minor" };
                                info.key_signature = format!("{name} {mode}");
                            }
                        }
                    }
                    _ => {}
                }
                tp = meta_end;
            } else if status == 0xF0 || status == 0xF7 {
                // SysEx event
                tp += 1;
                let (sysex_len, vl_bytes) = read_var_len(&data, tp);
                tp += vl_bytes + sysex_len as usize;
            } else if status & 0x80 != 0 {
                // Channel event
                running_status = status;
                tp += 1;
                let msg = status & 0xF0;
                let channel = status & 0x0F;
                channel_mask |= 1 << channel;
                match msg {
                    0x80 | 0xA0 | 0xB0 | 0xE0 => {
                        tp += 2;
                    } // 2 data bytes
                    0x90 => {
                        // Note on
                        if tp + 1 < track_end && data[tp + 1] > 0 {
                            info.note_count += 1;
                        }
                        tp += 2;
                    }
                    0xC0 | 0xD0 => {
                        tp += 1;
                    } // 1 data byte
                    _ => {
                        tp += 2;
                    }
                }
            } else {
                // Running status
                let msg = running_status & 0xF0;
                let channel = running_status & 0x0F;
                channel_mask |= 1 << channel;
                match msg {
                    0x80 | 0xA0 | 0xB0 | 0xE0 => {
                        tp += 2;
                    }
                    0x90 => {
                        if tp + 1 < track_end && data[tp + 1] > 0 {
                            info.note_count += 1;
                        }
                        tp += 2;
                    }
                    0xC0 | 0xD0 => {
                        tp += 1;
                    }
                    _ => {
                        tp += 1;
                    }
                }
            }
        }

        if track_ticks > total_ticks {
            total_ticks = track_ticks;
        }
        pos = track_end;
    }

    info.channels_used = channel_mask.count_ones() as u16;

    // Compute duration from ticks and tempo
    if ppqn > 0 && tempo_us > 0 {
        let beats = total_ticks as f64 / ppqn as f64;
        info.duration = beats * (tempo_us as f64 / 1_000_000.0);
    }

    // Round tempo
    info.tempo = (info.tempo * 10.0).round() / 10.0;
    info.duration = (info.duration * 100.0).round() / 100.0;

    Some(info)
}

/// Read a MIDI variable-length quantity. Returns (value, bytes_consumed).
fn read_var_len(data: &[u8], pos: usize) -> (u32, usize) {
    let mut val = 0u32;
    let mut i = pos;
    loop {
        if i >= data.len() {
            return (val, i - pos);
        }
        let b = data[i];
        val = (val << 7) | (b & 0x7F) as u32;
        i += 1;
        if b & 0x80 == 0 {
            break;
        }
        if i - pos > 4 {
            break;
        } // safety: max 4 bytes
    }
    (val, i - pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_midi(format: u16, tracks: u16, ppqn: u16, track_data: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        // MThd
        data.extend_from_slice(b"MThd");
        data.extend_from_slice(&6u32.to_be_bytes()); // header length
        data.extend_from_slice(&format.to_be_bytes());
        data.extend_from_slice(&tracks.to_be_bytes());
        data.extend_from_slice(&ppqn.to_be_bytes());
        // MTrk
        data.extend_from_slice(b"MTrk");
        data.extend_from_slice(&(track_data.len() as u32).to_be_bytes());
        data.extend_from_slice(track_data);
        data
    }

    #[test]
    fn test_read_var_len_single_byte() {
        let data = [0x40u8];
        assert_eq!(read_var_len(&data, 0), (0x40, 1));
    }

    #[test]
    fn test_read_var_len_empty_slice() {
        let data: [u8; 0] = [];
        assert_eq!(read_var_len(&data, 0), (0, 0));
    }

    #[test]
    fn test_read_var_len_pos_past_end_returns_zero_bytes_consumed() {
        let data = [0x40u8];
        assert_eq!(read_var_len(&data, 1), (0, 0));
    }

    #[test]
    fn test_read_var_len_128_two_bytes() {
        let data = [0x81u8, 0x00];
        assert_eq!(read_var_len(&data, 0), (128, 2));
    }

    #[test]
    fn test_read_var_len_offset() {
        let data = [0x00u8, 0x81, 0x00];
        assert_eq!(read_var_len(&data, 1), (128, 2));
    }

    #[test]
    fn test_read_var_len_incomplete_uses_partial() {
        let data = [0x81u8]; // continuation without next byte
        let (v, n) = read_var_len(&data, 0);
        assert_eq!(n, 1);
        assert_eq!(v, 1); // only first 7 bits
    }

    #[test]
    fn test_read_var_len_large_value() {
        // 8192 = 0x2000: first group 0x40 (64), second 0x00 → (64<<7)|0 = 8192
        let data = [0xC0u8, 0x00];
        assert_eq!(read_var_len(&data, 0), (0x2000, 2));
    }

    #[test]
    fn test_read_var_len_fifth_byte_triggers_safety_break() {
        // Four continuation bytes then a final byte — 5 bytes total consumed before break
        let data = [0xFFu8, 0xFF, 0xFF, 0xFF, 0x7F];
        let (v, n) = read_var_len(&data, 0);
        assert_eq!(n, 5);
        assert!(v > 0);
    }

    #[test]
    fn test_parse_empty_midi() {
        // Single track with just end-of-track
        let track = vec![0x00, 0xFF, 0x2F, 0x00]; // delta=0, meta end-of-track
        let data = make_midi(0, 1, 480, &track);
        let tmp = std::env::temp_dir().join("test_empty.mid");
        std::fs::write(&tmp, &data).unwrap();
        let info = parse_midi(&tmp).unwrap();
        assert_eq!(info.format, 0);
        assert_eq!(info.track_count, 1);
        assert_eq!(info.ppqn, 480);
        assert_eq!(info.note_count, 0);
        assert_eq!(info.tempo, 120.0); // default
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_tempo() {
        // Set tempo to 140 BPM = 428571 µs/beat = 0x068A7B
        let track = vec![
            0x00, 0xFF, 0x51, 0x03, 0x06, 0x8A, 0x7B, // tempo meta event
            0x00, 0xFF, 0x2F, 0x00, // end of track
        ];
        let data = make_midi(0, 1, 480, &track);
        let tmp = std::env::temp_dir().join("test_tempo.mid");
        std::fs::write(&tmp, &data).unwrap();
        let info = parse_midi(&tmp).unwrap();
        assert!(
            (info.tempo - 140.0).abs() < 0.5,
            "tempo should be ~140, got {}",
            info.tempo
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_time_signature() {
        // 3/4 time: nn=3, dd=2 (2^2=4)
        let track = vec![
            0x00, 0xFF, 0x58, 0x04, 0x03, 0x02, 0x18, 0x08, // time sig meta
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let data = make_midi(0, 1, 480, &track);
        let tmp = std::env::temp_dir().join("test_timesig.mid");
        std::fs::write(&tmp, &data).unwrap();
        let info = parse_midi(&tmp).unwrap();
        assert_eq!(info.time_signature, "3/4");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_note_on_velocity_zero_not_counted() {
        // MIDI: note-on with velocity 0 is equivalent to note-off — must not increment note_count
        let track = vec![
            0x00, 0x90, 60, 0, // "note on" C4 vel 0
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let data = make_midi(0, 1, 480, &track);
        let tmp = std::env::temp_dir().join("test_vel0_note_on.mid");
        std::fs::write(&tmp, &data).unwrap();
        let info = parse_midi(&tmp).unwrap();
        assert_eq!(info.note_count, 0);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_notes() {
        let track = vec![
            0x00, 0x90, 60, 100, // note on C4 vel=100
            0x60, 0x80, 60, 0, // note off after 96 ticks
            0x00, 0x90, 64, 80, // note on E4
            0x60, 0x80, 64, 0, // note off
            0x00, 0x90, 67, 90, // note on G4
            0x60, 0x80, 67, 0, // note off
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let data = make_midi(0, 1, 480, &track);
        let tmp = std::env::temp_dir().join("test_notes.mid");
        std::fs::write(&tmp, &data).unwrap();
        let info = parse_midi(&tmp).unwrap();
        assert_eq!(info.note_count, 3);
        assert!(info.channels_used >= 1);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_track_name() {
        let name = b"Piano";
        let mut track = vec![0x00, 0xFF, 0x03, name.len() as u8];
        track.extend_from_slice(name);
        track.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
        let data = make_midi(0, 1, 480, &track);
        let tmp = std::env::temp_dir().join("test_trackname.mid");
        std::fs::write(&tmp, &data).unwrap();
        let info = parse_midi(&tmp).unwrap();
        assert_eq!(info.track_names, vec!["Piano"]);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_key_signature() {
        // C major: sf=0, mi=0
        let track = vec![
            0x00, 0xFF, 0x59, 0x02, 0x00, 0x00, // C major
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let data = make_midi(0, 1, 480, &track);
        let tmp = std::env::temp_dir().join("test_keysig.mid");
        std::fs::write(&tmp, &data).unwrap();
        let info = parse_midi(&tmp).unwrap();
        assert_eq!(info.key_signature, "C major");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_key_signature_minor() {
        // A minor: sf=0, mi=1
        let track = vec![
            0x00, 0xFF, 0x59, 0x02, 0x00, 0x01, // A minor
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let data = make_midi(0, 1, 480, &track);
        let tmp = std::env::temp_dir().join("test_keysig_minor.mid");
        std::fs::write(&tmp, &data).unwrap();
        let info = parse_midi(&tmp).unwrap();
        assert_eq!(info.key_signature, "A minor");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_duration() {
        // 480 ticks at 120 BPM (500000 µs/beat), ppqn=480 → 1 beat → 0.5 seconds
        let track = vec![
            0x00, 0x90, 60, 100, // note on at tick 0
            0x83, 0x60, 0x80, 60, 0, // note off at tick 480 (var len: 0x83 0x60 = 480)
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let data = make_midi(0, 1, 480, &track);
        let tmp = std::env::temp_dir().join("test_duration.mid");
        std::fs::write(&tmp, &data).unwrap();
        let info = parse_midi(&tmp).unwrap();
        assert!(
            (info.duration - 0.5).abs() < 0.1,
            "duration should be ~0.5s, got {}",
            info.duration
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_not_midi() {
        let tmp = std::env::temp_dir().join("test_not_midi.mid");
        std::fs::write(&tmp, b"not a midi file").unwrap();
        assert!(parse_midi(&tmp).is_none());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_midi_wrong_magic_not_mthd() {
        let tmp = std::env::temp_dir().join("test_wrong_magic.mid");
        std::fs::write(&tmp, b"RIFF....WAVEfmt ").unwrap();
        assert!(parse_midi(&tmp).is_none());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_midi_file_too_short_for_minimum_header() {
        let tmp = std::env::temp_dir().join("test_midi_too_short.mid");
        std::fs::write(&tmp, b"MThd\x00\x00\x00\x06").unwrap();
        assert!(parse_midi(&tmp).is_none());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_midi_smpte_division_defaults_ppqn() {
        let tmp = std::env::temp_dir().join("test_midi_smpte_ppqn.mid");
        let mut data = Vec::new();
        data.extend_from_slice(b"MThd");
        data.extend_from_slice(&6u32.to_be_bytes());
        data.extend_from_slice(&0u16.to_be_bytes());
        data.extend_from_slice(&0u16.to_be_bytes());
        data.extend_from_slice(&0x8001u16.to_be_bytes());
        std::fs::write(&tmp, &data).unwrap();
        let info = parse_midi(&tmp).unwrap();
        assert_eq!(info.ppqn, 480, "SMPTE timecode division → internal PPQN default");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_midi_ppqn_960_standard_division() {
        let track = vec![0x00, 0xFF, 0x2F, 0x00];
        let data = make_midi(0, 1, 960, &track);
        let tmp = std::env::temp_dir().join("test_midi_ppqn_960.mid");
        std::fs::write(&tmp, &data).unwrap();
        let info = parse_midi(&tmp).unwrap();
        assert_eq!(
            info.ppqn, 960,
            "PPQN ticks/quarter when division bit 15 is clear"
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_parse_midi_running_status_counts_second_note_on() {
        // First event sets status 0x90; second pair omits status byte (running status).
        let track = vec![
            0x00, 0x90, 60, 100, // note on C4
            0x00, 64, 90, // running: note on E4 (vel 90)
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let data = make_midi(0, 1, 480, &track);
        let tmp = std::env::temp_dir().join("test_midi_running_status.mid");
        std::fs::write(&tmp, &data).unwrap();
        let info = parse_midi(&tmp).unwrap();
        assert_eq!(info.note_count, 2);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_var_len() {
        assert_eq!(read_var_len(&[0x00], 0), (0, 1));
        assert_eq!(read_var_len(&[0x7F], 0), (127, 1));
        assert_eq!(read_var_len(&[0x81, 0x00], 0), (128, 2));
        assert_eq!(read_var_len(&[0x83, 0x60], 0), (480, 2));
    }
}
