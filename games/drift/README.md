# DRIFT

A first-person survival and exploration game in zero gravity. The player wakes
aboard the *Tern*, a damaged survey craft adrift in the dark, and has to go
outside to keep it alive. `docs/drift-plan.md` is the design and the order of
work; this is a game rather than a showcase (`AGENTS.md`).

## What plays now

Ships are becoming **block-built and drivable** (see the plan's *Block-built
ships*): a 12 m shuttle of half-metre blocks floats off the wreckage as one
rigid body, and you can stand and walk aboard it while it flies and turns.
Every place with gravity is a *hull* (`assets/scripts/hull.decay`). F at its
seat flies it: W/S, A/D, Space/C fire its thrusters, the mouse and Q/E turn
it, X brakes, F again stands up.

Stage 4, **the wreckage**: past the hold's forward cargo hatch lies a derelict
module, a field of drifting debris that tumbles when you hit it, three salvage
canisters and the stars. G takes salvage in reach; it is stowed when you are
back aboard. The wreck is past the tether's reach: unclip to go.

Stage 3, **the airlock**: you wake in the *Tern*'s cabin under the ship's
gravity and walk to the airlock. E at a panel cycles it: one door closes, the
chamber pumps down, the other opens. Past the outer door is the hold, in
vacuum and zero gravity. The suit's oxygen and power run down outside and fill
up aboard; run out of air and you wake back in the cabin.

| Aboard | Does |
| --- | --- |
| Mouse | Look |
| W / S, A / D | Walk |
| Space | Jump |
| E at a panel | Cycle the airlock |


Stage 1, **floating**: the *Tern*'s cargo hold, with no gravity. The diver is a
physics body that keeps whatever momentum they give it. Just outside the outer
door is the aft handhold, where the tether's loose end waits, looking down the
hold at the handhold on the forward bulkhead.

| Outside | Does |
| --- | --- |
| Mouse | Look (click first to capture the cursor) |
| Q / E | Roll |
| W / S, A / D | Thrust forward/back, left/right |
| Space / C | Thrust up/down |
| X | Brake: thrust against the drift |
| F or right mouse | Hold on to a handhold in reach; keep holding to catch the next one |
| Space or left mouse, holding on | Push off the way you look |
| R (hold) | Reel the tether in; let go and it pays out again |
| U | Unclip from the tether; near its loose end, clip back on |
| G | Take the salvage in reach |

On a phone held sideways the controls are on screen: a move stick on the left,
a view stick on the right, roll buttons, and ACTION, JUMP and CROUCH. ACTION is
whatever you aim at: tap it at the airlock panel, by salvage or at the seat,
and hold it to hold on to a handhold. JUMP is up in space and CROUCH down.
Any finger presses a button while another steers. The tether's reel, clip and
the brake are keys only for now (R, U, X).

Stage 2, **the tether**: a 10 m line of physics links from the aft handhold
to the diver. It goes taut at its length and holds them there, catches on the
stanchion part way down the hold, and reels them home. The far bulkhead is past
the line's reach: unclip to get there, and come back to the loose end to clip
on again.

Thrusters burn propellant (shown bottom left); pushing off and catching
handholds is free, so moving well means moving hand to hand. Looking turns
about the view's own axes, so there is no "up" and no flip when looking past
vertical.

```sh
cargo run -p sindri-player -- games/drift
```

## How it is made

There is no game code in Rust. The wreckage's models are made by
`art/wreck.py` with the repository's GLB writer. `art/blocks.py` builds
the shuttle's block set, with modeled hull plates, deck treads, service
panels, ribs, hazard bands, lights, and vents. Their glTF normal and packed
roughness/metalness maps give the finishes different responses to light.
`art/hold.py` writes the scene and its other textures. The diver is
`assets/scripts/diver.decay`, the airlock `assets/scripts/airlock.decay`
(sharing `Ship` in `assets/scripts/ship.decay`), the tether `assets/scripts/tether.decay`
on the engine's 3D distance joints. `src/lib.rs` is a test harness that plays the
project the way every host does.

```sh
python3 games/drift/art/blocks.py
python3 games/drift/art/hold.py
cargo run --quiet --package decay-lsp -- --check games/drift/assets/scripts/diver.decay
cargo test -p drift
```

`tests/wreckage.rs` takes salvage and stows it, knocks debris away and flies
into the derelict. `tests/culling.rs` measures culled draws and checks the
modeled shuttle's material draw and triangle budgets.
`tests/airlock.rs` walks out through the airlock and back, checks a cycle
waits for each door and the pressure, and runs out of air outside.
`tests/tether.rs` is stopped by the line, reeled home, caught on the
stanchion, and unclips and clips back on. `tests/floating.rs` unclips, pushes off the aft handhold, drifts the length of the hold
and catches the forward one, and checks that thrust is kept when the thrusters
stop and that the brake takes it out again. The editor's
`play_matches_the_build` test plays it the same in the editor as in a build.

Chakra Petch is under the SIL Open Font License (`assets/fonts/Fonts-OFL.txt`).

You start with no thruster pack. Out of the airlock, grab the yellow guide
cable beside the door (F, or hold ACTION) and pull yourself along it into the
ruin (W, or the stick). Lost in the open, Space or JUMP vents suit air for a
small push, at a cost in oxygen.

Air, suit power and speed are read on your wrist, and the airlock's pressure on
its chamber wall. Brackets mark what ACTION will use. Venting air to push costs
oxygen, so spare oxygen canisters float along the way: take one with ACTION or F.
