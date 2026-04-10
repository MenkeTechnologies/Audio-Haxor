# audio-engine

AudioEngine binary for **AUDIO_HAXOR**: **JUCE 8** (`AudioDeviceManager`, `AudioTransportSource`, `AudioFormatReader`) for **input/output** device discovery, **library file playback** with 3-band EQ / gain / pan, **VST3** + **AU** (macOS) **plugin scanning** via `KnownPluginList` + `PluginDirectoryScanner`, **native plug-in editor windows** (`DocumentWindow` + `AudioProcessorEditor`), and a **persistent** stdin line protocol (one JSON request in → one JSON line out). A **stdin reader thread** runs `Engine::dispatch` (most commands take `Impl::mutex` on that thread). **Main** is the JUCE **message thread**: it loops `MessageManager::runDispatchLoopUntil` until stdin closes so async plugin creation and **UI** (`callFunctionOnMessageThread` for insert editors, etc.) can run.

## License (JUCE)

JUCE is distributed under the **GPLv3** (or a **commercial** license from Raw Material Software). Building and distributing this AudioEngine **without** a JUCE commercial license generally implies **GPL obligations** for the combined work. Confirm your licensing model before shipping binaries.

## Protocol

### JSON responses (`okObj()`)

**Never** call `okObj().getDynamicObject()` on the temporary `var` — the `var` is destroyed at the end of the full-expression, so the `DynamicObject*` dangles. Keep a named `juce::var out = okObj();`, mutate `out.getDynamicObject()`, and `return out`.

### Crash diagnostics

The AudioEngine installs **POSIX signal handlers** (SIGSEGV, SIGBUS, SIGILL, SIGFPE, SIGABRT) and **`std::terminate`**: on fatal signals it writes **`ENGINE [fatal signal …]`** plus a **`backtrace_symbols_fd`** stack to **stderr** and, when **`AUDIO_HAXOR_ENGINE_LOG`** or **`AUDIO_HAXOR_APP_LOG`** is set, appends the same to that file (raw lines, not the usual `[timestamp] ENGINE:` prefix; **prefer `AUDIO_HAXOR_ENGINE_LOG`** so crash dumps land next to normal engine lines). **SIGPIPE is ignored** so a closed stdout/pipe does not kill the process silently before the next write fails. After a crash, the host’s **`audio-engine IPC error`** line often includes a **stderr tail** with this text. For symbol names, link with **`-rdynamic`** on Linux if stacks are opaque.

The Tauri host sets **`AUDIO_HAXOR_ENGINE_LOG`** to the absolute path of **`engine.log`** in the app data directory (same folder as **`audio_haxor.db`** — on macOS typically **`~/Library/Application Support/com.menketechnologies.audio-haxor/`**, matching `tauri.conf.json` **`identifier`**; timestamped **`ENGINE:`** lines). When the file exceeds **5MB**, it is renamed to **`engine.log.1`** (replacing any previous backup). If rename fails, the file is **deleted or truncated** so it cannot grow unbounded. Rotation runs on startup and before each append (same cap as the host’s **`app.log`**). It also sets **`AUDIO_HAXOR_APP_LOG`** to **`app.log`** for backward compatibility; **`AppLog`** prefers **`AUDIO_HAXOR_ENGINE_LOG`** when both are set. **`AUDIO_HAXOR_PARENT_PID`** is the host’s process ID.

**Extra tracing:** set **`AUDIO_HAXOR_ENGINE_LOG_STDERR`** to any non-empty value so every **`engine.log`** line is **also** written to **stderr** (useful for piped/CLI runs where you did not set a log file path). Lifecycle messages include **stdin reader thread started**, **entering message pump loop (message thread)**, **AudioDeviceManager** first-init bracketing, **parent watchdog: armed**, and **parent watchdog:** exit reason when the host PID disappears. A background thread in **`ParentWatchdog.cpp`** polls every 2s: if that PID is gone (normal exit, crash, or **force quit** / SIGKILL where Rust teardown never runs), the AudioEngine calls **`std::_Exit(0)`** so orphaned **`audio-engine`** processes do not keep running. The AudioEngine appends lines as **`[YYYY-MM-DD HH:MM:SS] ENGINE: …`** (UTC timestamp; same bracket style as the host’s `write_app_log`). Typical messages: startup, **`stdin` parse failure**, one line per request with the command name (for example **`cmd list_output_devices`**) except **`ping`**, **`playback_status`**, and **`playback_seek`** (polling / frequent seeks), **`error:`** lines for every JSON error response from **`dispatch`** (unknown commands, validation failures, device errors, preview failures, etc.) and for fatal plugin-scan worker exceptions, and exit when stdin closes. For a **standalone** binary (no env var), file logging is a no-op and the parent watchdog is inactive.

**`audio device:`** lines (in addition to **`cmd …`**) summarize **discovery** (`list_output_devices`, `list_input_devices`, `list_audio_device_types`), **capability probes** (`get_output_device_info`, `get_input_device_info`), **driver selection** (`set_audio_device_type`), **validated device id** (`set_output_device`, `set_input_device` — stream not open yet), and **stream open/close** (`start_output_stream`, `start_input_stream`, `stop_output_stream`, `stop_input_stream`).

