# Sound Lab visual review

final result: passed

## Visual target and capture setup

The approved five-layer mobile mockup in the design conversation is the source
visual target. Its source raster is 853 × 1844, normalized to 390 × 844 for the
comparison. The source asset in the working session is
`generated_images/exec-9dcc78b7-1950-4448-8488-bf78e9c90a8e.png`.

The actual WebAssembly application was rendered in Chromium through
`scripts/browser/sound-lab.mjs`, at 390 × 844 (touch, device scale 1) and
1440 × 900 (desktop, device scale 1). Music and Sound FX were checked at both
sizes. Browser captures are emitted by that script and retained by the Pages CI
workflow in its `directory-captures` artifact:

- `sound-lab-phone.png` and `sound-lab-desktop.png`
- `sound-lab-phone-chords.png` and `sound-lab-phone-drums.png`
- `sound-lab-phone-file.png`
- `sound-lab-phone-sfx.png` and `sound-lab-desktop-sfx.png`

During local review these are under `/tmp/sound-lab-final-captures/`. The
normalized full-view comparison is `/tmp/sound-lab-phone-comparison.png`;
the focused inspector comparison is `/tmp/sound-lab-inspector-comparison.png`.
These comparison boards place the reference and running app in the same image.

## Comparison history

1. The original implementation represented only three layers and hid most of
   the mobile inspector below a large scrolling piano roll. All five layers now
   have aligned rows, previews, selection, mute, and solo. The phone inspector
   is a resizable bottom sheet; the desktop inspector is on the left.
2. The first working capture showed an empty phone page because Lead enters
   halfway through Intro. Selection now prefers a populated four-bar page,
   while explicit page controls still allow adding notes to empty bars. Fit
   uses the selected layer's visible note range.
3. A comparison found excessive transport height and clipped inspector actions.
   The phone transport now fits on one row, secondary history controls are in
   its overflow menu, and the inspector actions are visible. File delivery is
   accessible from the inspector's persistent File menu.
4. A focused inspector review and the recaptured full phone screen confirm the
   selected-layer controls, regeneration, variation, level, preset, octave,
   generation lock, all four sections, five layers, keyboard, and velocity strip
   are available. Additional captures show the generated-layer inspectors and
   the open File menu with Open, Save, MIDI import/export, WAV, and delivery.

## Fidelity surfaces

- Typography: bundled Inter is retained; the selected-layer heading has the
  clearest hierarchy. Track names and section bar ranges are readable at the
  actual phone width. Standard egui controls remain rather than rasterized UI.
- Layout: five color-coded rows line up with their note previews. Four section
  buttons span the arrangement. Phone note editing uses four-bar pages; desktop
  shows all eight bars in the selected section. Persistent controls are not
  clipped; generated-layer inspectors can shrink to fit their shorter content.
- Colors: navy surfaces and cyan Bass, green Arpeggio, amber Lead, coral Drums,
  and purple Chords follow the reference. Amber marks the primary action.
- Assets and sharpness: real note events, generated drum hits, and chord
  voicings are rendered as interactive vector UI. Existing Material icons and
  the bundled font are used. No screenshot is used as application chrome.
- Copy and states: generated layers identify their behavior and expose their
  sound controls. They do not offer unsupported regeneration or note editing.
  A locked melodic layer disables regeneration and variation. The reference's
  simultaneously locked Lead and enabled Regenerate state is intentionally
  corrected. Actual score content differs from the illustrative mockup.

## Interaction and regression evidence

All 91 Sound Lab tests passed locally. The final GUI pass includes clicks on
all five arrangement rows, section selection, preservation of locked tracks,
preventing generated-layer shortcuts from changing a previous melody, bass-led
chord preview updates, phone paging, and drawing the workstation at wide and
compact sizes. The generated chord preview also preserves sustained chords
through bass rests.

Native compilation, Clippy with warnings denied, formatting, and the WASM build
passed. Chromium interacted with Music and Sound FX at desktop and phone sizes;
the phone run also selected Chords and Drums and opened File / Export. No browser
console errors or page errors were reported.

## Accepted differences and remaining polish

The desktop layout is an adaptation of the approved mobile visual language;
there is no separate approved desktop raster in this iteration. Native egui
checkboxes, drag values, and slider handles differ from the mockup's custom
switches and circular slider handle. These are cosmetic follow-ups. Real
composition data is denser in the 32-bar overview and sparser in the Intro
melody than the illustration; the visual review does not substitute mock notes
for the actual score.

## Implementation checklist

- [x] Five visible and selectable sounding layers
- [x] Existing sound and composition controls retained
- [x] Generated layers presented with truthful previews and controls
- [x] Phone bottom sheet and desktop left inspector
- [x] Fitted note range, phone paging, and velocity display
- [x] Source/capture comparison and focused inspector review
- [x] Native tests, warnings, WASM build, and real browser smoke checks
- [ ] Repository-wide PR CI on the final commit
