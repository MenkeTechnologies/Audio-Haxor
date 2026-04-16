//! Ableton Live Set (.als) generator for programmatic techno track creation.
//!
//! Generates valid gzip-compressed XML files that Ableton Live can open.
//! Uses an embedded reference template from Live 12.3.7 for guaranteed compatibility.
//! Uses samples from the indexed library to create full arrangements.

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

/// Escape a string for safe XML attribute interpolation.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Embedded empty project template (gzipped) from Ableton Live 12.3.7
/// This is a valid minimal project that Ableton will open without errors.
const EMPTY_PROJECT_TEMPLATE: &[u8] = include_bytes!("empty_project_template.als.gz");

/// Ableton Live version info extracted from installed app
#[derive(Debug, Clone)]
pub struct AbletonVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub creator: String,
    pub minor_version_string: String,
}

impl Default for AbletonVersion {
    fn default() -> Self {
        Self {
            major: 12,
            minor: 0,
            patch: 0,
            creator: "Ableton Live 12.0".to_string(),
            minor_version_string: "12.0_12000".to_string(),
        }
    }
}

impl AbletonVersion {
    /// Detect Ableton version from installed app at standard macOS path
    pub fn detect() -> Self {
        Self::detect_from_path("/Applications/Ableton Live 12 Suite.app")
            .or_else(|| Self::detect_from_path("/Applications/Ableton Live 12 Standard.app"))
            .or_else(|| Self::detect_from_path("/Applications/Ableton Live 12 Intro.app"))
            .or_else(|| Self::detect_from_path("/Applications/Ableton Live 11 Suite.app"))
            .or_else(|| Self::detect_from_path("/Applications/Ableton Live 11 Standard.app"))
            .unwrap_or_default()
    }