The app’s **Audio Engine** settings populate **output/input buffer** dropdowns from **`buffer_sizes`** on **`get_output_device_info`** / **`get_input_device_info`** (same pattern as sample rate). Each line is a JSON object with at least `cmd`. Optional fields include `device_id`, `tone` (bool, output only), `buffer_frames` (positive `u32`, **capped at 8192**), **`sample_rate_hz`** (positive integer, optional on **`start_output_stream`** / **`start_input_stream`**), and **`start_playback`** (bool, output only): when `true` after **`playback_load`**, output is driven by **JUCE** transport on the selected device (not the test-tone path). Omit **`sample_rate_hz`** to let the driver pick a rate (or, for playback without an explicit rate, the file’s sample rate is used when **`start_playback`** is set). If **`buffer_frames`** is **omitted** and the driver’s initial buffer (**`getCurrentBufferSizeSamples()`**) is **smaller than 512** frames, the engine **reapplies** the device setup with the **smallest supported** buffer size **≥ 512** (or the **largest** available if none qualify) to reduce **xruns**; explicit **`buffer_frames`** from the client is **not** overridden. **`output_stream_status`**, **`input_stream_status`**, and **`start_output_stream` / `start_input_stream` success** payloads include **`current_buffer_frames`** (actual post-setup buffer length in samples).

### Library playback (JUCE + DSP)

**`playback_load`** opens the path with **`AudioFormatManager`** (`registerBasicFormats`). Duration and source sample rate come from the reader. The AudioEngine handles **`playback_load`** before **`AudioDeviceManager::initialise`** so opening a file does not block on CoreAudio first — device setup runs when **`start_output_stream`** (or any other audio-device command) runs. **`start_output_stream`** with **`start_playback: true`** wires **`AudioTransportSource`** → **`AudioSourcePlayer`** → **`AudioDeviceManager`** output callback. DSP (EQ / gain / pan) runs in **`DspStereoFileSource`**, then **optional VST3 / AU inserts** (see **`playback_set_inserts`**), before audio reaches the device. **Reverse playback** uses a sample-wise path — **inserts are skipped** in that mode.

| Command | Fields | Purpose |
|--------|--------|---------|
| `playback_load` | `path` (absolute) | Open file; store session; does **not** open output. |
| `start_output_stream` | `start_playback: true`, `device_id`, optional `buffer_frames`, optional `sample_rate_hz` | After **`playback_load`**, start transport on the device. |
| `playback_pause` | `paused` (bool) | Pause / resume transport. |
| `playback_seek` | `position_sec` | Seek (seconds on the forward timeline). |
| `playback_set_dsp` | `gain`, `pan`, `eq_low_db`, `eq_mid_db`, `eq_high_db` | Update DSP parameters. |
| `playback_set_speed` | `speed` (float, clamped 0.25–2.0) | **`ResamplingAudioSource`** on the forward file path (tape-style: pitch follows speed). **Reverse** playback ignores resampling (response may include a **note**). |
| `playback_set_reverse` | `reverse` (bool) | When `true`, full-decode-to-RAM reverse path for the next playback. |
| `playback_set_loop` | `loop` (bool) | Full-file loop: **forward** uses `AudioFormatReaderSource::setLooping`; **reverse** (RAM decode) wraps `reverseFrame` to 0 when the reversed pass reaches the start of the file. |
| `playback_status` | — | Position, duration, peak, pause, EOF, reverse, sample rates. When output is running, also **`spectrum`**: 1024 **uint8** magnitudes (0–255) for FFT bins 1…1024 of a Hann-windowed real FFT (order 11 → size 2048), normalized per frame to the max bin; **`spectrum_fft_size`**, **`spectrum_bins`** (1024), **`spectrum_sr_hz`**. The tap is **mono (L+R)/2** taken **after** DSP and **VST/AU inserts** on the forward path (reverse playback has no inserts). Samples are queued in **one** `spectrumRingMutex` acquisition per device callback block (not per sample). |
| `playback_stop` | — | Stop transport and clear session. |
| `playback_set_inserts` | `paths` (JSON array, max 32, empty clears): each entry is either an on-disk **`.vst3` bundle** / **`.component`**, or the same **`fileOrIdentifier`** string the UI got from **`plugin_chain`** **`plugins`** (e.g. **`AudioUnit:…`** — not a filesystem path). **Resolution:** **`fileOrIdentifier` must match an entry in the scan cache** (no synchronous disk probe — avoids hangs on WaveShell / heavy bundles). | Load **VST3** / **AU** (macOS) effect instances in order. **Requires** `stop_output_stream` first (no hot-swap while the device is open). Closes any open insert editor windows. **After** a successful non-empty load, the engine runs **`InsertChainRunner::prepare(44100 Hz, 512 samples)`** on the message thread so instances are prepared even before **`start_output_stream`** — native **`playback_open_insert_editor`** UIs often stay **blank** if `prepareToPlay` never ran. When playback opens, **`prepareToPlay` runs again** with the device rate/block. **Runtime:** when the device calls `AudioSource::prepareToPlay`, each plug-in’s **`AudioProcessor::prepareToPlay` is run on the JUCE message thread** (VST3’s `setupProcessing` / JUCE assert). The host calls **`setPlayConfigDetails(2, 2, …)`**, **`setProcessingPrecision(single)`**, then **`prepareToPlay`**. Each callback resizes the **scratch buffer to exactly `numSamples`** for that block (partial blocks no longer leave a larger buffer, so `processBlock` does not process stale samples). **`processBlock` runs under `getCallbackLock()`** (same pattern as JUCE `AudioProcessorPlayer`). **Inserts apply to file playback only**, not the test-tone path. |
| `playback_open_insert_editor` | `slot` (int, 0-based index into the **loaded** insert chain) | Opens a **native** editor for that instance (`createEditorIfNeeded`). **Marshalled onto the JUCE message thread** (`callFunctionOnMessageThread`); stdin IPC thread must not build `DocumentWindow` / plugin UI. The host uses a **JUCE (non-native) title bar** and **`setContentComponentSize`** so the window’s **content** area matches the editor — native title bars and raw `setSize` on the outer window often produced **blank** VST3 (`IPlugView` attach) / AU views. **`setContentOwned(..., true)`** so AU/VST **childBoundsChanged** can resize the outer window when the Cocoa / `IPlugView` reports size (often async). **Do not** call **`AudioProcessorEditor::setScaleFactor(getDesktopScaleFactor())`** from this host — JUCE applies an **AffineTransform** to the whole editor, which breaks **Retina** NSView / VST3 embedding (**blank white** client with a good title bar). Initial editor size uses a **large minimum** (480×360) because many AUs advertise 100×100 until the view loads. **`setResizeLimits(280, 220, …)`** stops spurious 0×0 child sizes from collapsing the frame. **`setOpaque(true)`** on the host window. For **AudioUnit** only, the first **`setVisible` / `toFront`** is **deferred** with **`MessageManager::callAsync`** so **`RequestViewController` / `embedViewController`** can post before the window is shown (same-stack show can leave an empty AU view). **VST3** (and other non–Audio Unit formats) use **synchronous** show on the message thread right after **`createEditorIfNeeded`** — deferring visibility was hurting **`IPlugView::attached`** for some VST3 plug-ins (white client). **`juce::Process::makeForegroundProcess()`** runs **synchronously before** any **`setVisible`** (including before **`callAsync`** for Audio Unit) so the **audio-engine** subprocess activates **NSApp** even when the host’s next message-tick is delayed (e.g. busy **`tauri dev`** loop); otherwise Cocoa / AU views may not attach. **`suspendProcessing(false)`** before creating the editor. After show, **`schedulePostShowLayout`** runs async plus **16 / 50 / 150 / 300 / 600 ms** deferred **resized/repaint**. **`setAlwaysOnTop(true)`** so it tends to float above the main app. Fails with `plugin has no editor` if the plug-in has no UI. |
| `playback_close_insert_editor` | `slot` (int) | Closes the hosted editor window for that chain index, if open. **Also runs on the message thread** so window teardown matches JUCE rules. |

