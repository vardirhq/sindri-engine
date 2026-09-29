# Flappy

A tiny side-view arcade genre showcase for Sindri: one-button flight through an endless line of gaps.

The game exists to prove a compact, readable Decay-first arcade loop rather than to reproduce the original Flappy Bird art or branding. Gameplay is authored in the scene and Decay: flap impulse, gravity-driven flight, scrolling obstacles, scoring, collision death, and restart.

## Controls

- **Tap / click / Space / W / Arrow Up** — flap
- **The same, a moment after a crash** — restart

Gaps change height every lap, the course speeds up as the score climbs, and the sky, hills, and ground scroll in parallax.

## Scope

Keep this showcase deliberately small: one scene, a handful of scripts, simple engine-native visuals, and a deterministic harness test that plays through a gap and then crashes. It should be useful as a project somebody can copy when starting a one-button arcade game.
