# Focused game pull request checks

A pull request changing one supported game can use a focused path. It runs
format and repository checks, typed Decay checks for changed scripts, Clippy
and the native tests for that game's package, its relevant art regeneration,
and an exported phone-viewport browser smoke of the game. The Pages workflow
builds a PR preview containing the changed game and the existing lab routes.
It does not build or smoke the rest of the site on that pull request.

Supported projects: Causeway (`game/`), Flappy, Lamplighter, Low Tide,
Orbital Last Stand (`orbital-baked`), Scorchball and Weave PoC. The classifier
also accepts Causeway's dedicated generators and `CHANGELOG.md`. A game
plus shared engine, workflow or documentation changes, more than one game,
Cargo manifest or build-script changes, and unknown paths use the full path.
Renames count both the old and new paths. Shapes Lab and Voxel Lab retain the
full path until their distinct browser hosts have a dedicated focused check.

The classifier errs toward a full run if the diff cannot be read. It only
selects a focused run on pull requests (or a branch workflow dispatch such as an
autofix rerun). The merge queue and main still run the full suite, and focused
runs do not leave a record eligible for full-suite reuse. The scope decision
is shared by CI and the PR Pages build in `scripts/ci_game_scope.py`, with
regression coverage in `scripts/test_ci_game_scope.py`.

A focused browser smoke checks that the changed export starts and renders; it
does not replace the broader browser matrix or a human gameplay review.
