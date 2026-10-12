#!/usr/bin/env python3
"""Create the staging directory index for an approved PR preview."""
from html import escape
from pathlib import Path
import sys

root, number, sha = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
root.mkdir(parents=True, exist_ok=True)
(root / "index.html").write_text(
    '<!doctype html><html lang="en"><meta charset="utf-8">'
    '<meta name="viewport" content="width=device-width,initial-scale=1">'
    '<title>Sindri WIP previews</title>'
    '<style>body{font:1.1rem system-ui;max-width:50rem;margin:4rem auto;'
    'padding:0 1.5rem;background:#111827;color:#f9fafb}'
    'a{color:#93c5fd}</style>'
    '<h1>Work in progress</h1>'
    '<p>These are unmerged, CI-verified development previews, not stable releases.</p>'
    f'<p><a href="./{escape(number)}/">PR #{escape(number)}: 3D labs</a> '
    f'(<code>{escape(sha[:12])}</code>)</p></html>',
    encoding="utf-8",
)
