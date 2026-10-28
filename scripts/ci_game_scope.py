#!/usr/bin/env python3
"""Select a safe single-game PR scope; unknown or shared changes run in full."""
import os
import subprocess
import sys

# Only games with their own Rust test package and project export are scoped.
GAMES = {
    'causeway': ('game/', 'sindri-causeway'),
    'flappy': ('games/flappy/', 'flappy'),
    'platformer': ('games/platformer/', 'platformer'),
    'lamplighter': ('games/lamplighter/', 'lamplighter'),
    'low-tide': ('games/low-tide/', 'low-tide'),
    'orbital-baked': ('games/orbital-baked/', 'orbital-baked'),
    'scorchball': ('games/scorchball/', 'scorchball'),
    'weave-poc': ('games/weave-poc/', 'weave-poc'),
}
CAUSEWAY_TOOLS = {'tools/causeway_art.py', 'tools/level.py', 'tools/check_causeway_art.py'}


def classify(paths):
    """Return (game, package), or None if this is not one game alone."""
    found = set()
    for path in paths:
        if path == 'CHANGELOG.md':
            continue
        if path in CAUSEWAY_TOOLS:
            found.add('causeway')
            continue
        match = next((game for game, (prefix, _) in GAMES.items() if path.startswith(prefix)), None)
        if match is None:
            return None
        # Cargo manifests affect dependency resolution and cache safety. Build
        # scripts can affect compilation outside the game; both require full CI.
        if path.rsplit('/', 1)[-1] in {'Cargo.toml', 'Cargo.lock', 'build.rs'}:
            return None
        found.add(match)
    if len(found) != 1:
        return None
    game = found.pop()
    return game, GAMES[game][1]


def main():
    if len(sys.argv) != 2:
        raise SystemExit('usage: ci_game_scope.py BASE')
    try:
        result = subprocess.run(
            ['git', 'diff', '--name-only', '--no-renames', '-z', sys.argv[1], 'HEAD'],
            check=True, stdout=subprocess.PIPE,
        )
        paths = [os.fsdecode(path) for path in result.stdout.split(b'\0') if path]
        selected = classify(paths)
    except (OSError, subprocess.CalledProcessError):
        selected = None
    game, package = selected if selected else ('', '')
    values = f'light={str(bool(selected)).lower()}\ngame={game}\npackage={package}\n'
    print(values, end='')
    if output := os.environ.get('GITHUB_OUTPUT'):
        with open(output, 'a', encoding='utf-8') as out:
            out.write(values)


if __name__ == '__main__':
    main()