`stop_output_stream` tears down the output device graph and clears playback as needed.

| Command | Purpose |
|--------|---------|
| `ping` | Version + host id |
| `waveform_preview` | **`path`** (absolute), optional **`width_px`** (32–8192, default 800), **`start_sec`**, **`duration_sec`** (max 300 s) — decodes a segment and returns **`peaks`**: array of `{ "min", "max" }` per column (mono mix). No device init. Handled **before** `Engine::Impl::mutex`: work runs on a **`std::async` thread** and takes a dedicated **`previewMutex`** so long decodes do not block other engine commands behind the main lock. |
| `spectrogram_preview` | **`path`**, optional **`width_px`** / **`height_px`** (16–512, defaults 256×128), **`fft_order`** (8–15 → FFT size 256…32768), **`start_sec`**, **`duration_sec`** (max 120 s) — STFT magnitudes in dB. Response **`rows`**: outer = frequency (low→high), inner = time columns; **`db_min`** / **`db_max`** (–100…0). Same **`std::async`** + **`previewMutex`** pattern as **`waveform_preview`**. |
| `engine_state` | Aggregated stream snapshot |
| `list_output_devices` / `list_input_devices` | Enumerate devices (uses the active JUCE device type; falls back across types if the current type lists nothing) |
| `list_audio_device_types` | Lists JUCE driver types: `types` is a **JSON array of strings** (driver names); `current` is the active type |
| `set_audio_device_type` | `type` (string) — switches **AudioDeviceManager** driver; **stops** active output/input streams |
| `get_output_device_info` / `get_input_device_info` | Default config + `buffer_size` + `sample_rates` + `buffer_sizes` + `audio_device_type` |
| `set_output_device` / `set_input_device` | Validate `device_id` |
| `start_output_stream` / `stop_output_stream` | Output; optional tone or file playback |
| `start_input_stream` / `stop_input_stream` | Input; peak meter |
| `output_stream_status` / `input_stream_status` | Status lines |
| `set_output_tone` | 440 Hz sine when F32 output + not in file playback mode |
| `plugin_chain` | Scanned plugin list + **`insert_paths`** / **`slots`**. First request starts a **background** VST3/AU directory scan; responses use **`phase`: `"scanning"`** until done, then **`phase`: `"juce"`** with **`plugins`**. While **`phase` is `"scanning"`**, JSON includes **`scan_done`**, **`scan_total`** (candidate file counts), **`scan_skipped`** (modules skipped because the cached listing is up to date), **`scan_cache_loaded`** (whether `known-plugin-list.xml` was read), **`scan_current_format`** (`VST3` / `AU`), **`scan_current_name`** (plug-in currently being scanned). For **which bundle/path is being scanned** (and a monotonic **`scan_seq`** aligned with **`scan_done` + 1** for the next module), see **`engine.log`**: lines **`plugin scan: START …`** include **`file="…"`** (AU identifier or `.vst3` path), **`name="…"`**, **`format_pos`** within the format, and **`cache_listing_up_to_date`**. **Each module is scanned in a child `audio-engine` process** (`--plugin-scan-one`): the parent persists the current cache, spawns the child, waits up to **`AUDIO_HAXOR_PLUGIN_SCAN_TIMEOUT_SEC`** (default **120**, clamped **5–3600** seconds), then **kills** the child on timeout — **`plugin scan: TIMEOUT_KILL`** / **`TIMEOUT_SKIP`** — and blacklists that identifier (same **`plugin-scan-skip.txt`** path as other skips). **`plugin scan: OOP_FAIL_SKIP`** / **`OOP_START_FAIL_SKIP`** cover child failures or failure to spawn. Successful children write merged **`KnownPluginList`** XML back for the parent to load. **`plugin scan: FAIL_SKIP …`** (legacy wording in older logs) covered **`scanNextFile`** throws in the **in-process** scanner; the subprocess path treats non-zero child exit like a skip. You can **pre-populate** **`plugin-scan-skip.txt`** to skip a known-bad AU/VST without loading it. The engine also **always excludes** several Apple system AU identifiers that **hang indefinitely** inside JUCE’s `scanNextFile` (no throw, so no throw-based skip): **AUVectorPanner** (`AudioUnit:Panners/aupn,vbas,appl`), **AUMixer** (`AudioUnit:Mixers/mixr,aumx,appl`), **AUMatrixMixer** (`…/mixr,mxmx,appl`), **AUMultiChannelMixer** (`…/mixr,mcmx,appl`), **AUSpatialMixer** (`…,spmx,appl` — any AU identifier whose subtype is **`spmx`** and manufacturer **`appl`** is skipped, not only `AudioUnit:Mixers/mixr,spmx,appl`). Add more via **`plugin-scan-skip.txt`** or **`AUDIO_HAXOR_PLUGIN_SCAN_SKIP`** using the exact **`file="…"`** string from **`plugin scan: START`** in **`engine.log`**. Set **`AUDIO_HAXOR_PLUGIN_SCAN_SKIP`** to a comma-separated list of **additional** full identifiers to skip without editing the file. **`plugin scan: SKIP_LIST …`** in **`engine.log`** is logged when an identifier is removed by the merged skip list (file + built-ins + env). **`plugin scan: started`** (total candidates), **`plugin scan: finished ok`** / **`plugin scan: finished cancelled`** (terminal markers; cancelled mid-scan includes **`plugin_count`**), and **`plugin scan: VST3 phase complete; starting AU`** bracket phases on macOS. **`KnownPluginList`** is persisted to **`known-plugin-list.xml`** in the same app-data folder as the dead-man’s-pedal file so restarts reuse unchanged entries (see **Plugin scan cache files** below — incremental writes during the worker; JUCE **`BLACKLISTED`** entries are serialized **after** plugin descriptions). JUCE **dead-man's-pedal** file **`plugin-scan-dead-mans-pedal.txt`** lives in **`appDataDirectoryForAudioEngine()`**: the **parent directory of `engine.log`** when the host sets **`AUDIO_HAXOR_ENGINE_LOG`** (same as Tauri app data); if neither log env var is set (standalone AudioEngine), **`userApplicationDataDirectory/MenkeTechnologies/audio-haxor`**. It defers crashy modules to the end on later runs. **Native crashes** in the **child** process lose that module’s result but **do not** kill the long-lived parent AudioEngine; the **last `plugin scan: START` line** before a crash identifies the module — copy **`file=`** into **`plugin-scan-skip.txt`** if it keeps crashing. **Playback:** while **file output is running** (`outputRunning` + `playbackMode`), the scan worker **sleeps 2ms** after each module so the parent’s audio callback is less likely to starve (child processes carry most **`scanNextFile`** CPU load). |