    /// Detect version from a specific app bundle path
    pub fn detect_from_path(app_path: &str) -> Option<Self> {
        let plist_path = format!("{}/Contents/Info.plist", app_path);
        if !Path::new(&plist_path).exists() {
            return None;
        }

        let output = Command::new("defaults")
            .args(["read", &plist_path, "CFBundleShortVersionString"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let version_str = String::from_utf8_lossy(&output.stdout);
        let version_str = version_str.trim();

        Self::parse_version_string(version_str)
    }

    /// Parse version string like "12.3.7 (2026-03-30_c92a51f028)"
    fn parse_version_string(version_str: &str) -> Option<Self> {
        let version_part = version_str.split_whitespace().next()?;
        let parts: Vec<&str> = version_part.split('.').collect();

        let major: u32 = parts.first()?.parse().ok()?;
        let minor: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch: u32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

        // Use Live 11.0 creator string for compatibility
        let creator = "Ableton Live 11.0".to_string();
        
        // Use Live 11 format for maximum compatibility.
        // Live 12 can open Live 11 files, but not vice versa.
        // The XML structure differs significantly between versions.
        let minor_version_string = "11.0_433".to_string();

        Some(Self {
            major,
            minor,
            patch,
            creator,
            minor_version_string,
        })
    }
}

/// Unique ID allocator for ALS XML elements
struct IdAllocator {
    next_id: u64,
}

impl IdAllocator {
    fn new(start: u64) -> Self {
        Self { next_id: start }
    }

    fn next(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

/// Represents an audio sample to be placed in the arrangement
#[derive(Debug, Clone, PartialEq)]
pub struct SampleInfo {
    pub path: String,
    pub name: String,
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub file_size: u64,
    pub bpm: Option<f64>,
}

/// An audio clip placement in the arrangement
#[derive(Debug, Clone, PartialEq)]
pub struct ClipPlacement {
    pub sample: SampleInfo,
    pub start_beat: f64,
    pub duration_beats: f64,
}

/// A track containing multiple clip placements
#[derive(Debug, Clone, PartialEq)]
pub struct TrackInfo {
    pub name: String,
    pub color: u8,
    pub clips: Vec<ClipPlacement>,
}

impl SampleInfo {
    fn loop_bars(&self, project_bpm: f64) -> u32 {
        let bpm = self.bpm.unwrap_or(project_bpm);
        let duration = if self.duration_secs <= 0.0 || self.duration_secs > 300.0 {
            (4.0 * 60.0 * 4.0) / project_bpm
        } else {
            self.duration_secs
        };
        if bpm <= 0.0 { return 4; }
        let bars = (duration * bpm) / (60.0 * 4.0);
        if bars <= 1.5 { 1 }
        else if bars <= 3.0 { 2 }
        else if bars <= 6.0 { 4 }
        else if bars <= 12.0 { 8 }
        else { 16 }
    }
}

/// Techno arrangement section
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Intro,
    Buildup,
    Drop,
    Breakdown,
    Drop2,
    Outro,
}

impl Section {
    fn bars(&self) -> u32 {
        match self {
            Section::Intro => 16,
            Section::Buildup => 16,
            Section::Drop => 32,
            Section::Breakdown => 16,
            Section::Drop2 => 32,
            Section::Outro => 16,
        }
    }
}

/// Configuration for generating a techno track
#[derive(Debug, Clone, PartialEq)]
pub struct TechnoConfig {
    pub bpm: f64,
    pub kick: SampleInfo,
    pub clap: SampleInfo,
    pub hat: SampleInfo,
}

impl TechnoConfig {
    /// Generate a full techno arrangement with standard structure
    pub fn generate_arrangement(&self) -> Vec<TrackInfo> {
        let sections = [
            Section::Intro,
            Section::Buildup,
            Section::Drop,
            Section::Breakdown,
            Section::Drop2,
            Section::Outro,
        ];

        let mut kick_clips = Vec::new();
        let mut clap_clips = Vec::new();
        let mut hat_clips = Vec::new();

        let mut current_bar: u32 = 0;

        for section in sections {
            let section_bars = section.bars();
            let start_beat = (current_bar * 4) as f64;

            match section {
                Section::Intro => {
                    // Kick every 4 bars, building up
                    for bar in (0..section_bars).step_by(4) {
                        let beat = start_beat + (bar * 4) as f64;
                        kick_clips.push(ClipPlacement {
                            sample: self.kick.clone(),
                            start_beat: beat,
                            duration_beats: 1.0,
                        });
                    }
                }
                Section::Buildup => {
                    // Kick on every beat, hats 8th notes, clap on 2 and 4
                    for bar in 0..section_bars {
                        for beat_in_bar in 0..4 {
                            let beat = start_beat + (bar * 4 + beat_in_bar) as f64;
                            kick_clips.push(ClipPlacement {
                                sample: self.kick.clone(),
                                start_beat: beat,
                                duration_beats: 1.0,
                            });
                            // Hats on 8th notes
                            hat_clips.push(ClipPlacement {
                                sample: self.hat.clone(),
                                start_beat: beat,
                                duration_beats: 0.5,
                            });
                            hat_clips.push(ClipPlacement {
                                sample: self.hat.clone(),
                                start_beat: beat + 0.5,
                                duration_beats: 0.5,
                            });
                            // Clap on 2 and 4
                            if beat_in_bar == 1 || beat_in_bar == 3 {
                                clap_clips.push(ClipPlacement {
                                    sample: self.clap.clone(),
                                    start_beat: beat,
                                    duration_beats: 1.0,
                                });
                            }
                        }
                    }
                }
                Section::Drop | Section::Drop2 => {
                    // Full pattern: kick on every beat, hat 16ths, clap on 2&4
                    for bar in 0..section_bars {
                        for beat_in_bar in 0..4 {
                            let beat = start_beat + (bar * 4 + beat_in_bar) as f64;
                            kick_clips.push(ClipPlacement {
                                sample: self.kick.clone(),
                                start_beat: beat,
                                duration_beats: 1.0,
                            });
                            // Hats on 16th notes
                            for sixteenth in 0..4 {
                                hat_clips.push(ClipPlacement {
                                    sample: self.hat.clone(),
                                    start_beat: beat + (sixteenth as f64 * 0.25),
                                    duration_beats: 0.25,
                                });
                            }
                            // Clap on 2 and 4
                            if beat_in_bar == 1 || beat_in_bar == 3 {
                                clap_clips.push(ClipPlacement {
                                    sample: self.clap.clone(),
                                    start_beat: beat,
                                    duration_beats: 1.0,
                                });
                            }
                        }
                    }
                }
                Section::Breakdown => {
                    // No kick, just hats and occasional clap
                    for bar in 0..section_bars {
                        for beat_in_bar in 0..4 {
                            let beat = start_beat + (bar * 4 + beat_in_bar) as f64;
                            hat_clips.push(ClipPlacement {
                                sample: self.hat.clone(),
                                start_beat: beat,
                                duration_beats: 0.5,
                            });
                            // Sparse claps
                            if bar % 4 == 3 && beat_in_bar == 3 {
                                clap_clips.push(ClipPlacement {
                                    sample: self.clap.clone(),
                                    start_beat: beat,
                                    duration_beats: 1.0,
                                });
                            }
                        }
                    }
                }
                Section::Outro => {
                    // Kick fading out, every 2 bars then every 4
                    for bar in (0..section_bars / 2).step_by(2) {
                        let beat = start_beat + (bar * 4) as f64;
                        kick_clips.push(ClipPlacement {
                            sample: self.kick.clone(),
                            start_beat: beat,
                            duration_beats: 1.0,
                        });
                    }
                }
            }

            current_bar += section_bars;
        }

        vec![
            TrackInfo {
                name: "Kick".to_string(),
                color: 69, // Orange
                clips: kick_clips,
            },
            TrackInfo {
                name: "Clap".to_string(),
                color: 26, // Purple
                clips: clap_clips,
            },
            TrackInfo {
                name: "Hat".to_string(),
                color: 17, // Yellow
                clips: hat_clips,
            },
        ]
    }
}

/// Generate an Ableton Live Set file
pub fn generate_als(
    output_path: &Path,
    tracks: &[TrackInfo],
    bpm: f64,
) -> Result<(), String> {
    generate_als_with_version(output_path, tracks, bpm, &AbletonVersion::detect())
}

/// Generate ALS with specific Ableton version
pub fn generate_als_with_version(
    output_path: &Path,
    tracks: &[TrackInfo],
    bpm: f64,
    version: &AbletonVersion,
) -> Result<(), String> {
    let mut ids = IdAllocator::new(1000);

    let mut tracks_xml = String::new();
    for track in tracks {
        tracks_xml.push_str(&generate_audio_track(track, &mut ids));
    }

    let next_pointee_id = ids.next();

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Ableton MajorVersion="5" MinorVersion="{minor_version}" SchemaChangeCount="3" Creator="{creator}" Revision="">"#,
        minor_version = version.minor_version_string,
        creator = version.creator,
    ) + &format!(
        r#"
	<LiveSet>
		<NextPointeeId Value="{next_pointee_id}" />
		<OverwriteProtectionNumber Value="2819" />
		<LomId Value="0" />
		<LomIdView Value="0" />
		<Tracks>
{tracks_xml}		</Tracks>
		<MasterTrack>
{master_track}
		</MasterTrack>
		<PreHearTrack>
{prehear_track}
		</PreHearTrack>
		<SendsPre>
			<SendPreBool Id="0" Value="true" />
			<SendPreBool Id="1" Value="true" />
		</SendsPre>
		<Transport>
			<PhaseNudgeTempo Value="{bpm}" />
			<LoopOn Value="false" />
			<LoopStart Value="0" />
			<LoopLength Value="16" />
			<LoopIsSongStart Value="false" />
			<CurrentTime Value="0" />
			<PunchIn Value="false" />
			<PunchOut Value="false" />
			<MetronomeClickOn Value="false" />
			<DrawMode Value="false" />
		</Transport>
		<SongMasterValues>
			<SessionTempo Value="{bpm}" />
			<SessionTempoHasListener Value="false" />
			<SessionTimeSignatureNumerator Value="4" />
			<SessionTimeSignatureDenominator Value="4" />
		</SongMasterValues>
		<GlobalQuantisation Value="4" />
		<AutoQuantisation Value="0" />
		<Grid>
			<FixedNumerator Value="1" />
			<FixedDenominator Value="16" />
			<GridIntervalPixel Value="20" />
			<Ntoles Value="2" />
			<SnapToGrid Value="true" />
			<Fixed Value="false" />
		</Grid>
		<ScaleInformation>
			<RootNote Value="0" />
			<Name Value="Major" />
		</ScaleInformation>
		<InKey Value="false" />
		<SmpteFormat Value="0" />
		<TimeSelection>
			<AnchorTime Value="0" />
			<OtherTime Value="0" />
		</TimeSelection>
		<SequencerNavigator>
			<BeatTimeHelper>
				<CurrentZoom Value="0.30000001192092896" />
			</BeatTimeHelper>
			<ScrollerPos X="-1" Y="-1" />
			<ClientSize X="0" Y="0" />
		</SequencerNavigator>
		<ViewStateSessionMixerHeight Value="120" />
		<IsContentSplitterOpen Value="true" />
		<IsExpressionSplitterOpen Value="true" />
		<ExpressionLanes>
			<ExpressionLane Id="0">
				<Type Value="0" />
				<Size Value="41" />
				<IsMinimized Value="false" />
			</ExpressionLane>
			<ExpressionLane Id="1">
				<Type Value="1" />
				<Size Value="41" />
				<IsMinimized Value="false" />
			</ExpressionLane>
			<ExpressionLane Id="2">
				<Type Value="2" />
				<Size Value="41" />
				<IsMinimized Value="true" />
			</ExpressionLane>
			<ExpressionLane Id="3">
				<Type Value="3" />
				<Size Value="41" />
				<IsMinimized Value="true" />
			</ExpressionLane>
		</ExpressionLanes>
		<ContentLanes>
			<ExpressionLane Id="0">
				<Type Value="4" />
				<Size Value="41" />
				<IsMinimized Value="false" />
			</ExpressionLane>
			<ExpressionLane Id="1">
				<Type Value="5" />
				<Size Value="25" />
				<IsMinimized Value="true" />
			</ExpressionLane>
		</ContentLanes>
		<ViewStateFxSlotCount Value="4" />
		<Locators>
			<Locators />
		</Locators>
		<DetailClipKeyMidis />
		<TracksListWrapper LomId="0" />
		<VisibleTracksListWrapper LomId="0" />
		<ReturnTracksListWrapper LomId="0" />
		<ScenesListWrapper LomId="0" />
		<CuePointsListWrapper LomId="0" />
		<ChooserBar Value="0" />
		<Annotation Value="" />
		<SoloOrPflSavedValue Value="true" />
		<SoloInPlace Value="false" />
		<CrossfadeCurve Value="2" />
		<LatencyCompensation Value="2" />
		<HighlightedTrackIndex Value="0" />
		<GroovePool>
			<Grooves />
		</GroovePool>
		<AutomationMode Value="false" />
		<SnapAutomationToGrid Value="true" />
		<ArrangementOverdub Value="false" />
		<ColorSequenceIndex Value="0" />
		<AutoColorPickerForPlayerAndGroupTracks Value="false" />
		<AutoColorPickerForReturnAndMasterTracks Value="false" />
		<ViewData Value="{{}}" />
		<MidiFoldIn Value="false" />
		<MidiPrelisten Value="false" />
		<LinkedTrackGroups />
		<Scenes>
			<Scene Id="0">
				<LomId Value="0" />
				<Name Value="" />
				<Annotation Value="" />
				<Color Value="-1" />
				<Tempo Value="120" />
				<IsTempoEnabled Value="false" />
				<TimeSignatureId Value="201" />
				<IsTimeSignatureEnabled Value="false" />
				<LomIdView Value="0" />
				<ClipSlotsListWrapper LomId="0" />
			</Scene>
		</Scenes>
	</LiveSet>
</Ableton>
"#,
        next_pointee_id = next_pointee_id,
        tracks_xml = tracks_xml,
        master_track = generate_master_track(&mut ids),
        prehear_track = generate_prehear_track(&mut ids),
        bpm = bpm,
    );

    // Compress with gzip
    let file = File::create(output_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder
        .write_all(xml.as_bytes())
        .map_err(|e| format!("Failed to write compressed data: {}", e))?;
    encoder
        .finish()
        .map_err(|e| format!("Failed to finish compression: {}", e))?;

    Ok(())
}

fn generate_audio_track(track: &TrackInfo, ids: &mut IdAllocator) -> String {
    let track_id = ids.next();
    let mut clips_xml = String::new();

    for clip in &track.clips {
        clips_xml.push_str(&generate_audio_clip(clip, ids));
    }

    // Generate 8 empty clip slots for session view
    let mut clip_slots = String::new();
    for i in 0..8 {
        clip_slots.push_str(&format!(
            r#"							<ClipSlot Id="{i}">
								<LomId Value="0" />
								<ClipSlot>
									<Value />
								</ClipSlot>
								<HasStop Value="true" />
								<NeedRefreeze Value="true" />
							</ClipSlot>
"#,
            i = i
        ));
    }

    let automation_target_id = ids.next();
    let pointee_id = ids.next();

    format!(
        r#"			<AudioTrack Id="{track_id}">
				<LomId Value="0" />
				<LomIdView Value="0" />
				<IsContentSelectedInDocument Value="false" />
				<PreferredContentViewMode Value="0" />
				<TrackDelay>
					<Value Value="0" />
					<IsValueSampleBased Value="false" />
				</TrackDelay>
				<Name>
					<EffectiveName Value="{name}" />
					<UserName Value="{name}" />
					<Annotation Value="" />
					<MemorizedFirstClipName Value="" />
				</Name>
				<Color Value="{color}" />
				<AutomationEnvelopes>
					<Envelopes />
				</AutomationEnvelopes>
				<TrackGroupId Value="-1" />
				<TrackUnfolded Value="true" />
				<DevicesListWrapper LomId="0" />
				<ClipSlotsListWrapper LomId="0" />
				<ArrangementClipsListWrapper LomId="0" />
				<TakeLanesListWrapper LomId="0" />
				<ViewData Value="{{}}" />
				<TakeLanes>
					<TakeLanes>
						<TakeLane Id="0">
							<LomId Value="0" />
							<Height Value="68" />
							<IsContentSelectedInDocument Value="false" />
							<ClipAutomation>
								<Events>
{clips_xml}								</Events>
							</ClipAutomation>
						</TakeLane>
					</TakeLanes>
					<AreTakeLanesFolded Value="true" />
				</TakeLanes>
				<LinkedTrackGroupId Value="-1" />
				<SavedPlayingSlot Value="-1" />
				<SavedPlayingOffset Value="0" />
				<Freeze Value="false" />
				<NeedArrangerRefreeze Value="true" />
				<PostProcessFreezeClips Value="0" />
				<DeviceChain>
					<AutomationLanes>
						<AutomationLanes>
							<AutomationLane Id="0">
								<SelectedDevice Value="0" />
								<SelectedEnvelope Value="0" />
								<IsContentSelectedInDocument Value="false" />
								<LaneHeight Value="68" />
							</AutomationLane>
						</AutomationLanes>
						<AreAdditionalAutomationLanesFolded Value="false" />
					</AutomationLanes>
					<ClipEnvelopeChooserViewState>
						<SelectedDevice Value="0" />
						<SelectedEnvelope Value="0" />
						<PreferModulationVisible Value="false" />
					</ClipEnvelopeChooserViewState>
					<AudioInputRouting>
						<Target Value="AudioIn/External/S0" />
						<UpperDisplayString Value="Ext. In" />
						<LowerDisplayString Value="1/2" />
						<MpeSettings>
							<ZoneType Value="0" />
							<FirstNoteChannel Value="1" />
							<LastNoteChannel Value="15" />
						</MpeSettings>
						<MpePitchBendUsesTuning Value="true" />
					</AudioInputRouting>
					<MidiInputRouting>
						<Target Value="MidiIn/External.All/-1" />
						<UpperDisplayString Value="Ext: All Ins" />
						<LowerDisplayString Value="" />
						<MpeSettings>
							<ZoneType Value="0" />
							<FirstNoteChannel Value="1" />
							<LastNoteChannel Value="15" />
						</MpeSettings>
						<MpePitchBendUsesTuning Value="true" />
					</MidiInputRouting>
					<AudioOutputRouting>
						<Target Value="AudioOut/Master" />
						<UpperDisplayString Value="Master" />
						<LowerDisplayString Value="" />
						<MpeSettings>
							<ZoneType Value="0" />
							<FirstNoteChannel Value="1" />
							<LastNoteChannel Value="15" />
						</MpeSettings>
						<MpePitchBendUsesTuning Value="true" />
					</AudioOutputRouting>
					<MidiOutputRouting>
						<Target Value="MidiOut/None" />
						<UpperDisplayString Value="None" />
						<LowerDisplayString Value="" />
						<MpeSettings>
							<ZoneType Value="0" />
							<FirstNoteChannel Value="1" />
							<LastNoteChannel Value="15" />
						</MpeSettings>
						<MpePitchBendUsesTuning Value="true" />
					</MidiOutputRouting>
					<Mixer>
						<LomId Value="0" />
						<LomIdView Value="0" />
						<IsExpanded Value="true" />
						<BreakoutIsExpanded Value="false" />
						<On>
							<LomId Value="0" />
							<Manual Value="true" />
							<AutomationTarget Id="{automation_target_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<MidiCCOnOffThresholds>
								<Min Value="64" />
								<Max Value="127" />
							</MidiCCOnOffThresholds>
						</On>
						<ModulationSourceCount Value="0" />
						<ParametersListWrapper LomId="0" />
						<Pointee Id="{pointee_id}" />
						<LastSelectedTimeableIndex Value="0" />
						<LastSelectedClipEnvelopeIndex Value="0" />
						<LastPresetRef>
							<Value />
						</LastPresetRef>
						<LockedScripts />
						<IsFolded Value="false" />
						<ShouldShowPresetName Value="false" />
						<UserName Value="" />
						<Annotation Value="" />
						<SourceContext>
							<Value />
						</SourceContext>
						<MpePitchBendUsesTuning Value="true" />
						<ViewData Value="{{}}" />
						<Sends />
						<Speaker>
							<LomId Value="0" />
							<Manual Value="true" />
							<AutomationTarget Id="{speaker_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<MidiCCOnOffThresholds>
								<Min Value="64" />
								<Max Value="127" />
							</MidiCCOnOffThresholds>
						</Speaker>
						<SoloSink Value="false" />
						<PanMode Value="0" />
						<Pan>
							<LomId Value="0" />
							<Manual Value="0" />
							<MidiControllerRange>
								<Min Value="-1" />
								<Max Value="1" />
							</MidiControllerRange>
							<AutomationTarget Id="{pan_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<ModulationTarget Id="{pan_mod_id}">
								<LockEnvelope Value="0" />
							</ModulationTarget>
						</Pan>
						<SplitStereoPanL>
							<LomId Value="0" />
							<Manual Value="-1" />
							<MidiControllerRange>
								<Min Value="-1" />
								<Max Value="1" />
							</MidiControllerRange>
							<AutomationTarget Id="{split_l_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<ModulationTarget Id="{split_l_mod_id}">
								<LockEnvelope Value="0" />
							</ModulationTarget>
						</SplitStereoPanL>
						<SplitStereoPanR>
							<LomId Value="0" />
							<Manual Value="1" />
							<MidiControllerRange>
								<Min Value="-1" />
								<Max Value="1" />
							</MidiControllerRange>
							<AutomationTarget Id="{split_r_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<ModulationTarget Id="{split_r_mod_id}">
								<LockEnvelope Value="0" />
							</ModulationTarget>
						</SplitStereoPanR>
						<Volume>
							<LomId Value="0" />
							<Manual Value="0.794328212738037" />
							<MidiControllerRange>
								<Min Value="0.0003162277571" />
								<Max Value="1.99526226520538" />
							</MidiControllerRange>
							<AutomationTarget Id="{volume_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<ModulationTarget Id="{volume_mod_id}">
								<LockEnvelope Value="0" />
							</ModulationTarget>
						</Volume>
						<ViewStateSesstionTrackWidth Value="93" />
						<CrossFadeState>
							<LomId Value="0" />
							<Manual Value="1" />
							<AutomationTarget Id="{crossfade_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
						</CrossFadeState>
						<SendsListWrapper LomId="0" />
					</Mixer>
					<MainSequencer>
						<LomId Value="0" />
						<LomIdView Value="0" />
						<IsExpanded Value="true" />
						<BreakoutIsExpanded Value="false" />
						<On>
							<LomId Value="0" />
							<Manual Value="true" />
							<AutomationTarget Id="{seq_on_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<MidiCCOnOffThresholds>
								<Min Value="64" />
								<Max Value="127" />
							</MidiCCOnOffThresholds>
						</On>
						<ModulationSourceCount Value="0" />
						<ParametersListWrapper LomId="0" />
						<Pointee Id="{seq_pointee_id}" />
						<LastSelectedTimeableIndex Value="0" />
						<LastSelectedClipEnvelopeIndex Value="0" />
						<LastPresetRef>
							<Value />
						</LastPresetRef>
						<LockedScripts />
						<IsFolded Value="false" />
						<ShouldShowPresetName Value="true" />
						<UserName Value="" />
						<Annotation Value="" />
						<SourceContext>
							<Value />
						</SourceContext>
						<MpePitchBendUsesTuning Value="true" />
						<ViewData Value="{{}}" />
						<ClipSlotList>
{clip_slots}						</ClipSlotList>
						<MonitoringEnum Value="1" />
						<Sample>
							<LomId Value="0" />
							<ArrangerAutomation>
								<Events />
								<AutomationTransformViewState>
									<IsTransformPending Value="false" />
									<TimeAndValueTransforms />
								</AutomationTransformViewState>
							</ArrangerAutomation>
							<ModulationSourceCount Value="0" />
						</Sample>
						<VolumeModulationTarget Id="{vol_mod_target_id}">
							<LockEnvelope Value="0" />
						</VolumeModulationTarget>
						<TranspositionModulationTarget Id="{trans_mod_target_id}">
							<LockEnvelope Value="0" />
						</TranspositionModulationTarget>
						<GrainSizeModulationTarget Id="{grain_mod_target_id}">
							<LockEnvelope Value="0" />
						</GrainSizeModulationTarget>
						<FluxModulationTarget Id="{flux_mod_target_id}">
							<LockEnvelope Value="0" />
						</FluxModulationTarget>
						<SampleOffsetModulationTarget Id="{offset_mod_target_id}">
							<LockEnvelope Value="0" />
						</SampleOffsetModulationTarget>
						<PitchViewScrollPosition Value="-1073741824" />
						<SampleOffsetModulationScrollPosition Value="-1073741824" />
						<Recorder>
							<IsArmed Value="false" />
							<TakeCounter Value="1" />
						</Recorder>
					</MainSequencer>
					<FreezeSequencer>
						<LomId Value="0" />
						<LomIdView Value="0" />
						<IsExpanded Value="true" />
						<BreakoutIsExpanded Value="false" />
						<On>
							<LomId Value="0" />
							<Manual Value="true" />
							<AutomationTarget Id="{freeze_on_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<MidiCCOnOffThresholds>
								<Min Value="64" />
								<Max Value="127" />
							</MidiCCOnOffThresholds>
						</On>
						<ModulationSourceCount Value="0" />
						<ParametersListWrapper LomId="0" />
						<Pointee Id="{freeze_pointee_id}" />
						<LastSelectedTimeableIndex Value="0" />
						<LastSelectedClipEnvelopeIndex Value="0" />
						<LastPresetRef>
							<Value />
						</LastPresetRef>
						<LockedScripts />
						<IsFolded Value="false" />
						<ShouldShowPresetName Value="true" />
						<UserName Value="" />
						<Annotation Value="" />
						<SourceContext>
							<Value />
						</SourceContext>
						<MpePitchBendUsesTuning Value="true" />
						<ViewData Value="{{}}" />
						<ClipSlotList />
						<MonitoringEnum Value="1" />
						<Sample>
							<LomId Value="0" />
							<ArrangerAutomation>
								<Events />
								<AutomationTransformViewState>
									<IsTransformPending Value="false" />
									<TimeAndValueTransforms />
								</AutomationTransformViewState>
							</ArrangerAutomation>
							<ModulationSourceCount Value="0" />
						</Sample>
						<VolumeModulationTarget Id="{freeze_vol_id}">
							<LockEnvelope Value="0" />
						</VolumeModulationTarget>
						<TranspositionModulationTarget Id="{freeze_trans_id}">
							<LockEnvelope Value="0" />
						</TranspositionModulationTarget>
						<GrainSizeModulationTarget Id="{freeze_grain_id}">
							<LockEnvelope Value="0" />
						</GrainSizeModulationTarget>
						<FluxModulationTarget Id="{freeze_flux_id}">
							<LockEnvelope Value="0" />
						</FluxModulationTarget>
						<SampleOffsetModulationTarget Id="{freeze_offset_id}">
							<LockEnvelope Value="0" />
						</SampleOffsetModulationTarget>
						<PitchViewScrollPosition Value="-1073741824" />
						<SampleOffsetModulationScrollPosition Value="-1073741824" />
						<Recorder>
							<IsArmed Value="false" />
							<TakeCounter Value="1" />
						</Recorder>
					</FreezeSequencer>
					<DeviceChain>
						<Devices />
						<SignalModulations />
					</DeviceChain>
				</DeviceChain>
			</AudioTrack>
"#,
        track_id = track_id,
        name = xml_escape(&track.name),
        color = track.color,
        clips_xml = clips_xml,
        clip_slots = clip_slots,
        automation_target_id = automation_target_id,
        pointee_id = pointee_id,
        speaker_id = ids.next(),
        pan_id = ids.next(),
        pan_mod_id = ids.next(),
        split_l_id = ids.next(),
        split_l_mod_id = ids.next(),
        split_r_id = ids.next(),
        split_r_mod_id = ids.next(),
        volume_id = ids.next(),
        volume_mod_id = ids.next(),
        crossfade_id = ids.next(),
        seq_on_id = ids.next(),
        seq_pointee_id = ids.next(),
        vol_mod_target_id = ids.next(),
        trans_mod_target_id = ids.next(),
        grain_mod_target_id = ids.next(),
        flux_mod_target_id = ids.next(),
        offset_mod_target_id = ids.next(),
        freeze_on_id = ids.next(),
        freeze_pointee_id = ids.next(),
        freeze_vol_id = ids.next(),
        freeze_trans_id = ids.next(),
        freeze_grain_id = ids.next(),
        freeze_flux_id = ids.next(),
        freeze_offset_id = ids.next(),
    )
}

fn generate_audio_clip(clip: &ClipPlacement, ids: &mut IdAllocator) -> String {
    let clip_id = ids.next();
    let sample = &clip.sample;

    // Calculate sample duration in samples
    let default_duration = (sample.duration_secs * sample.sample_rate as f64) as u64;

    format!(
        r#"									<AudioClip Id="{clip_id}" Time="{start_beat}">
										<LomId Value="0" />
										<LomIdView Value="0" />
										<CurrentStart Value="{start_beat}" />
										<CurrentEnd Value="{end_beat}" />
										<Loop>
											<LoopStart Value="0" />
											<LoopEnd Value="{duration_beats}" />
											<StartRelative Value="0" />
											<LoopOn Value="false" />
											<OutMarker Value="{duration_beats}" />
											<HiddenLoopStart Value="0" />
											<HiddenLoopEnd Value="{duration_beats}" />
										</Loop>
										<Name Value="{name}" />
										<Annotation Value="" />
										<Color Value="-1" />
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
											<RightTime Value="0" />
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
											<JumpIndexA Value="0" />
											<JumpIndexB Value="0" />
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
										<TakeId Value="0" />
										<IsInKey Value="false" />
										<ScaleInformation>
											<Root Value="0" />
											<Name Value="0" />
										</ScaleInformation>
										<SampleRef>
											<FileRef>
												<RelativePathType Value="0" />
												<RelativePath Value="" />
												<Path Value="{path}" />
												<Type Value="1" />
												<LivePackName Value="" />
												<LivePackId Value="" />
												<OriginalFileSize Value="0" />
												<OriginalCrc Value="0" />
												<SourceHint Value="" />
											</FileRef>
											<LastModDate Value="0" />
											<SourceContext />
											<SampleUsageHint Value="0" />
											<DefaultDuration Value="{default_duration}" />
											<DefaultSampleRate Value="{sample_rate}" />
											<SamplesToAutoWarp Value="0" />
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
											<WarpMarker Id="1" SecTime="{duration_secs}" BeatTime="{duration_beats}" />
										</WarpMarkers>
										<SavedWarpMarkersForStretched />
										<MarkersGenerated Value="true" />
										<IsSongTempoLeader Value="false" />
									</AudioClip>
"#,
        clip_id = clip_id,
        start_beat = clip.start_beat,
        end_beat = clip.start_beat + clip.duration_beats,
        duration_beats = clip.duration_beats,
        name = xml_escape(&sample.name),
        path = xml_escape(&sample.path),
        default_duration = default_duration,
        sample_rate = sample.sample_rate,
        duration_secs = sample.duration_secs,
    )
}

fn generate_master_track(ids: &mut IdAllocator) -> String {
    let track_id = ids.next();
    let automation_target_id = ids.next();
    let pointee_id = ids.next();

    format!(
        r#"			<MasterTrack Id="{track_id}">
				<LomId Value="0" />
				<LomIdView Value="0" />
				<IsContentSelectedInDocument Value="false" />
				<PreferredContentViewMode Value="0" />
				<TrackDelay>
					<Value Value="0" />
					<IsValueSampleBased Value="false" />
				</TrackDelay>
				<Name>
					<EffectiveName Value="Master" />
					<UserName Value="" />
					<Annotation Value="" />
					<MemorizedFirstClipName Value="" />
				</Name>
				<Color Value="-1" />
				<AutomationEnvelopes>
					<Envelopes />
				</AutomationEnvelopes>
				<TrackGroupId Value="-1" />
				<TrackUnfolded Value="false" />
				<DevicesListWrapper LomId="0" />
				<ClipSlotsListWrapper LomId="0" />
				<ArrangementClipsListWrapper LomId="0" />
				<TakeLanesListWrapper LomId="0" />
				<ViewData Value="{{}}" />
				<TakeLanes>
					<TakeLanes />
					<AreTakeLanesFolded Value="true" />
				</TakeLanes>
				<LinkedTrackGroupId Value="-1" />
				<DeviceChain>
					<AutomationLanes>
						<AutomationLanes>
							<AutomationLane Id="0">
								<SelectedDevice Value="0" />
								<SelectedEnvelope Value="0" />
								<IsContentSelectedInDocument Value="false" />
								<LaneHeight Value="68" />
							</AutomationLane>
						</AutomationLanes>
						<AreAdditionalAutomationLanesFolded Value="false" />
					</AutomationLanes>
					<ClipEnvelopeChooserViewState>
						<SelectedDevice Value="0" />
						<SelectedEnvelope Value="0" />
						<PreferModulationVisible Value="false" />
					</ClipEnvelopeChooserViewState>
					<AudioInputRouting>
						<Target Value="AudioIn/External/S0" />
						<UpperDisplayString Value="Ext. In" />
						<LowerDisplayString Value="1/2" />
						<MpeSettings>
							<ZoneType Value="0" />
							<FirstNoteChannel Value="1" />
							<LastNoteChannel Value="15" />
						</MpeSettings>
						<MpePitchBendUsesTuning Value="true" />
					</AudioInputRouting>
					<MidiInputRouting>
						<Target Value="MidiIn/External.All/-1" />
						<UpperDisplayString Value="Ext: All Ins" />
						<LowerDisplayString Value="" />
						<MpeSettings>
							<ZoneType Value="0" />
							<FirstNoteChannel Value="1" />
							<LastNoteChannel Value="15" />
						</MpeSettings>
						<MpePitchBendUsesTuning Value="true" />
					</MidiInputRouting>
					<AudioOutputRouting>
						<Target Value="AudioOut/External/S0" />
						<UpperDisplayString Value="Ext. Out" />
						<LowerDisplayString Value="1/2" />
						<MpeSettings>
							<ZoneType Value="0" />
							<FirstNoteChannel Value="1" />
							<LastNoteChannel Value="15" />
						</MpeSettings>
						<MpePitchBendUsesTuning Value="true" />
					</AudioOutputRouting>
					<MidiOutputRouting>
						<Target Value="MidiOut/None" />
						<UpperDisplayString Value="None" />
						<LowerDisplayString Value="" />
						<MpeSettings>
							<ZoneType Value="0" />
							<FirstNoteChannel Value="1" />
							<LastNoteChannel Value="15" />
						</MpeSettings>
						<MpePitchBendUsesTuning Value="true" />
					</MidiOutputRouting>
					<Mixer>
						<LomId Value="0" />
						<LomIdView Value="0" />
						<IsExpanded Value="true" />
						<BreakoutIsExpanded Value="false" />
						<On>
							<LomId Value="0" />
							<Manual Value="true" />
							<AutomationTarget Id="{automation_target_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<MidiCCOnOffThresholds>
								<Min Value="64" />
								<Max Value="127" />
							</MidiCCOnOffThresholds>
						</On>
						<ModulationSourceCount Value="0" />
						<ParametersListWrapper LomId="0" />
						<Pointee Id="{pointee_id}" />
						<LastSelectedTimeableIndex Value="0" />
						<LastSelectedClipEnvelopeIndex Value="0" />
						<LastPresetRef>
							<Value />
						</LastPresetRef>
						<LockedScripts />
						<IsFolded Value="false" />
						<ShouldShowPresetName Value="false" />
						<UserName Value="" />
						<Annotation Value="" />
						<SourceContext>
							<Value />
						</SourceContext>
						<MpePitchBendUsesTuning Value="true" />
						<ViewData Value="{{}}" />
						<Sends />
						<Speaker>
							<LomId Value="0" />
							<Manual Value="true" />
							<AutomationTarget Id="{speaker_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<MidiCCOnOffThresholds>
								<Min Value="64" />
								<Max Value="127" />
							</MidiCCOnOffThresholds>
						</Speaker>
						<SoloSink Value="false" />
						<PanMode Value="0" />
						<Pan>
							<LomId Value="0" />
							<Manual Value="0" />
							<MidiControllerRange>
								<Min Value="-1" />
								<Max Value="1" />
							</MidiControllerRange>
							<AutomationTarget Id="{pan_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<ModulationTarget Id="{pan_mod_id}">
								<LockEnvelope Value="0" />
							</ModulationTarget>
						</Pan>
						<SplitStereoPanL>
							<LomId Value="0" />
							<Manual Value="-1" />
							<MidiControllerRange>
								<Min Value="-1" />
								<Max Value="1" />
							</MidiControllerRange>
							<AutomationTarget Id="{split_l_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<ModulationTarget Id="{split_l_mod_id}">
								<LockEnvelope Value="0" />
							</ModulationTarget>
						</SplitStereoPanL>
						<SplitStereoPanR>
							<LomId Value="0" />
							<Manual Value="1" />
							<MidiControllerRange>
								<Min Value="-1" />
								<Max Value="1" />
							</MidiControllerRange>
							<AutomationTarget Id="{split_r_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<ModulationTarget Id="{split_r_mod_id}">
								<LockEnvelope Value="0" />
							</ModulationTarget>
						</SplitStereoPanR>
						<Volume>
							<LomId Value="0" />
							<Manual Value="1" />
							<MidiControllerRange>
								<Min Value="0.0003162277571" />
								<Max Value="1.99526226520538" />
							</MidiControllerRange>
							<AutomationTarget Id="{volume_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<ModulationTarget Id="{volume_mod_id}">
								<LockEnvelope Value="0" />
							</ModulationTarget>
						</Volume>
						<ViewStateSesstionTrackWidth Value="93" />
						<CrossFadeState>
							<LomId Value="0" />
							<Manual Value="1" />
							<AutomationTarget Id="{crossfade_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
						</CrossFadeState>
						<Tempo>
							<LomId Value="0" />
							<Manual Value="130" />
							<MidiControllerRange>
								<Min Value="60" />
								<Max Value="200" />
							</MidiControllerRange>
							<AutomationTarget Id="{tempo_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<ModulationTarget Id="{tempo_mod_id}">
								<LockEnvelope Value="0" />
							</ModulationTarget>
						</Tempo>
						<TimeSignature>
							<LomId Value="0" />
							<Manual Value="201" />
							<AutomationTarget Id="{time_sig_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
						</TimeSignature>
						<GlobalGrooveAmount>
							<LomId Value="0" />
							<Manual Value="1" />
							<MidiControllerRange>
								<Min Value="0" />
								<Max Value="1.30999994277954" />
							</MidiControllerRange>
							<AutomationTarget Id="{groove_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<ModulationTarget Id="{groove_mod_id}">
								<LockEnvelope Value="0" />
							</ModulationTarget>
						</GlobalGrooveAmount>
						<SendsListWrapper LomId="0" />
					</Mixer>
					<DeviceChain>
						<Devices />
						<SignalModulations />
					</DeviceChain>
				</DeviceChain>
			</MasterTrack>"#,
        track_id = track_id,
        automation_target_id = automation_target_id,
        pointee_id = pointee_id,
        speaker_id = ids.next(),
        pan_id = ids.next(),
        pan_mod_id = ids.next(),
        split_l_id = ids.next(),
        split_l_mod_id = ids.next(),
        split_r_id = ids.next(),
        split_r_mod_id = ids.next(),
        volume_id = ids.next(),
        volume_mod_id = ids.next(),
        crossfade_id = ids.next(),
        tempo_id = ids.next(),
        tempo_mod_id = ids.next(),
        time_sig_id = ids.next(),
        groove_id = ids.next(),
        groove_mod_id = ids.next(),
    )
}

fn generate_prehear_track(ids: &mut IdAllocator) -> String {
    let track_id = ids.next();
    let automation_target_id = ids.next();
    let pointee_id = ids.next();

    format!(
        r#"			<PreHearTrack Id="{track_id}">
				<LomId Value="0" />
				<LomIdView Value="0" />
				<IsContentSelectedInDocument Value="false" />
				<PreferredContentViewMode Value="0" />
				<TrackDelay>
					<Value Value="0" />
					<IsValueSampleBased Value="false" />
				</TrackDelay>
				<Name>
					<EffectiveName Value="Preview" />
					<UserName Value="" />
					<Annotation Value="" />
					<MemorizedFirstClipName Value="" />
				</Name>
				<Color Value="-1" />
				<AutomationEnvelopes>
					<Envelopes />
				</AutomationEnvelopes>
				<TrackGroupId Value="-1" />
				<TrackUnfolded Value="false" />
				<DevicesListWrapper LomId="0" />
				<ClipSlotsListWrapper LomId="0" />
				<ArrangementClipsListWrapper LomId="0" />
				<TakeLanesListWrapper LomId="0" />
				<ViewData Value="{{}}" />
				<TakeLanes>
					<TakeLanes />
					<AreTakeLanesFolded Value="true" />
				</TakeLanes>
				<LinkedTrackGroupId Value="-1" />
				<DeviceChain>
					<AutomationLanes>
						<AutomationLanes>
							<AutomationLane Id="0">
								<SelectedDevice Value="0" />
								<SelectedEnvelope Value="0" />
								<IsContentSelectedInDocument Value="false" />
								<LaneHeight Value="68" />
							</AutomationLane>
						</AutomationLanes>
						<AreAdditionalAutomationLanesFolded Value="false" />
					</AutomationLanes>
					<ClipEnvelopeChooserViewState>
						<SelectedDevice Value="0" />
						<SelectedEnvelope Value="0" />
						<PreferModulationVisible Value="false" />
					</ClipEnvelopeChooserViewState>
					<AudioInputRouting>
						<Target Value="AudioIn/External/S0" />
						<UpperDisplayString Value="Ext. In" />
						<LowerDisplayString Value="1/2" />
						<MpeSettings>
							<ZoneType Value="0" />
							<FirstNoteChannel Value="1" />
							<LastNoteChannel Value="15" />
						</MpeSettings>
						<MpePitchBendUsesTuning Value="true" />
					</AudioInputRouting>
					<MidiInputRouting>
						<Target Value="MidiIn/External.All/-1" />
						<UpperDisplayString Value="Ext: All Ins" />
						<LowerDisplayString Value="" />
						<MpeSettings>
							<ZoneType Value="0" />
							<FirstNoteChannel Value="1" />
							<LastNoteChannel Value="15" />
						</MpeSettings>
						<MpePitchBendUsesTuning Value="true" />
					</MidiInputRouting>
					<AudioOutputRouting>
						<Target Value="AudioOut/External/S0" />
						<UpperDisplayString Value="Ext. Out" />
						<LowerDisplayString Value="1/2" />
						<MpeSettings>
							<ZoneType Value="0" />
							<FirstNoteChannel Value="1" />
							<LastNoteChannel Value="15" />
						</MpeSettings>
						<MpePitchBendUsesTuning Value="true" />
					</AudioOutputRouting>
					<MidiOutputRouting>
						<Target Value="MidiOut/None" />
						<UpperDisplayString Value="None" />
						<LowerDisplayString Value="" />
						<MpeSettings>
							<ZoneType Value="0" />
							<FirstNoteChannel Value="1" />
							<LastNoteChannel Value="15" />
						</MpeSettings>
						<MpePitchBendUsesTuning Value="true" />
					</MidiOutputRouting>
					<Mixer>
						<LomId Value="0" />
						<LomIdView Value="0" />
						<IsExpanded Value="true" />
						<BreakoutIsExpanded Value="false" />
						<On>
							<LomId Value="0" />
							<Manual Value="true" />
							<AutomationTarget Id="{automation_target_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<MidiCCOnOffThresholds>
								<Min Value="64" />
								<Max Value="127" />
							</MidiCCOnOffThresholds>
						</On>
						<ModulationSourceCount Value="0" />
						<ParametersListWrapper LomId="0" />
						<Pointee Id="{pointee_id}" />
						<LastSelectedTimeableIndex Value="0" />
						<LastSelectedClipEnvelopeIndex Value="0" />
						<LastPresetRef>
							<Value />
						</LastPresetRef>
						<LockedScripts />
						<IsFolded Value="false" />
						<ShouldShowPresetName Value="false" />
						<UserName Value="" />
						<Annotation Value="" />
						<SourceContext>
							<Value />
						</SourceContext>
						<MpePitchBendUsesTuning Value="true" />
						<ViewData Value="{{}}" />
						<Sends />
						<Speaker>
							<LomId Value="0" />
							<Manual Value="true" />
							<AutomationTarget Id="{speaker_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<MidiCCOnOffThresholds>
								<Min Value="64" />
								<Max Value="127" />
							</MidiCCOnOffThresholds>
						</Speaker>
						<SoloSink Value="false" />
						<PanMode Value="0" />
						<Pan>
							<LomId Value="0" />
							<Manual Value="0" />
							<MidiControllerRange>
								<Min Value="-1" />
								<Max Value="1" />
							</MidiControllerRange>
							<AutomationTarget Id="{pan_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<ModulationTarget Id="{pan_mod_id}">
								<LockEnvelope Value="0" />
							</ModulationTarget>
						</Pan>
						<SplitStereoPanL>
							<LomId Value="0" />
							<Manual Value="-1" />
							<MidiControllerRange>
								<Min Value="-1" />
								<Max Value="1" />
							</MidiControllerRange>
							<AutomationTarget Id="{split_l_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<ModulationTarget Id="{split_l_mod_id}">
								<LockEnvelope Value="0" />
							</ModulationTarget>
						</SplitStereoPanL>
						<SplitStereoPanR>
							<LomId Value="0" />
							<Manual Value="1" />
							<MidiControllerRange>
								<Min Value="-1" />
								<Max Value="1" />
							</MidiControllerRange>
							<AutomationTarget Id="{split_r_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<ModulationTarget Id="{split_r_mod_id}">
								<LockEnvelope Value="0" />
							</ModulationTarget>
						</SplitStereoPanR>
						<Volume>
							<LomId Value="0" />
							<Manual Value="1" />
							<MidiControllerRange>
								<Min Value="0.0003162277571" />
								<Max Value="1.99526226520538" />
							</MidiControllerRange>
							<AutomationTarget Id="{volume_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
							<ModulationTarget Id="{volume_mod_id}">
								<LockEnvelope Value="0" />
							</ModulationTarget>
						</Volume>
						<ViewStateSesstionTrackWidth Value="93" />
						<CrossFadeState>
							<LomId Value="0" />
							<Manual Value="1" />
							<AutomationTarget Id="{crossfade_id}">
								<LockEnvelope Value="0" />
							</AutomationTarget>
						</CrossFadeState>
						<SendsListWrapper LomId="0" />
					</Mixer>
					<DeviceChain>
						<Devices />
						<SignalModulations />
					</DeviceChain>
				</DeviceChain>
			</PreHearTrack>"#,
        track_id = track_id,
        automation_target_id = automation_target_id,
        pointee_id = pointee_id,
        speaker_id = ids.next(),
        pan_id = ids.next(),
        pan_mod_id = ids.next(),
        split_l_id = ids.next(),
        split_l_mod_id = ids.next(),
        split_r_id = ids.next(),
        split_r_mod_id = ids.next(),
        volume_id = ids.next(),
        volume_mod_id = ids.next(),
        crossfade_id = ids.next(),
    )
}

/// Generate a techno track from sample paths
pub fn generate_techno_als(
    output_path: &Path,
    kick_path: &str,
    clap_path: &str,
    hat_path: &str,
    bpm: f64,
) -> Result<(), String> {
    let config = TechnoConfig {
        bpm,
        kick: SampleInfo {
            path: kick_path.to_string(),
            name: Path::new(kick_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Kick".to_string()),
            duration_secs: 0.5,
            sample_rate: 44100,
            file_size: std::fs::metadata(kick_path).map(|m| m.len()).unwrap_or(0),
            bpm: Some(bpm),
        },
        clap: SampleInfo {
            path: clap_path.to_string(),
            name: Path::new(clap_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Clap".to_string()),
            duration_secs: 0.3,
            sample_rate: 44100,
            file_size: std::fs::metadata(clap_path).map(|m| m.len()).unwrap_or(0),
            bpm: Some(bpm),
        },
        hat: SampleInfo {
            path: hat_path.to_string(),
            name: Path::new(hat_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Hat".to_string()),
            duration_secs: 0.1,
            sample_rate: 44100,
            file_size: std::fs::metadata(hat_path).map(|m| m.len()).unwrap_or(0),
            bpm: Some(bpm),
        },
    };

    let tracks = config.generate_arrangement();
    generate_als(output_path, &tracks, bpm)
}


/// Generate an empty Ableton project using the embedded template.
/// This is guaranteed to open in Ableton Live 12.x without errors.
pub fn generate_empty_als(output_path: &Path) -> Result<(), String> {
    // Decompress the embedded template
    let mut decoder = GzDecoder::new(EMPTY_PROJECT_TEMPLATE);
    let mut xml = String::new();
    decoder
        .read_to_string(&mut xml)
        .map_err(|e| format!("Failed to decompress template: {}", e))?;

    // Re-compress and write to output
    let file = File::create(output_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder
        .write_all(xml.as_bytes())
        .map_err(|e| format!("Failed to write compressed data: {}", e))?;
    encoder
        .finish()
        .map_err(|e| format!("Failed to finish compression: {}", e))?;

    Ok(())
}

/// Generate an ALS file using the embedded template (same approach as generate_true_techno).
/// This is guaranteed to produce valid ALS files that Ableton can open.
pub fn generate_als_from_template(
    output_path: &Path,
    tracks: &[TrackInfo],
    bpm: f64,
) -> Result<(), String> {
    use regex::Regex;
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct Ids {
        next_id: AtomicU32,
        used: std::sync::Mutex<HashSet<u32>>,
    }
    impl Ids {
        fn new(start: u32) -> Self {
            Self { next_id: AtomicU32::new(start), used: std::sync::Mutex::new(HashSet::new()) }
        }
        fn alloc(&self) -> u32 {
            loop {
                let id = self.next_id.fetch_add(1, Ordering::SeqCst);
                let mut used = self.used.lock().unwrap();
                if !used.contains(&id) {
                    used.insert(id);
                    return id;
                }
            }
        }
        fn reserve(&self, id: u32) {
            self.used.lock().unwrap().insert(id);
        }
        fn max_id(&self) -> u32 {
            self.next_id.load(Ordering::SeqCst)
        }
    }

    // Step 1: Generate empty ALS from embedded template
    generate_empty_als(output_path)?;

    // Step 2: Decompress and parse
    let file = File::open(output_path).map_err(|e| format!("Failed to open template: {}", e))?;
    let mut decoder = GzDecoder::new(file);
    let mut xml = String::new();
    decoder.read_to_string(&mut xml).map_err(|e| format!("Failed to decompress: {}", e))?;

    let ids = Ids::new(1_000_000);

    // Reserve all existing IDs from template
    let id_re = Regex::new(r#"Id="(\d+)""#).unwrap();
    for cap in id_re.captures_iter(&xml) {
        if let Ok(id) = cap[1].parse::<u32>() {
            ids.reserve(id);
        }
    }

    // Step 3: Extract the audio track template from the valid ALS
    let track_start = xml.find("<AudioTrack").ok_or("No AudioTrack in template")?;
    let track_end = xml.find("</AudioTrack>").ok_or("No AudioTrack end in template")? + "</AudioTrack>".len();
    let audio_track_template = xml[track_start..track_end].to_string();

    // Step 4: Create tracks by cloning the template
    let mut all_tracks_xml = Vec::new();
    let name_re = Regex::new(r#"<EffectiveName Value="[^"]*" />"#).unwrap();
    let username_re = Regex::new(r#"(<EffectiveName Value="[^"]*" />\s*<UserName Value=")[^"]*(" />)"#).unwrap();
    let color_re = Regex::new(r#"<Color Value="\d+" />"#).unwrap();
    for track in tracks {
        let mut t = audio_track_template.clone();

        // Replace all IDs with fresh unique ones
        let mut replacements: Vec<(String, String)> = Vec::new();
        for cap in id_re.captures_iter(&t) {
            let old = format!(r#"Id="{}""#, &cap[1]);
            let new = format!(r#"Id="{}""#, ids.alloc());
            replacements.push((old, new));
        }
        for (old, new) in replacements {
            t = t.replacen(&old, &new, 1);
        }

        // Set name
        t = name_re.replace(&t, format!(r#"<EffectiveName Value="{}" />"#, xml_escape(&track.name))).to_string();
        t = username_re.replace(&t, format!(r#"${{1}}{}${{2}}"#, xml_escape(&track.name))).to_string();

        // Set color
        t = color_re.replace_all(&t, format!(r#"<Color Value="{}" />"#, track.color)).to_string();

        // Create clips
        let mut clips_xml = Vec::new();
        for clip in &track.clips {
            let clip_id = ids.alloc();
            let sample = &clip.sample;
            let start_beat = clip.start_beat;
            let end_beat = clip.start_beat + clip.duration_beats;
            let loop_bars = sample.loop_bars(bpm);
            let loop_beats = (loop_bars as f64) * 4.0;
            let loop_beats = if clip.duration_beats < loop_beats { clip.duration_beats } else { loop_beats };
            let warp_sec = (loop_beats * 60.0) / bpm;

            clips_xml.push(format!(
                r#"<AudioClip Id="{clip_id}" Time="{start_beat}">
											<LomId Value="0" />
											<LomIdView Value="0" />
											<CurrentStart Value="{start_beat}" />
											<CurrentEnd Value="{end_beat}" />
											<Loop>
												<LoopStart Value="0" />
												<LoopEnd Value="{loop_beats}" />
												<StartRelative Value="0" />
												<LoopOn Value="true" />
												<OutMarker Value="{loop_beats}" />
												<HiddenLoopStart Value="0" />
												<HiddenLoopEnd Value="{loop_beats}" />
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
												<RightTime Value="{end_beat}" />
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
											<SampleRef>
												<FileRef>
													<RelativePathType Value="0" />
													<RelativePath Value="" />
													<Path Value="{path}" />
													<Type Value="2" />
													<LivePackName Value="" />
													<LivePackId Value="" />
													<OriginalFileSize Value="{file_size}" />
													<OriginalCrc Value="0" />
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
																<LivePackName Value="" />
																<LivePackId Value="" />
																<OriginalFileSize Value="{file_size}" />
																<OriginalCrc Value="0" />
															</FileRef>
														</OriginalFileRef>
														<BrowserContentPath Value="" />
														<LocalFiltersJson Value="" />
													</SourceContext>
												</SourceContext>
												<SampleUsageHint Value="0" />
												<DefaultDuration Value="{loop_beats}" />
												<DefaultSampleRate Value="44100" />
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
												<CrossfadeLength Value="0" />
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
											<MarkerDensity Value="2" />
											<AutoWarpTolerance Value="4" />
											<WarpMarkers>
												<WarpMarker Id="0" SecTime="0" BeatTime="0" />
												<WarpMarker Id="1" SecTime="{warp_sec}" BeatTime="{loop_beats}" />
											</WarpMarkers>
											<SavedWarpMarkersForStretched />
											<MarkersGenerated Value="true" />
											<IsSongTempoLeader Value="false" />
										</AudioClip>"#,
                clip_id = clip_id,
                start_beat = start_beat,
                end_beat = end_beat,
                loop_beats = loop_beats,
                name = xml_escape(&sample.name),
                color = track.color,
                path = xml_escape(&sample.path),
                file_size = sample.file_size,
                warp_sec = warp_sec,
            ));
        }

        // Inject clips into track
        let clips_joined = clips_xml.join("\n");
        t = t.replacen(
            "<Events />",
            &format!("<Events>\n{}\n\t\t\t\t\t\t\t\t\t\t\t\t\t</Events>", clips_joined),
            1,
        );

        all_tracks_xml.push(t);
    }

    // Step 5: Replace the template's audio track with our tracks
    let before_track = &xml[..track_start];
    let after_track = &xml[track_end..];
    let joined = all_tracks_xml.join("\n\t\t\t");
    let mut xml = format!("{}{}{}", before_track, joined, after_track);

    // Update NextPointeeId
    let next_id = ids.max_id() + 1000;
    let next_id_re = Regex::new(r#"<NextPointeeId Value="\d+" />"#).unwrap();
    xml = next_id_re.replace(&xml, format!(r#"<NextPointeeId Value="{}" />"#, next_id)).to_string();

    // Set tempo
    let tempo_re = Regex::new(r#"<Tempo>\s*<LomId Value="0" />\s*<Manual Value="[^"]+" />"#).unwrap();
    xml = tempo_re.replace(&xml, format!(r#"<Tempo>
							<LomId Value="0" />
							<Manual Value="{}" />"#, bpm)).to_string();

    let tempo_event_re = Regex::new(r#"<FloatEvent Id="\d+" Time="-63072000" Value="[^"]+" />"#).unwrap();
    xml = tempo_event_re.replace(&xml, format!(r#"<FloatEvent Id="0" Time="-63072000" Value="{}" />"#, bpm)).to_string();

    // Step 6: Write compressed output
    let output_file = File::create(output_path).map_err(|e| format!("Failed to create file: {}", e))?;
    let mut encoder = GzEncoder::new(output_file, Compression::default());
    encoder.write_all(xml.as_bytes()).map_err(|e| format!("Failed to write: {}", e))?;
    encoder.finish().map_err(|e| format!("Failed to compress: {}", e))?;

    Ok(())
}

/// Generate an empty Ableton project with a specific BPM using the embedded template.
pub fn generate_empty_als_with_bpm(output_path: &Path, bpm: f64) -> Result<(), String> {
    // Decompress the embedded template
    let mut decoder = GzDecoder::new(EMPTY_PROJECT_TEMPLATE);
    let mut xml = String::new();
    decoder
        .read_to_string(&mut xml)
        .map_err(|e| format!("Failed to decompress template: {}", e))?;

    // Replace the default tempo (120) with the requested BPM
    // The template has: <Manual Value="120" /> in the Tempo section
    // We need to be careful to only replace the tempo value, not other 120s
    let xml = xml.replace(
        r#"<Tempo>
						<LomId Value="0" />
						<Manual Value="120" />"#,
        &format!(
            r#"<Tempo>
						<LomId Value="0" />
						<Manual Value="{}" />"#,
            bpm
        ),
    );

    // Re-compress and write to output
    let file = File::create(output_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder
        .write_all(xml.as_bytes())
        .map_err(|e| format!("Failed to write compressed data: {}", e))?;
    encoder
        .finish()
        .map_err(|e| format!("Failed to finish compression: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;
    use flate2::read::GzDecoder;

    #[test]
    fn test_ableton_version_default() {
        let v = AbletonVersion::default();
        assert_eq!(v.major, 12);
        assert_eq!(v.creator, "Ableton Live 12.0");
    }

    #[test]
    fn test_ableton_version_detect_from_path_missing() {
        let v = AbletonVersion::detect_from_path("/non/existent/path");
        assert!(v.is_none());
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("hello"), "hello");
        assert_eq!(xml_escape("at&t"), "at&amp;t");
        assert_eq!(xml_escape("quote \""), "quote &quot;");
        assert_eq!(xml_escape("single '"), "single &apos;");
        assert_eq!(xml_escape("tag <br>"), "tag &lt;br&gt;");
        assert_eq!(xml_escape("multiple &&"), "multiple &amp;&amp;");
        assert_eq!(xml_escape("mixed <\"&'>"), "mixed &lt;&quot;&amp;&apos;&gt;");
    }

    #[test]
    fn test_id_allocator() {
        let mut ids = IdAllocator::new(100);
        assert_eq!(ids.next(), 100);
        assert_eq!(ids.next(), 101);
        assert_eq!(ids.next(), 102);
    }

    #[test]
    fn test_ableton_version_parsing() {
        let v = AbletonVersion::parse_version_string("12.3.7 (2026-03-30_c92a51f028)").unwrap();
        assert_eq!(v.major, 12);
        assert_eq!(v.minor, 3);
        assert_eq!(v.patch, 7);
        
        let v2 = AbletonVersion::parse_version_string("11.0.12").unwrap();
        assert_eq!(v2.major, 11);
        assert_eq!(v2.minor, 0);
        assert_eq!(v2.patch, 12);

        let v3 = AbletonVersion::parse_version_string("12.1").unwrap();
        assert_eq!(v3.major, 12);
        assert_eq!(v3.minor, 1);
        assert_eq!(v3.patch, 0);
        
        assert!(AbletonVersion::parse_version_string("invalid").is_none());
        assert!(AbletonVersion::parse_version_string("").is_none());
    }

    #[test]
    fn test_section_bars() {
        assert_eq!(Section::Intro.bars(), 16);
        assert_eq!(Section::Buildup.bars(), 16);
        assert_eq!(Section::Drop.bars(), 32);
        assert_eq!(Section::Breakdown.bars(), 16);
        assert_eq!(Section::Drop2.bars(), 32);
        assert_eq!(Section::Outro.bars(), 16);
    }

    #[test]
    fn test_sample_info_loop_bars() {
        // 120 bpm = 2.0s per bar (4 beats)
        let sample = SampleInfo {
            path: "test.wav".to_string(),
            name: "test".to_string(),
            duration_secs: 2.0,
            sample_rate: 44100,
            file_size: 1000,
            bpm: Some(120.0),
        };
        assert_eq!(sample.loop_bars(120.0), 1);

        // 4.0s = 2 bars
        let sample2 = sample.clone();
        let mut sample2 = sample2;
        sample2.duration_secs = 4.0;
        assert_eq!(sample2.loop_bars(120.0), 2);

        // 8.0s = 4 bars
        let mut sample3 = sample.clone();
        sample3.duration_secs = 8.0;
        assert_eq!(sample3.loop_bars(120.0), 4);

        // 16.0s = 8 bars
        let mut sample4 = sample.clone();
        sample4.duration_secs = 16.0;
        assert_eq!(sample4.loop_bars(120.0), 8);
        
        // Very short sample
        let mut sample_short = sample.clone();
        sample_short.duration_secs = 0.1;
        assert_eq!(sample_short.loop_bars(120.0), 1);

        // Very long sample
        let mut sample_long = sample.clone();
        sample_long.duration_secs = 120.0;
        assert_eq!(sample_long.loop_bars(120.0), 16);
        
        // No BPM provided, use project BPM
        let mut sample_no_bpm = sample.clone();
        sample_no_bpm.bpm = None;
        sample_no_bpm.duration_secs = 2.0;
        assert_eq!(sample_no_bpm.loop_bars(120.0), 1);
    }

    #[test]
    fn test_techno_arrangement_logic() {
        let kick = SampleInfo {
            path: "kick.wav".to_string(),
            name: "Kick".to_string(),
            duration_secs: 0.5,
            sample_rate: 44100,
            file_size: 1000,
            bpm: None,
        };
        let clap = SampleInfo {
            path: "clap.wav".to_string(),
            name: "Clap".to_string(),
            duration_secs: 0.5,
            sample_rate: 44100,
            file_size: 1000,
            bpm: None,
        };
        let hat = SampleInfo {
            path: "hat.wav".to_string(),
            name: "Hat".to_string(),
            duration_secs: 0.5,
            sample_rate: 44100,
            file_size: 1000,
            bpm: None,
        };

        let config = TechnoConfig {
            bpm: 130.0,
            kick,
            clap,
            hat,
        };

        let tracks = config.generate_arrangement();
        assert_eq!(tracks.len(), 3);
        assert_eq!(tracks[0].name, "Kick");
        assert_eq!(tracks[1].name, "Clap");
        assert_eq!(tracks[2].name, "Hat");

        let kick_clips = &tracks[0].clips;
        let clap_clips = &tracks[1].clips;
        let hat_clips = &tracks[2].clips;

        // Intro: bars 0-16 (beats 0-64)
        // Kick every 4 bars: 0, 16, 32, 48
        assert!(kick_clips.iter().any(|c| c.start_beat == 0.0));
        assert!(kick_clips.iter().any(|c| c.start_beat == 16.0));
        assert!(kick_clips.iter().any(|c| c.start_beat == 32.0));
        assert!(kick_clips.iter().any(|c| c.start_beat == 48.0));
        assert!(!kick_clips.iter().any(|c| c.start_beat == 4.0)); // Should NOT have kick here

        // Buildup: bars 16-32 (beats 64-128)
        // Kick on every beat
        for b in 64..128 {
            assert!(kick_clips.iter().any(|c| c.start_beat == b as f64), "Missing buildup kick at beat {}", b);
        }
        // Clap on 2 and 4 (beats 65, 67, 69, 71...)
        assert!(clap_clips.iter().any(|c| c.start_beat == 65.0));
        assert!(clap_clips.iter().any(|c| c.start_beat == 67.0));

        // Drop: bars 32-64 (beats 128-256)
        // Hats on 16th notes
        assert!(hat_clips.iter().any(|c| c.start_beat == 128.0));
        assert!(hat_clips.iter().any(|c| c.start_beat == 128.25));
        assert!(hat_clips.iter().any(|c| c.start_beat == 128.5));
        assert!(hat_clips.iter().any(|c| c.start_beat == 128.75));

        // Breakdown: bars 64-80 (beats 256-320)
        // No kick
        assert!(!kick_clips.iter().any(|c| c.start_beat >= 256.0 && c.start_beat < 320.0));
        // Sparse clap at end of 4-bar phrases (bar 67, 71, 75, 79)
        // Bar 67 is beats 268-272. Sparse clap on beat 3 of the bar = beat 271.
        assert!(clap_clips.iter().any(|c| c.start_beat == 271.0));
    }

    #[test]
    fn test_generate_empty_als() {
        let output = std::env::temp_dir().join("empty_test.als");
        let result = generate_empty_als(&output);
        assert!(result.is_ok());
        assert!(output.exists());

        // Verify it's a valid gzip and contains Ableton tag
        let file = File::open(&output).unwrap();
        let mut decoder = GzDecoder::new(file);
        let mut xml = String::new();
        decoder.read_to_string(&mut xml).expect("Failed to decompress");
        assert!(xml.contains("<Ableton"));
        
        fs::remove_file(output).ok();
    }

    #[test]
    fn test_generate_empty_als_with_bpm() {
        let output = std::env::temp_dir().join("empty_bpm_test.als");
        let result = generate_empty_als_with_bpm(&output, 145.0);
        assert!(result.is_ok());

        let file = File::open(&output).unwrap();
        let mut decoder = GzDecoder::new(file);
        let mut xml = String::new();
        decoder.read_to_string(&mut xml).unwrap();
        
        assert!(xml.contains("145"));
        assert!(xml.contains("<Tempo"));
        
        fs::remove_file(output).ok();
    }

    #[test]
    fn test_generate_als_from_template() {
        let output = std::env::temp_dir().join("template_test.als");
        
        let sample = SampleInfo {
            path: "mock_kick.wav".to_string(),
            name: "Mock Kick".to_string(),
            duration_secs: 0.5,
            sample_rate: 44100,
            file_size: 1000,
            bpm: None,
        };
        
        let track = TrackInfo {
            name: "Kick".to_string(),
            color: 1,
            clips: vec![
                ClipPlacement {
                    sample: sample.clone(),
                    start_beat: 0.0,
                    duration_beats: 4.0,
                }
            ],
        };
        
        let result = generate_als_from_template(&output, &[track], 130.0);
        assert!(result.is_ok());
        assert!(output.exists());

        let file = File::open(&output).unwrap();
        let mut decoder = GzDecoder::new(file);
        let mut xml = String::new();
        decoder.read_to_string(&mut xml).unwrap();
        
        assert!(xml.contains("Mock Kick"));
        assert!(xml.contains("mock_kick.wav"));
        assert!(xml.contains("<AudioClip"));
        
        fs::remove_file(output).ok();
    }

    #[test]
    fn test_generate_minimal_als() {
        let output = std::env::temp_dir().join("test_techno.als");

        let kick = SampleInfo {
            path: "/tmp/mock_kick.wav".to_string(),
            name: "Kick 808".to_string(),
            duration_secs: 0.2,
            sample_rate: 96000,
            file_size: 50000,
            bpm: None,
        };

        let clap = SampleInfo {
            path: "/tmp/mock_clap.wav".to_string(),
            name: "Snare 808".to_string(),
            duration_secs: 0.3,
            sample_rate: 96000,
            file_size: 60000,
            bpm: None,
        };

        let hat = SampleInfo {
            path: "/tmp/mock_hat.wav".to_string(),
            name: "Hihat Closed".to_string(),
            duration_secs: 0.15,
            sample_rate: 44100,
            file_size: 30000,
            bpm: None,
        };

        let config = TechnoConfig {
            bpm: 130.0,
            kick,
            clap,
            hat,
        };

        let tracks = config.generate_arrangement();
        let result = generate_als(&output, &tracks, 130.0);

        assert!(result.is_ok(), "Failed to generate ALS: {:?}", result);
        assert!(output.exists(), "Output file not created");

        let metadata = fs::metadata(&output).unwrap();
        assert!(metadata.len() > 0, "Output file is empty");
        
        fs::remove_file(output).ok();
    }

    #[test]
    fn test_generate_techno_als() {
        let output = std::env::temp_dir().join("techno_test.als");
        
        // Use mock paths - the generator doesn't check if they exist during generation
        // as it only embeds them in the XML.
        let result = generate_techno_als(
            &output,
            "mock_kick.wav",
            "mock_clap.wav",
            "mock_hat.wav",
            128.0
        );
        
        assert!(result.is_ok());
        assert!(output.exists());
        
        let file = File::open(&output).unwrap();
        let mut decoder = GzDecoder::new(file);
        let mut xml = String::new();
        decoder.read_to_string(&mut xml).unwrap();
        
        assert!(xml.contains("mock_kick.wav"));
        assert!(xml.contains("128"));
        
        fs::remove_file(output).ok();
    }
}
