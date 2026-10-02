# Sequence Stage

A feature example for sequences and the editor's Timeline. Open `sindri.toml`
in the editor or run the exported project in a WebGPU browser.

The Director entity carries a `sindri.sequence` with two sequences. **intro**
plays when the scene starts: the camera starts close and pulls back as the
ship flies in and swings upright, the ring blooms around it, and the title and
hint fade up. The camera is not a child of the Director; its track names it
from the top of the scene, `/camera`, which is how one director drives
anything in a scene. Its cues play a launch
sound, a landing click and mark the moment it is ready. **idle** loops after
it: the ship bobs and the ring breathes. Space or a tap replays the intro.

Select the Director and open the Timeline panel to see both. Every track is
authored there, not in a script: `assets/scripts/director.decay` only says
which sequence plays and prints the cues it hears with `Sequence.cued`. The
scene is saved at the intro's finished look, so the editor shows the stage;
turn on the Timeline's preview and click the ruler to see any moment of it.

`game/tests/the_sequence_demo_works.rs` opens the project, compiles its
script and plays it through the session every export runs: the ship lands,
the title fades up, the stage moves on to its idle loop, and Space replays.

See [the Sequence surface](../../docs/scripting.md#sequence).