## Plugin scan cache files

- **`known-plugin-list.xml`** — Same directory as **`plugin-scan-skip.txt`**, **`plugin-scan-dead-mans-pedal.txt`**, and (when the host sets it) the parent of **`engine.log`**: **`appDataDirectoryForAudioEngine()`**. With the Tauri app on macOS that is typically **`~/Library/Application Support/com.menketechnologies.audio-haxor/`**. If you run **`audio-engine` standalone** without **`AUDIO_HAXOR_ENGINE_LOG`** / **`AUDIO_HAXOR_APP_LOG`**, the directory is **`~/Library/Application Support/MenkeTechnologies/audio-haxor/`** instead — a **different** folder, so looking only under the bundle id path will not show files from a CLI run.

- **`known-plugin-list.backup.N.xml`** — **Not** read by the engine (parity / manual diff only). On **`plugin_rescan`**, the current **`known-plugin-list.xml`** is moved to **`known-plugin-list.backup.1.xml`**, previous **`.1` → `.2`**, and so on; the oldest backup beyond the configured depth is dropped. Depth is **`kKnownPluginListBackupSlots`** in **`Engine.cpp`** (source of truth for the count).

  **JUCE layout (not truncation):** The root tag is **`KNOWNPLUGINS`**. JUCE **`KnownPluginList::createXml()`** writes **one XML child per successfully known plugin** (`PluginDescription`), **then** appends **`<BLACKLISTED id="…"/>`** for identifiers that failed or were blacklisted during scan. So **all `BLACKLISTED` nodes come after** the plugin entries — that section is **not** “plugins omitted after the first N.” If you see only a small number of plugin elements before a long **`BLACKLISTED`** run, **`KnownPluginList`** really has that many **successful** descriptions and the rest are failures or skips — use **`plugin scan: FAIL_SKIP`** / **`START`** in **`engine.log`** and **`plugin-scan-skip.txt`**. This cache is **separate** from the main app’s SQLite plugin inventory.

  **Persistence:** The engine **re-saves after each scanned module** (and on per-module failure paths) so a crash mid-scan still leaves a partial cache. Writes use a **temporary file** (`known-plugin-list.xml.tmp`) and **`replaceFileIn`** so the main XML is never left half-written if the process is killed mid-flush (which previously could produce a tiny or invalid file and force a rescan that overwrote a good backup). **Disk/XML write errors** log **`known-plugin-list.xml tmp write failed`**, **`atomic replace failed`**, or **`known-plugin-list.xml write failed (continuing)`** and do **not** abort the scan (no need to restart the engine to continue past a bad module). If **`known-plugin-list.xml` exists but cannot be parsed**, it is **moved aside** as **`known-plugin-list.invalid.<timestamp>.xml`** before a full rescan runs, so the bad bytes are not silently overwritten. When **`plugin_chain`** must **join** a prior scan thread before starting a new run, it does so **without** holding the scan mutex — the worker takes that mutex at the end of each run; joining while holding it **deadlocked** the engine (symptom: no **`plugin_chain`** response until the IPC client timed out). **VST3** and **AU** phases are isolated: an unexpected error in one phase is logged and the other phase still runs. **`plugin_chain`** reaches **`phase: "juce"`** with whatever plugins were successfully registered; **`phase: failed`** is reserved for a future fatal IPC path and is **not** used for ordinary per-plugin scan failures.

  **Fast path:** The first **`plugin_chain`** while scan state is **Idle** loads **`known-plugin-list.xml`** when it contains at least one plugin type, returns **`phase: "juce"`** immediately (no background rescan), and populates insert-slot options. Remove or invalidate the XML to force a full scan on the next **Idle** **`plugin_chain`**.

  **`plugin_rescan`:** Cancels any running scan (cooperative flag checked per module), **rotates** the current **`known-plugin-list.xml`** into **`known-plugin-list.backup.N.xml`** (see above), then deletes **`plugin-scan-skip.txt`** and **`plugin-scan-dead-mans-pedal.txt`**, clears the in-memory cache, and resets scan state to **Idle**. The next **`plugin_chain`** triggers a full fresh scan. The UI exposes this as **"Wipe cache & rescan"**. Accepts an optional **`timeout_sec`** field (5–3600) to update the per-plugin scan timeout for the current engine process (sets `AUDIO_HAXOR_PLUGIN_SCAN_TIMEOUT_SEC` in-process). The UI exposes this as a number input next to the rescan button; the value is persisted in preferences as **`pluginScanTimeoutSec`** and also passed as an env var to the engine at spawn time.

