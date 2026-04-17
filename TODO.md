loading spinner in Blacklist/SOurce dirs modal until hooked up, buttons dont work at first

ALS generator
- blacklist stems/construction kits samples b/c no loopable in this context
- blacklist all samples longer than 32 bars
- show all tracks even if no samples found
- OCean Demolish Night keeps on generating as name
- loops should loop on loop length, not on fixed 1 bar length

New rows in ALS gen table
- add kick stutters/rolls at various places like as fill
- add snare stutters/rolls at various places like as fill
- kick drops on downbeat etc

Need way better fx sweeps, more density/layers, pre sweeps on 8 bar division, post sweeps start on 8 bar division, sweep down starts on 1st beat of 8 bars not during fill, sweep up not cutoff by fill

put scatter samples into own group

trance lead generator, feeds tons of MIDI file patterns to learn patterns

Detecting sample packs
Yeah, it is. The problem space is genuinely ambiguous — there's no universal convention for how
  sample libraries are structured. Let me lay out the constraints before writing code.

  What we can anchor on:

  1. Known library roots — dirs like SampleLibraries, Samples, sounds/packs signal "everything below
  here is pack content"
  2. Known manufacturers — already detected. The pack is typically the next directory after the
  manufacturer in the path
  3. Internal structure dirs — numbered prefixes (02 Hi-Tech Hits), category names (Kicks), Samples,
  Audio, BPM dirs (128 BPM) are inside a pack, not the pack itself
  4. Duplicate dir names — STUTTER MATTER/STUTTER MATTER/ strongly signals the pack root
  5. Pack naming patterns — "Vol 1", "Edition", "Collection", dashes with label names (Tidy - Bits &
  Pieces)

  The hard cases:

  - Nested packs inside a label directory
  - Flat Splice structures where the pack dir IS the only meaningful level
  - User-reorganized libraries that don't follow any convention
  - Zip extractions that add an extra level


  /Users/wizard/mnt/production/MusicProduction/SampleLibraries/sampletraxx/STUTTER MATTER/STUTTER
MATTER/ST_STUTTER MATTER/Samples/02 Hi-Tech Hits/, the sample pack name is STUTTER_MATTER, not 02
Hi-Tech Hits

All checkboxes like Random seed lock etc must have same styles, like tonal checkboxes in track counts pane


