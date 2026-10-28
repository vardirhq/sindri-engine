"""The fast path must never classify shared or ambiguous changes as one game."""
import unittest
import os
from pathlib import Path
import subprocess
import sys
import tempfile

from scripts.ci_game_scope import classify


class ScopeTests(unittest.TestCase):
    def test_each_game_and_changelog(self):
        for path, game, package in [
            ('game/assets/main.scene', 'causeway', 'sindri-causeway'),
            ('games/flappy/assets/main.scene', 'flappy', 'flappy'),
            ('games/platformer/assets/level-1.scene', 'platformer', 'platformer'),
            ('games/lamplighter/assets/world.scene', 'lamplighter', 'lamplighter'),
            ('games/low-tide/assets/main.scene', 'low-tide', 'low-tide'),
            ('games/orbital-baked/assets/main.scene', 'orbital-baked', 'orbital-baked'),
            ('games/scorchball/assets/main.scene', 'scorchball', 'scorchball'),
            ('games/weave-poc/assets/main.scene', 'weave-poc', 'weave-poc'),
        ]:
            with self.subTest(game=game):
                self.assertEqual(classify([path, 'CHANGELOG.md']), (game, package))

    def test_causeway_generators(self):
        self.assertEqual(classify(['tools/level.py', 'game/tests/run.rs']), ('causeway', 'sindri-causeway'))

    def test_shared_and_ambiguous_always_full(self):
        for paths in [[], ['CHANGELOG.md'], ['games/platformer/assets/x', 'games/low-tide/assets/x'],
                      ['games/platformer/assets/x', 'crates/sindri-render/src/lib.rs'],
                      ['games/platformer/Cargo.toml'], ['games/platformer/build.rs'],
                      ['games/platformer/assets/x', 'docs/parity.md'], ['games/unknown/assets/x'],
                      ['games/shapes-lab/assets/x'], ['games/voxel-lab/src/lib.rs']]:
            with self.subTest(paths=paths):
                self.assertIsNone(classify(paths))

    def test_rename_across_boundaries_is_full(self):
        self.assertIsNone(classify(['games/platformer/assets/a', 'games/low-tide/assets/b']))

    def test_git_diff_rename_and_failed_base_are_conservative(self):
        script = Path(__file__).with_name('ci_game_scope.py').resolve()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(['git', 'init', '-q'], cwd=root, check=True)
            subprocess.run(['git', 'config', 'user.email', 'ci@example.invalid'], cwd=root, check=True)
            subprocess.run(['git', 'config', 'user.name', 'CI test'], cwd=root, check=True)
            old = root / 'games/platformer/assets/old.scene'
            old.parent.mkdir(parents=True)
            old.write_text('scene')
            subprocess.run(['git', 'add', '.'], cwd=root, check=True)
            subprocess.run(['git', 'commit', '-qm', 'base'], cwd=root, check=True)
            new = root / 'games/low-tide/assets/new.scene'
            new.parent.mkdir(parents=True)
            old.rename(new)
            subprocess.run(['git', 'add', '-A'], cwd=root, check=True)
            subprocess.run(['git', 'commit', '-qm', 'move'], cwd=root, check=True)
            env = dict(os.environ)
            env.pop('GITHUB_OUTPUT', None)
            for base in ('HEAD^', 'missing-ref'):
                result = subprocess.run([sys.executable, str(script), base], cwd=root, env=env,
                                        check=True, capture_output=True, text=True)
                self.assertIn('light=false', result.stdout)


if __name__ == '__main__':
    unittest.main()