## Automated tests (IPC)

From the repository root, after a **Debug** or **Release** build (`node scripts/build-audio-engine.mjs`):

```bash
pnpm run test:audio-engine
# or: node scripts/run-audio-engine-tests.mjs
```

These tests spawn the binary and assert **stdin/stdout JSON** (`ping`, bad JSON handling including **multiple** bad lines, **`cmd`** case folding, **blank stdin lines** (no matching stdout line), **`playback_load`** / **`waveform_preview`** / **`spectrogram_preview`** path validation including **directories** and **empty on-disk** files that are not a supported format — **no** `list_*_devices` / `plugin_chain`, which can block in piped shells). Override the binary path with **`AUDIO_ENGINE_TEST_BIN`**. On **Linux** without a display, run under **`xvfb-run -a`** (as in CI).

## Build

**Prerequisites:** **CMake** ≥ 3.22, **Ninja**, and a C++20 toolchain. Platform libs (e.g. **ALSA** on Linux) must match your JUCE audio backend expectations.

From the **repository root**:

```bash
# Debug (matches pnpm tauri dev — beforeDevCommand)
node scripts/build-audio-engine.mjs

# Release (matches prepare AudioEngine)
AUDIO_ENGINE_BUILD_TYPE=release node scripts/build-audio-engine.mjs
```

Artifacts land at **`audio-engine-artifacts/debug/audio-engine`** or **`audio-engine-artifacts/release/audio-engine`** (intentionally **not** under Cargo `target/` — MSVC sidecars next to `audio-engine.exe` broke `cargo test` on Windows). **Release** bundles use `scripts/prepare-audio-engine-audioengine.mjs` → `src-tauri/binaries/audio-engine-<triple>` for Tauri `externalBin`.

### Helper `.app` architecture (macOS)

The `audio-engine` sidecar is **not** shipped as a bare Mach-O sibling of the main `audio-haxor` binary in `Contents/MacOS/`. It is reshaped into a nested **helper `.app`** at:

```
AUDIO_HAXOR.app/
  Contents/
    MacOS/
      audio-haxor                                # Tauri main binary
      AudioHaxorEngineHelper.app/                # nested helper bundle (sibling of audio-haxor)
        Contents/
          Info.plist                             # bundle id: com.menketechnologies.audio-haxor.audio-engine-helper
          MacOS/
            audio-engine                         # JUCE host binary, hardened runtime + plugin entitlements
          _CodeSignature/
            CodeResources
```

