//! Ableton Live Set (.als) generator for programmatic techno track creation.
//!
//! Generates valid gzip-compressed XML files that Ableton Live 12 can open.
//! Uses samples from the indexed library to create full arrangements.

use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::Write;
use std::path::Path;

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
#[derive(Debug, Clone)]
pub struct SampleInfo {
    pub path: String,
    pub name: String,
    pub duration_secs: f64,
    pub sample_rate: u32,
}

/// An audio clip placement in the arrangement
#[derive(Debug, Clone)]
pub struct ClipPlacement {
    pub sample: SampleInfo,
    pub start_beat: f64,
    pub duration_beats: f64,
}

/// A track containing multiple clip placements
#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub name: String,
    pub color: u8,
    pub clips: Vec<ClipPlacement>,
}

/// Techno arrangement section
#[derive(Debug, Clone, Copy)]
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
    let mut ids = IdAllocator::new(1000);

    let mut tracks_xml = String::new();
    for track in tracks {
        tracks_xml.push_str(&generate_audio_track(track, &mut ids));
    }

    let next_pointee_id = ids.next();

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Ableton MajorVersion="5" MinorVersion="12.0_12049" SchemaChangeCount="3" Creator="Ableton Live 12.0" Revision="">
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
		<ExpressionLanes />
		<ContentLanes />
		<Locators />
		<DetailClipKeyMidis />
		<TracksListWrapper LomId="0" />
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
        name = track.name,
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
        name = sample.name,
        path = sample.path,
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
        },
        clap: SampleInfo {
            path: clap_path.to_string(),
            name: Path::new(clap_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Clap".to_string()),
            duration_secs: 0.3,
            sample_rate: 44100,
        },
        hat: SampleInfo {
            path: hat_path.to_string(),
            name: Path::new(hat_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Hat".to_string()),
            duration_secs: 0.1,
            sample_rate: 44100,
        },
    };

    let tracks = config.generate_arrangement();
    generate_als(output_path, &tracks, bpm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_generate_minimal_als() {
        let output = std::env::temp_dir().join("test_techno.als");

        let kick = SampleInfo {
            path: "/Applications/Ableton Live 12 Suite.app/Contents/App-Resources/Core Library/Samples/Multisamples/Drum Machines/808/Kick 808 Tone1.wav".to_string(),
            name: "Kick 808".to_string(),
            duration_secs: 0.2,
            sample_rate: 96000,
        };

        let clap = SampleInfo {
            path: "/Applications/Ableton Live 12 Suite.app/Contents/App-Resources/Core Library/Samples/Multisamples/Drum Machines/808/Snare 808 Tone1 k.wav".to_string(),
            name: "Snare 808".to_string(),
            duration_secs: 0.3,
            sample_rate: 96000,
        };

        let hat = SampleInfo {
            path: "/Applications/Ableton Live 12 Suite.app/Contents/App-Resources/Core Library/Samples/One Shots/Drums/Hihat/Hihat Closed Argus.wav".to_string(),
            name: "Hihat Closed".to_string(),
            duration_secs: 0.15,
            sample_rate: 44100,
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

        println!("Generated ALS at: {}", output.display());
        println!("File size: {} bytes", metadata.len());
    }
}
