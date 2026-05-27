//! Generate a toy Ableton project with group track structure and samples
//! This tests that we can create valid projects with the proper track hierarchy

use app_lib::als_generator::generate_empty_als;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use regex::Regex;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Embedded GroupTrack template from a working Ableton project
const GROUP_TRACK_TEMPLATE: &str = include_str!("../src/group_track_template.xml");

/// Sample paths from user's indexed library - multiple samples per track
const SAMPLES: &[&str] = &[
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Horns1.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Horns2.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Horns3.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Horns4.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Horns5.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Horns6.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Horns7.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Horns8.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trombones1.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trombones2.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trombones3.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trombones4.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trombones5.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trombones6.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trombones7.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trombones8.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trumpets1.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trumpets2.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trumpets3.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trumpets4.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trumpets5.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trumpets6.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trumpets7.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Trumpets8.wav",
    "/Users/wizard/mnt/production/MusicProduction/Samples/CM/Blockbuster/Clockwork Studio/Brass/Tuba1.wav",
];

/// Clips per track
const CLIPS_PER_TRACK: usize = 5;

fn main() {
    let output_path = Path::new("/Users/wizard/Desktop/Toy_Project.als");

    match generate_toy_project(output_path) {
        Ok(()) => {
            println!("Generated: {}", output_path.display());
            println!("Groups: Drums (Kick, Snare, HiHat), Synths (Bass, Lead)");
            println!("Each track has a sample at beat 1.");
            println!("Open in Ableton Live to verify.");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

struct SampleInfo {
    path: String,
    name: String,
    file_size: u64,
    is_core_library: bool,
}

impl SampleInfo {
    fn from_path(path: &str) -> Result<Self, String> {
        let metadata =
            std::fs::metadata(path).map_err(|e| format!("Cannot read {}: {}", path, e))?;
        let name = Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Sample")
            .to_string();
        let is_core_library = path.contains("Ableton Live") && path.contains("Core Library");
        Ok(Self {
            path: path.to_string(),
            name,
            file_size: metadata.len(),
            is_core_library,
        })
    }

    /// Get XML-escaped path (& -> &amp;, etc.)
    fn xml_path(&self) -> String {
        self.path
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    /// Get XML-escaped name
    fn xml_name(&self) -> String {
        self.name
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    /// Get LivePackName for XML
    fn live_pack_name(&self) -> &str {
        if self.is_core_library {
            "Core Library"
        } else {
            ""
        }
    }

    /// Get LivePackId for XML
    fn live_pack_id(&self) -> &str {
        if self.is_core_library {
            "www.ableton.com/0"
        } else {
            ""
        }
    }
}

fn generate_toy_project(output_path: &Path) -> Result<(), String> {
    // Load all samples
    let samples: Vec<SampleInfo> = SAMPLES
        .iter()
        .filter_map(|p| SampleInfo::from_path(p).ok())
        .collect();

    if samples.len() < 5 * CLIPS_PER_TRACK {
        return Err(format!(
            "Need at least {} samples, found {}",
            5 * CLIPS_PER_TRACK,
            samples.len()
        ));
    }

    generate_empty_als(output_path)?;

    let file = File::open(output_path).map_err(|e| e.to_string())?;
    let mut decoder = GzDecoder::new(file);
    let mut xml = String::new();
    decoder
        .read_to_string(&mut xml)
        .map_err(|e| e.to_string())?;

    // Extract the AudioTrack XML
    let track_start = xml.find("<AudioTrack").ok_or("No AudioTrack found")?;
    let track_end =
        xml.find("</AudioTrack>").ok_or("No AudioTrack end found")? + "</AudioTrack>".len();
    let original_audio_track = xml[track_start..track_end].to_string();

    // Group IDs and colors
    const DRUMS_GROUP_ID: u32 = 100000;
    const DRUMS_COLOR: u32 = 3;
    const SYNTHS_GROUP_ID: u32 = 100001;
    const SYNTHS_COLOR: u32 = 26;

    // Create groups
    let drums_group =
        create_group_track_from_template("Drums", DRUMS_COLOR, DRUMS_GROUP_ID, 110000)?;
    let synths_group =
        create_group_track_from_template("Synths", SYNTHS_COLOR, SYNTHS_GROUP_ID, 130000)?;

    // Create audio tracks with multiple samples in a row
    // Each track gets CLIPS_PER_TRACK samples, each 4 bars, placed sequentially
    let kick_samples: Vec<&SampleInfo> = samples[0..CLIPS_PER_TRACK].iter().collect();
    let snare_samples: Vec<&SampleInfo> = samples[CLIPS_PER_TRACK..2 * CLIPS_PER_TRACK]
        .iter()
        .collect();
    let hihat_samples: Vec<&SampleInfo> = samples[2 * CLIPS_PER_TRACK..3 * CLIPS_PER_TRACK]
        .iter()
        .collect();
    let bass_samples: Vec<&SampleInfo> = samples[3 * CLIPS_PER_TRACK..4 * CLIPS_PER_TRACK]
        .iter()
        .collect();
    let lead_samples: Vec<&SampleInfo> = samples[4 * CLIPS_PER_TRACK..5 * CLIPS_PER_TRACK]
        .iter()
        .collect();

    let snare = create_audio_track_with_clips(
        &original_audio_track,
        "Snare",
        DRUMS_COLOR,
        20000,
        DRUMS_GROUP_ID,
        &snare_samples,
        250000,
    )?;
    let hihat = create_audio_track_with_clips(
        &original_audio_track,
        "HiHat",
        DRUMS_COLOR,
        40000,
        DRUMS_GROUP_ID,
        &hihat_samples,
        260000,
    )?;
    let bass = create_audio_track_with_clips(
        &original_audio_track,
        "Bass",
        SYNTHS_COLOR,
        60000,
        SYNTHS_GROUP_ID,
        &bass_samples,
        270000,
    )?;
    let lead = create_audio_track_with_clips(
        &original_audio_track,
        "Lead",
        SYNTHS_COLOR,
        80000,
        SYNTHS_GROUP_ID,
        &lead_samples,
        280000,
    )?;

    // Modify original Kick track - need to change colors within the AudioTrack section
    // First, extract and modify the AudioTrack, then replace it back
    let kick_start = xml.find("<AudioTrack").ok_or("No AudioTrack")?;
    let kick_end = xml.find("</AudioTrack>").ok_or("No AudioTrack end")? + "</AudioTrack>".len();
    let mut kick_track = xml[kick_start..kick_end].to_string();

    // Modify Kick track
    kick_track = kick_track.replace(
        r#"<EffectiveName Value="1-Audio" />"#,
        r#"<EffectiveName Value="Kick" />"#,
    );
    kick_track = kick_track.replace(
        r#"<EffectiveName Value="Kick" />
					<UserName Value="" />"#,
        r#"<EffectiveName Value="Kick" />
					<UserName Value="Kick" />"#,
    );
    // Replace ALL colors in Kick track
    let color_re = Regex::new(r#"<Color Value="\d+" />"#).map_err(|e| e.to_string())?;
    kick_track = color_re
        .replace_all(&kick_track, format!(r#"<Color Value="{}" />"#, DRUMS_COLOR))
        .to_string();

    kick_track = kick_track.replacen(
        r#"<TrackGroupId Value="-1" />"#,
        &format!(r#"<TrackGroupId Value="{}" />"#, DRUMS_GROUP_ID),
        1,
    );

    // Add multiple samples to Kick track
    let kick_clips: Vec<String> = kick_samples
        .iter()
        .enumerate()
        .map(|(i, s)| {
            create_audio_clip(
                s,
                DRUMS_COLOR,
                240000 + i as u32 * 100,
                (i * 4 + 1) as u32,
                4,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let kick_clips_xml = kick_clips.join("\n");
    kick_track = kick_track.replacen(
        "<Events />",
        &format!(
            "<Events>\n{}\n\t\t\t\t\t\t\t\t\t\t\t\t\t</Events>",
            kick_clips_xml
        ),
        1,
    );

    // Replace the original AudioTrack with modified Kick
    let mut xml = format!("{}{}{}", &xml[..kick_start], kick_track, &xml[kick_end..]);

    // Insert Drums group before the AudioTrack
    let audio_track_pos = xml.find("<AudioTrack").ok_or("No AudioTrack")?;
    xml.insert_str(audio_track_pos, &format!("{}\n\t\t\t", drums_group));

    // Insert other tracks
    let insert_after_kick = "</AudioTrack>\n\t\t\t<ReturnTrack";
    let pos = xml
        .find(insert_after_kick)
        .ok_or("Insert marker not found")?;
    let insert_pos = pos + "</AudioTrack>\n\t\t\t".len();

    let middle_tracks = format!(
        "{}\n\t\t\t{}\n\t\t\t{}\n\t\t\t{}\n\t\t\t{}\n\t\t\t",
        snare, hihat, synths_group, bass, lead
    );
    xml.insert_str(insert_pos, &middle_tracks);

    // Update NextPointeeId
    xml = xml.replace(
        r#"<NextPointeeId Value="12720" />"#,
        r#"<NextPointeeId Value="500000" />"#,
    );

    let file = File::create(output_path).map_err(|e| e.to_string())?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder
        .write_all(xml.as_bytes())
        .map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())?;

    Ok(())
}

/// Create an AudioClip at a specific position
/// - start_bar: 1-indexed bar number (bar 1 = first bar)
/// - length_bars: clip length in bars (4 = 4 bars = 16 beats)
fn create_audio_clip(
    sample: &SampleInfo,
    color: u32,
    clip_id: u32,
    start_bar: u32,
    length_bars: u32,
) -> Result<String, String> {
    let beats_per_bar = 4;
    let start_beat = (start_bar - 1) * beats_per_bar; // bar 1 = beat 0, bar 5 = beat 16
    let length_beats = length_bars * beats_per_bar;
    let end_beat = start_beat + length_beats;

    Ok(format!(
        r#"<AudioClip Id="{clip_id}" Time="{start_beat}">
										<LomId Value="0" />
										<LomIdView Value="0" />
										<CurrentStart Value="{start_beat}" />
										<CurrentEnd Value="{end_beat}" />
										<Loop>
											<LoopStart Value="0" />
											<LoopEnd Value="{length_beats}" />
											<StartRelative Value="0" />
											<LoopOn Value="true" />
											<OutMarker Value="{length_beats}" />
											<HiddenLoopStart Value="0" />
											<HiddenLoopEnd Value="{length_beats}" />
										</Loop>
										<Name Value="{name}" />
										<Annotation Value="" />
										<Color Value="{color}" />
										<LaunchMode Value="0" />
										<LaunchQuantisation Value="0" />
										<TimeSignature>
											<TimeSignatures>
												<RemoteableTimeSignature Id="0">
													<Numerator Value="4" />
													<Denominator Value="4" />
													<Time Value="0" />
												</RemoteableTimeSignature>
											</TimeSignatures>
										</TimeSignature>
										<Envelopes>
											<Envelopes />
										</Envelopes>
										<ScrollerTimePreserver>
											<LeftTime Value="0" />
											<RightTime Value="{length_beats}" />
										</ScrollerTimePreserver>
										<TimeSelection>
											<AnchorTime Value="0" />
											<OtherTime Value="0" />
										</TimeSelection>
										<Legato Value="false" />
										<Ram Value="false" />
										<GrooveSettings>
											<GrooveId Value="-1" />
										</GrooveSettings>
										<Disabled Value="false" />
										<VelocityAmount Value="0" />
										<FollowAction>
											<FollowTime Value="4" />
											<IsLinked Value="true" />
											<LoopIterations Value="1" />
											<FollowActionA Value="4" />
											<FollowActionB Value="0" />
											<FollowChanceA Value="100" />
											<FollowChanceB Value="0" />
											<JumpIndexA Value="1" />
											<JumpIndexB Value="1" />
											<FollowActionEnabled Value="false" />
										</FollowAction>
										<Grid>
											<FixedNumerator Value="1" />
											<FixedDenominator Value="16" />
											<GridIntervalPixel Value="20" />
											<Ntoles Value="2" />
											<SnapToGrid Value="true" />
											<Fixed Value="false" />
										</Grid>
										<FreezeStart Value="0" />
										<FreezeEnd Value="0" />
										<IsWarped Value="true" />
										<TakeId Value="1" />
										<IsInKey Value="true" />
										<ScaleInformation>
											<Root Value="0" />
											<Name Value="0" />
										</ScaleInformation>
										<SampleRef>
											<FileRef>
												<RelativePathType Value="0" />
												<RelativePath Value="" />
												<Path Value="{path}" />
												<Type Value="2" />
												<LivePackName Value="{live_pack_name}" />
												<LivePackId Value="{live_pack_id}" />
												<OriginalFileSize Value="{file_size}" />
												<OriginalCrc Value="0" />
												<SourceHint Value="" />
											</FileRef>
											<LastModDate Value="0" />
											<SourceContext>
												<SourceContext Id="0">
													<OriginalFileRef>
														<FileRef Id="0">
															<RelativePathType Value="0" />
															<RelativePath Value="" />
															<Path Value="{path}" />
															<Type Value="2" />
															<LivePackName Value="{live_pack_name}" />
															<LivePackId Value="{live_pack_id}" />
															<OriginalFileSize Value="{file_size}" />
															<OriginalCrc Value="0" />
															<SourceHint Value="" />
														</FileRef>
													</OriginalFileRef>
													<BrowserContentPath Value="" />
													<LocalFiltersJson Value="" />
												</SourceContext>
											</SourceContext>
											<SampleUsageHint Value="0" />
											<DefaultDuration Value="88200" />
											<DefaultSampleRate Value="44100" />
											<SamplesToAutoWarp Value="1" />
										</SampleRef>
										<Onsets>
											<UserOnsets />
											<HasUserOnsets Value="false" />
										</Onsets>
										<WarpMode Value="0" />
										<GranularityTones Value="30" />
										<GranularityTexture Value="65" />
										<FluctuationTexture Value="25" />
										<TransientResolution Value="6" />
										<TransientLoopMode Value="2" />
										<TransientEnvelope Value="100" />
										<ComplexProFormants Value="100" />
										<ComplexProEnvelope Value="128" />
										<Sync Value="true" />
										<HiQ Value="true" />
										<Fade Value="true" />
										<Fades>
											<FadeInLength Value="0" />
											<FadeOutLength Value="0" />
											<ClipFadesAreInitialized Value="true" />
											<CrossfadeInState Value="0" />
											<FadeInCurveSkew Value="0" />
											<FadeInCurveSlope Value="0" />
											<FadeOutCurveSkew Value="0" />
											<FadeOutCurveSlope Value="0" />
											<IsDefaultFadeIn Value="true" />
											<IsDefaultFadeOut Value="true" />
										</Fades>
										<PitchCoarse Value="0" />
										<PitchFine Value="0" />
										<SampleVolume Value="1" />
										<WarpMarkers>
											<WarpMarker Id="0" SecTime="0" BeatTime="0" />
											<WarpMarker Id="1" SecTime="7" BeatTime="{length_beats}" />
										</WarpMarkers>
										<SavedWarpMarkersForStretched />
										<MarkersGenerated Value="true" />
										<IsSongTempoLeader Value="false" />
									</AudioClip>"#,
        clip_id = clip_id,
        start_beat = start_beat,
        end_beat = end_beat,
        length_beats = length_beats,
        name = sample.xml_name(),
        color = color,
        path = sample.xml_path(),
        file_size = sample.file_size,
        live_pack_name = sample.live_pack_name(),
        live_pack_id = sample.live_pack_id()
    ))
}

fn create_group_track_from_template(
    name: &str,
    color: u32,
    group_id: u32,
    id_offset: u32,
) -> Result<String, String> {
    let mut track = GROUP_TRACK_TEMPLATE.to_string();

    let track_id_re = Regex::new(r#"<GroupTrack Id="\d+""#).map_err(|e| e.to_string())?;
    track = track_id_re
        .replace(&track, format!(r#"<GroupTrack Id="{}""#, group_id))
        .to_string();

    track = track.replace(
        r#"<EffectiveName Value="Drums" />"#,
        &format!(r#"<EffectiveName Value="{}" />"#, name),
    );
    track = track.replace(
        r#"<UserName Value="Drums" />"#,
        &format!(r#"<UserName Value="{}" />"#, name),
    );

    let color_re = Regex::new(r#"<Color Value="\d+" />"#).map_err(|e| e.to_string())?;
    track = color_re
        .replace(&track, format!(r#"<Color Value="{}" />"#, color))
        .to_string();

    let id_re = Regex::new(r#"Id="(\d+)""#).map_err(|e| e.to_string())?;
    let mut first = true;
    track = id_re
        .replace_all(&track, |caps: &regex::Captures| {
            if first {
                first = false;
                format!(r#"Id="{}""#, group_id)
            } else {
                let old_id: u32 = caps[1].parse().unwrap_or(0);
                let new_id = old_id + id_offset;
                format!(r#"Id="{}""#, new_id)
            }
        })
        .to_string();

    eprintln!("GroupTrack {}: ID={}", name, group_id);
    Ok(track)
}

fn create_audio_track_with_sample(
    template: &str,
    name: &str,
    color: u32,
    id_offset: u32,
    group_id: u32,
    sample: &SampleInfo,
    clip_id: u32,
    start_bar: u32,
    length_bars: u32,
) -> Result<String, String> {
    let mut track = template.to_string();

    // Set name
    let name_re = Regex::new(r#"<EffectiveName Value="[^"]*" />"#).map_err(|e| e.to_string())?;
    track = name_re
        .replace(&track, format!(r#"<EffectiveName Value="{}" />"#, name))
        .to_string();

    let username_re =
        Regex::new(r#"(<EffectiveName Value="[^"]*" />\s*<UserName Value=")[^"]*(" />)"#)
            .map_err(|e| e.to_string())?;
    track = username_re
        .replace(&track, format!(r#"${{1}}{}${{2}}"#, name))
        .to_string();

    // Set ALL colors in the track to match (track color + device colors)
    let color_re = Regex::new(r#"<Color Value="\d+" />"#).map_err(|e| e.to_string())?;
    track = color_re
        .replace_all(&track, format!(r#"<Color Value="{}" />"#, color))
        .to_string();

    // Set group membership
    track = track.replacen(
        r#"<TrackGroupId Value="-1" />"#,
        &format!(r#"<TrackGroupId Value="{}" />"#, group_id),
        1,
    );

    // Add AudioClip to <Events />
    let clip = create_audio_clip(sample, color, clip_id, start_bar, length_bars)?;
    track = track.replacen(
        "<Events />",
        &format!("<Events>\n{}\n\t\t\t\t\t\t\t\t\t\t\t\t\t</Events>", clip),
        1,
    );

    // Offset all IDs
    let id_re = Regex::new(r#"Id="(\d+)""#).map_err(|e| e.to_string())?;
    track = id_re
        .replace_all(&track, |caps: &regex::Captures| {
            let old_id: u32 = caps[1].parse().unwrap_or(0);
            let new_id = old_id + id_offset;
            format!(r#"Id="{}""#, new_id)
        })
        .to_string();

    eprintln!(
        "AudioTrack {}: offset={}, group={}, clip={}, bar={}",
        name, id_offset, group_id, clip_id, start_bar
    );
    Ok(track)
}

fn create_audio_track_with_clips(
    template: &str,
    name: &str,
    color: u32,
    id_offset: u32,
    group_id: u32,
    samples: &[&SampleInfo],
    base_clip_id: u32,
) -> Result<String, String> {
    let mut track = template.to_string();

    // Set name
    let name_re = Regex::new(r#"<EffectiveName Value="[^"]*" />"#).map_err(|e| e.to_string())?;
    track = name_re
        .replace(&track, format!(r#"<EffectiveName Value="{}" />"#, name))
        .to_string();

    let username_re =
        Regex::new(r#"(<EffectiveName Value="[^"]*" />\s*<UserName Value=")[^"]*(" />)"#)
            .map_err(|e| e.to_string())?;
    track = username_re
        .replace(&track, format!(r#"${{1}}{}${{2}}"#, name))
        .to_string();

    // Set ALL colors in the track to match (track color + device colors)
    let color_re = Regex::new(r#"<Color Value="\d+" />"#).map_err(|e| e.to_string())?;
    track = color_re
        .replace_all(&track, format!(r#"<Color Value="{}" />"#, color))
        .to_string();

    // Set group membership
    track = track.replacen(
        r#"<TrackGroupId Value="-1" />"#,
        &format!(r#"<TrackGroupId Value="{}" />"#, group_id),
        1,
    );

    // Create all AudioClips - each sample gets 4 bars, placed sequentially
    let clips: Vec<String> = samples
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let clip_id = base_clip_id + i as u32 * 100;
            let start_bar = (i * 4 + 1) as u32; // bar 1, 5, 9, 13, etc.
            create_audio_clip(s, color, clip_id, start_bar, 4)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let clips_xml = clips.join("\n");
    track = track.replacen(
        "<Events />",
        &format!(
            "<Events>\n{}\n\t\t\t\t\t\t\t\t\t\t\t\t\t</Events>",
            clips_xml
        ),
        1,
    );

    // Offset all IDs
    let id_re = Regex::new(r#"Id="(\d+)""#).map_err(|e| e.to_string())?;
    track = id_re
        .replace_all(&track, |caps: &regex::Captures| {
            let old_id: u32 = caps[1].parse().unwrap_or(0);
            let new_id = old_id + id_offset;
            format!(r#"Id="{}""#, new_id)
        })
        .to_string();

    eprintln!(
        "AudioTrack {}: offset={}, group={}, {} clips",
        name,
        id_offset,
        group_id,
        samples.len()
    );
    Ok(track)
}