**Why `Contents/MacOS/` and not `Contents/Frameworks/`?** Apple's HIG suggests `Contents/Frameworks/` for embedded helper bundles, but we tried that first and it didn't work — `audiocomponentd` still refused XPC view-controller delivery for out-of-process AU plugins, leaving editor windows blank. LaunchServices treats `.app` bundles inside `Contents/Frameworks/` as embedded frameworks rather than registrable apps. Bitwig Studio's `Bitwig Studio.app/Contents/MacOS/` contains four nested helper `.app`s — `Bitwig Audio Engine ARM64-NEON.app`, `Bitwig Audio Engine X64-AVX2.app`, **`Bitwig Plug-in Host ARM64-NEON.app`**, **`Bitwig Plug-in Host X64-SSE41.app`** — and Bitwig's AU plugin editors work. Their pattern is the existence proof: nested helper `.app` next to the main binary in `Contents/MacOS/` is what `audiocomponentd` accepts.

**Why a nested `.app` instead of the bare sidecar Tauri's `externalBin` produces?** macOS routes most third-party AUv2 plugins through `audiocomponentd` via `kAudioComponentInstantiation_LoadOutOfProcess`, even when the host calls the legacy `AudioComponentInstanceNew`. The plugin's UI is then delivered as a `_RemoteAUv2ViewFactory` proxy NSView (from `AudioToolboxCore.framework`) whose actual view is shipped over an XPC connection. For `audiocomponentd` to authorize the host process and complete the audit-token handshake, the host must look like a real Cocoa application from LaunchServices' perspective — its own bundle, its own `Info.plist`, its own bundle identifier, its own NSApp activation context.

