# DRIFT

A first-person survival and exploration game in zero gravity. The player wakes
aboard the *Tern*, a damaged survey craft adrift in the dark, and has to go
outside to keep it alive. `docs/drift-plan.md` is the design and the order of
work; this is a game rather than a showcase (`AGENTS.md`).

## What plays now

Stage 1, **floating**: the *Tern*'s cargo hold, with no gravity. The diver is a
physics body that keeps whatever momentum they give it. They wake holding the
aft handhold, looking down the hold at the one on the forward bulkhead.

| Input | Does |
| --- | --- |
| Mouse | Look (click first to capture the cursor) |
| Q / E | Roll |
| W / S, A / D | Thrust forward/back, left/right |
| Space / C | Thrust up/down |
| X | Brake: thrust against the drift |
| F or right mouse | Hold on to a handhold in reach; keep holding to catch the next one |
| Space or left mouse, holding on | Push off the way you look |

Thrusters burn propellant (shown bottom left); pushing off and catching
handholds is free, so moving well means moving hand to hand. Looking turns
about the view's own axes, so there is no "up" and no flip when looking past
vertical.

```sh
cargo run -p sindri-player -- games/drift
```

## How it is made

There is no game code in Rust. The scene is written by `art/hold.py`, which
also draws the textures; re-running it changes nothing. The movement is
`assets/scripts/diver.decay`. `src/lib.rs` is a test harness that plays the
project the way every host does.

```sh
python3 games/drift/art/hold.py
cargo run --quiet --package decay-lsp -- --check games/drift/assets/scripts/diver.decay
cargo test -p drift
```

`tests/floating.rs` pushes off the aft handhold, drifts the length of the hold
and catches the forward one, and checks that thrust is kept when the thrusters
stop and that the brake takes it out again. The editor's
`play_matches_the_build` test plays it the same in the editor as in a build.

Chakra Petch is under the SIL Open Font License (`assets/fonts/Fonts-OFL.txt`).
