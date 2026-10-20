# Orbital Last Stand: complete-game edition

The October 2026 direction moves beyond reference parity: make Orbital a complete,
cohesive game worth returning to. It remains Sindri's forcing function. All game
rules live in Decay, content in profiles/scenes/prefabs, and presentation in Weave.
A general missing capability is implemented in the engine, never in a bespoke host.

## Game direction

A lone pilot holds an evacuation corridor for ten minutes. Five sectors unfold
as a build grows from one gun into an improbable constellation. Distinct ships
are sidegrades; salvage unlocks options rather than permanent damage inflation.
Boss Rush is a finite twelve-boss gauntlet with its own victory condition.

The visual direction is midnight navy, luminous mint instrumentation, warm ivory
typography and copper planetary light. Vector threats stay legible over a quiet
space backdrop. The hangar, archive, records, settings and debrief use the same
responsive interface on desktop, keyboard/controller and touch.

## Acceptance

- [x] Mission-control title, responsive hangar, archive, records and settings.
- [x] Four distinct playable hulls, persistent unlocks and three difficulties.
- [x] Career currency, commendations, discoveries and a bounded sortie history.
- [x] Complete debrief, repeat sortie and return-to-hangar flows.
- [x] Evasive burst with cooldown and keyboard/controller/touch controls.
- [x] First-flight guidance, explicit mission progress and touch pause.
- [x] Finite Boss Rush, clean replay and all existing combat content retained.
- [x] Three composed score tracks, context crossfades and new burst/victory cues.
- [x] Nineteen effective synergy recipes and honest module descriptions.
- [x] Original key art, distinct ship art and richer restrained arena treatment.
- [x] Persistent mixer, reduced motion and reduced flash settings.
- [x] Typed script checks, deterministic lifecycle/progression regressions.
- [x] Native captures and real exported WebGPU review at desktop and HiDPI
      touch phone sizes (`scripts/browser/orbital-complete.mjs`).
- [ ] Hands-on review inside the editor's Play.

Shared text saves are the first general engine gap found by this edition:
`SaveStore` already stores Unicode text, but Decay could only reach numbers and
flags. The public text calls close that gap for pilot names and sortie records.

## Scope and evidence

This is a game release built on existing Sindri capabilities, with text-save
access the general capability found by the game. Historical parity plans remain
as a record of the recreation; catalog descriptions now state implemented combat
effects rather than promising rooms or shops the game does not contain.

The production campaign remains 600 seconds. `the_complete_game` shortens the
authored mission clock for its goal/replay regression; `boss_rush` clears the
actual roster in order and verifies the twelfth kill ends the gauntlet. Tests
also cover settlement idempotence, purchases, saved pilot names and bindings,
controller movement, burst cooldown, difficulty, every synergy recipe, and the
Black Sun's authored physics attack. Layout tests cover five aspect ratios.

Completed career progress persists on this device. There is no mid-sortie world
snapshot, online leaderboard, cloud save or multiplayer in this edition.