A bare Mach-O sibling of `audio-haxor` inside `Contents/MacOS/` does **not** qualify: `[NSBundle mainBundle]` for that process resolves up the directory tree and returns the parent `AUDIO_HAXOR.app`, so `audiocomponentd` considers it part of the main app and the XPC view delivery either stalls or returns the **1×1 placeholder NSView** indefinitely → permanently blank plugin editor windows. Wrapping the binary in a nested `.app` makes `[NSBundle mainBundle]` stop at `AudioHaxorEngineHelper.app` (its own bundle id, distinct from the parent's `com.menketechnologies.audio-haxor`), and `audiocomponentd` accepts the helper as a real host. This is the standard pattern Bitwig, Reaper, Chromium and Electron all use for their helper processes.

The helper `Info.plist` sets `LSUIElement=true`, which makes the helper an "agent" process — no Dock icon, no menu bar at startup. When a plugin editor window is opened, the helper transitions to a regular foreground app at runtime via `audio_haxor::activateAsForegroundApp()` (which does both `setActivationPolicy:Regular` and `activateIgnoringOtherApps:YES` — JUCE's `juce::Process::makeForegroundProcess()` only does the first half, which is necessary but not sufficient for `audiocomponentd` to deliver XPC view callbacks).

The helper's `Main.cpp` also calls `[NSApp finishLaunching]` at startup (via `audio_haxor::finishCocoaAppLaunching()`) — `juce::ScopedJuceInitialiser_GUI` calls `[NSApplication sharedApplication]` but never `finishLaunching`, leaving NSApp in a half-initialised state where the process is not registered with LaunchServices as a real Cocoa app. Without `finishLaunching`, `audiocomponentd` treats us as a "background utility" and refuses XPC view delivery even with the nested bundle in place.

**Bundling pipeline.** Tauri's `externalBin` ships the audio-engine binary at `Contents/MacOS/audio-engine` as usual; immediately afterwards `scripts/postbundle-audio-engine-helper.sh` runs and:

1. Moves `Contents/MacOS/audio-engine` → `Contents/MacOS/AudioHaxorEngineHelper.app/Contents/MacOS/audio-engine`.
2. Drops `audio-engine/helper-app/Info.plist` into `Contents/MacOS/AudioHaxorEngineHelper.app/Contents/Info.plist`.
3. Strips quarantine xattrs (`xattr -cr`) so they don't invalidate `codesign`.
4. **Codesigns innermost first**: signs the helper binary with `--options runtime` + `src-tauri/Entitlements.plist` (hardened runtime + the plugin-host entitlements: `disable-library-validation`, `allow-jit`, `allow-unsigned-executable-memory`, `disable-executable-page-protection`).
5. Seals the helper `.app` (`codesign --force --sign - "$HELPER_APP"`).
6. Re-seals the outer `.app` (`codesign --force --sign - "$APP"` — **without** `--deep`, so the inner helper signature we just made is not overwritten).
7. Verifies both signatures with `codesign --verify --verbose=2`.
8. Regenerates the DMG from the reshaped bundle (when `scripts/bundle_dmg.sh` is present), re-signs the DMG, copies the reshaped **`AUDIO_HAXOR.app`** to **`/Applications/AUDIO_HAXOR.app`** (replacing any previous install via `ditto`), and moves the versioned **`.dmg`** into **`/Applications/`**. If DMG bundling is skipped, the script still installs the reshaped **`/Applications/AUDIO_HAXOR.app`** only.

Order matters: the outer `.app`'s `_CodeSignature/CodeResources` includes a hash of the inner helper `.app`'s signature, so the inner one must be fully signed first. Re-signing the outer with `--deep` would recursively replace inner signatures and break this.

**Build entry points that run the postbundle reshape automatically:**
- `pnpm nuke` — full clean rebuild (`scripts/nuke.sh`)
- `pnpm rebuild` — incremental rebuild (`scripts/rebuild.sh`)
- `pnpm build` — wraps `tauri build` + postbundle in one command

**Plain `pnpm tauri build` / `pnpm tauri:build` does NOT** run the postbundle reshape and produces a bundle whose AU plugin editors will be blank. Either use one of the entry points above, or invoke `bash scripts/postbundle-audio-engine-helper.sh` manually after `tauri build`.

**Rust resolver.** `src-tauri/src/audio_engine.rs::resolve_audio_engine_binary()` checks the helper path (`Contents/MacOS/AudioHaxorEngineHelper.app/Contents/MacOS/audio-engine`) **before** the legacy sibling/triple-suffix paths, so a bundled `.app` always uses the helper. Dev builds (`pnpm tauri dev`) keep using the workspace artifact at `audio-engine-artifacts/<profile>/audio-engine` via the parent-walk fallback — no helper involved.

### JUCE source patches (macOS)

`audio-engine/CMakeLists.txt` applies one **idempotent in-place patch** to the `FetchContent`-downloaded JUCE 8 source after **`FetchContent_MakeAvailable(JUCE)`**. The patch is gated on `if(APPLE)`, uses a sentinel string for re-run safety, and re-applies automatically whenever `build/_deps/juce-src/` is wiped (e.g. by `pnpm nuke`).

**`modules/juce_audio_processors/format_types/juce_AudioUnitPluginFormat.mm`** — `AudioUnitPluginWindowCocoa::createView` is patched to **prefer the legacy `kAudioUnitProperty_CocoaUI` path over `kAudioUnitProperty_RequestViewController`**. Stock JUCE 8 unconditionally prefers `RequestViewController` and `return true`s without ever attaching the legacy `NSView*` it just obtained from `uiViewForAudioUnit:withSize:`, leaving the wrapper waiting for an async XPC view-controller delivery (`waitingForViewCallback = true`). For our spawned `audio-engine` subprocess that XPC delivery never arrives, so the editor renders **permanently blank** (white client area, JUCE-drawn title bar). Many AUv2 plugins (UAD Ampeg B15N, Sonimus SatsonCM, etc.) implement BOTH properties, so they all hit this. Symptom in `engine.log`: editor child reports `size=1x1 grandchildren=0` after `createEditorIfNeeded`. The patch adds `pluginView == nil &&` to the gating condition so JUCE only takes the async path when CocoaUI returned nothing (e.g. true AUv3 / RequestViewController-only plugins).

If JUCE upstream changes the `createView` signature, the CMake patch fails fast with `FATAL_ERROR` so the regression is caught at configure time instead of producing silently-broken AU editors at runtime.

### Linux (typical)

```bash
sudo apt-get install -y cmake ninja-build build-essential \
  libasound2-dev libfreetype6-dev libx11-dev libxinerama-dev libxrandr-dev libxcursor-dev libgl1-mesa-dev
```

### Device types (`list_audio_device_types`)

Use **`createAudioDeviceTypes()`** into a **local** `OwnedArray` — do **not** call **`getAvailableDeviceTypes()`** for this IPC. The latter runs **`scanDevicesIfNeeded()`** on the manager’s internal list; with **two** `AudioDeviceManager` instances (output + input) in the same process, that path can destabilize or crash the AudioEngine on some platforms (macOS), which shows up in the host as **`AudioEngine closed stdout`** after a respawn.

The **`types`** field is a **JSON array of strings** (one name per driver). Older builds used an array of `{ "type": "…" }` objects; the UI accepts both.

## Manual stdin/stdout (shell)

The AudioEngine speaks **one JSON object per line** on stdin and prints **one JSON line** per response on stdout (see `src/Main.cpp`).

**Bundled macOS app:** The AudioEngine lives under **`Contents/MacOS/`**. Tauri’s [external-binary](https://v2.tauri.app/develop/sidecar/) naming is **`audio-engine-<rustc-host-tuple>`** in `src-tauri/binaries/` at build time; the shipped `.app` often has **only** that suffixed file (not a plain **`audio-engine`**). The host resolves **`audio-engine`** via walking up to `audio-engine-artifacts/` (dev builds), then **`Contents/MacOS/audio-engine`**, then **`Contents/MacOS/audio-engine-<AUDIO_HAXOR_TARGET_TRIPLE>`** (compile-time triple from `src-tauri/build.rs`). **`ls "/Applications/AUDIO_HAXOR.app/Contents/MacOS"`** shows what you actually have.

```bash
# If you have the plain name:
AE="/Applications/AUDIO_HAXOR.app/Contents/MacOS/audio-engine"
# If you only have the triple-suffixed binary:
# AE="/Applications/AUDIO_HAXOR.app/Contents/MacOS/audio-engine-$(rustc --print host-tuple)"

printf '%s\n' '{"cmd":"ping"}' | "$AE"
printf '%s\n' '{"cmd":"plugin_chain"}' | "$AE"
```

Any non-JSON line (e.g. testing the pipe with random text) gets a single **`{"ok":false,"error":"bad JSON"}`** response — that confirms stdin/stdout wiring; use **`printf '%s\n' …`** for real commands so `{` / `"` are not eaten by the shell (avoid **`echo …`** with nested quotes unless you are sure of your shell’s rules).

**Dev build (repo):** after `node scripts/build-audio-engine.mjs`, use the unsuffixed binary:

```bash
printf '%s\n' '{"cmd":"ping"}' | ./audio-engine-artifacts/debug/audio-engine
```

Use **`printf '%s\n' …`** instead of **`echo …`** so quotes and newlines are predictable.

**Process sits with no stdout when piping (even `ping`):** Older builds called **`AudioDeviceManager::initialise`** for output + input inside **`Engine`** construction, **before** the stdin thread ran (that could block in CoreAudio). **Also**, **`MessageManager::callAsync`** + blocking on **`fut.get()`** from the stdin thread was unsafe on **macOS** until **`[NSApp run]`** is pumping. Current builds **defer** device-manager init until the **first non-`ping`** command and run **`Engine::dispatch` on the stdin thread** for every line (no `callAsync` / futures for IPC), so **`ping`**, typos like **`dping`**, and **`bad JSON`** all return a line without waiting on the AppKit loop for delivery.

**Hangs on `list_audio_device_types` / `list_*_devices` when piping from Terminal:** Those commands call JUCE **`AudioDeviceManager::createAudioDeviceTypes`** / enumeration, which uses **CoreAudio** on macOS. With **stdin from a pipe** the process is not a normal foreground GUI session; CoreAudio can **block for a long time or indefinitely** (waiting on HAL / session / permissions). Prefer exercising device IPC from the **host app** (Audio Engine tab) or run tests against **`ping`** / non-audio commands in the shell. **`plugin_chain`** can also take a long time on first call while JUCE scans plug-ins.

**Override path (host or manual testing):** set **`AUDIO_HAXOR_AUDIO_ENGINE`** to an absolute path to any built `audio-engine` binary (see “Stale AudioEngine” below).

## Stale AudioEngine / `unknown cmd`

The app keeps **one** long-lived `audio-engine` child. After rebuilding, an old process can still answer stdin until replaced. The Tauri host respawns when the resolved binary’s **size/mtime** changes. If IPC looks wrong, quit the app or set **`AUDIO_HAXOR_AUDIO_ENGINE`** to an absolute path to a fresh `audio-engine-artifacts/debug/audio-engine` or `audio-engine-artifacts/release/audio-engine`.

## Host app (WEB UI)

`frontend/js/audio-engine.js` drives the Audio Engine tab and coordinates **`playback_*`** with the floating player. **Open plug-in editor** compares the current UI insert paths to **`plugin_chain` `insert_paths`**; if they already match the loaded chain, it opens the native editor **without** stopping the output stream. If the UI differs (or the chain was never applied), it runs **`stop_output_stream`** then **`playback_set_inserts`** (no “inserts applied” toast — same as before) so **`playback_set_inserts`** never races an active stream. Behavior matches the root **`README.md`** Audio Engine / dev-vs-build sections (IPC guards, `engine_state` resync, input peak polling, etc.). On first panel refresh, if prefs request a different JUCE driver than the live process, **`set_audio_device_type`** is applied **only when no output/input stream is running** — otherwise the restore is deferred (that IPC **stops** streams, which would cut library playback). The **panel refresh** completes after device lists and caps; **VST3/AU directory scan** (`plugin_chain` → `phase: scanning`) continues in the **background** so the tab does not stay on “loading” for the full scan duration. While scanning, the UI **polls** `plugin_chain` and refreshes the Audio Engine plug-in progress line plus a **single** slide-in toast (updated in place, not stacked) — **`scan_done` / current plug-in name stay flat while JUCE blocks inside one slow module**, so the client shows **elapsed seconds** on that step. After **`AE_PLUGIN_SCAN_STUCK_HINT_SEC`** seconds on the same step (see `frontend/js/audio-engine.js`) it also appends **`ui.ae.plugins_scan_stuck_hint`**. **Hang vs crash:** the worker process stays alive while a **child** is stuck in `scanNextFile`; the parent eventually **kills** that child after **`AUDIO_HAXOR_PLUGIN_SCAN_TIMEOUT_SEC`** (see **`TIMEOUT_KILL`** / **`TIMEOUT_SKIP`** in **`engine.log`**). If **`plugin_chain`** still returns JSON, the main AudioEngine did not exit. The **dead-man’s-pedal** file still helps **crashy** plug-ins on **later** runs. **Restart Audio Engine** is only needed if the parent itself wedged (rare) or you changed the binary. **Insert chain** fuzzy pickers default to **effects only**: the WEB UI filters **`plugin_chain` `plugins`** by **`isInstrument`** (JUCE `PluginDescription::isInstrument`) unless the user enables **Show instruments** (preference **`aeInsertPickerShowInstruments`** = **`on`**). It also defaults to **hiding Audio Units** (preference **`aeInsertPickerHideAudioUnits`** = **`on`** when unset; flip to **`off`** to include them) because AU plugin editor windows render as a permanent 1×1 placeholder in this build — `audiocomponentd` refuses to deliver out-of-process AU view controllers (`_RemoteAUv2ViewFactory` XPC) to ad-hoc signed hosts. The *audio* path of an AU plugin still works fine (no UI required), so users who only need to drop a known-configured AU into the chain can disable the filter; users who expect to open the editor should stick to VST3. The limitation goes away the day the project is signed with a real Apple Developer ID — the helper `.app`, hardened runtime entitlements, `[NSApp finishLaunching]`, and `activateIgnoringOtherApps:YES` calls are all in place and only need a real `TeamIdentifier` to make `audiocomponentd` accept us. **`playback_set_inserts`** normalizes **`pluginFormatName`** for **`AudioUnit:`** identifiers (e.g. **`AU` → `AudioUnit`**) so JUCE’s format manager can match; if loading still fails with JUCE’s “no compatible plug-in format”, the engine replaces that with a clearer message when **`fileMightContainThisPluginType`** fails (e.g. **AudioComponentFindNext** — plug-in uninstalled or stale listing — use **Wipe cache & rescan**).
