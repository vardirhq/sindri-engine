#!/usr/bin/env python3
"""Build the directory and require coverage of every published project and doc."""

import html
import json
from pathlib import Path
import shutil
import sys

SITE = Path(__file__).resolve().parent
LABELS = {"games": "Game", "demos": "Feature demo", "docs": "Documentation"}


def build(root: Path) -> None:
    entries = json.loads((SITE / "directory.json").read_text())
    paths = [entry["path"] for entry in entries]
    if len(paths) != len(set(paths)):
        raise SystemExit("Duplicate directory routes")
    published = {
        str(page.parent.relative_to(root)) + "/"
        for group in ("examples", "docs")
        for page in (root / group).glob("*/index.html")
    }
    if set(paths) != published:
        raise SystemExit(
            f"Directory route mismatch: unlisted={sorted(published - set(paths))}, "
            f"unpublished={sorted(set(paths) - published)}"
        )
    cards = []
    for entry in entries:
        category = entry["category"]
        if category not in LABELS or not entry["path"].startswith(("examples/", "docs/")):
            raise SystemExit(f"Invalid directory entry: {entry}")
        source = entry["source"]
        if ".." in Path(source).parts or source.startswith(("/", "http:")):
            raise SystemExit(f"Invalid source path: {source}")
        escape = html.escape
        terms = " ".join([entry["title"], entry["description"], *entry["tags"]]).lower()
        tags = "".join(f"<li>{escape(tag)}</li>" for tag in entry["tags"])
        action = "Read" if category == "docs" else "Launch"
        source_link = "https://github.com/vardirhq/sindri-engine/" + (
            "blob/main/" if category == "docs" else "tree/main/"
        ) + source
        cards.append(
            f'<article class="directory-entry" data-category="{category}" '
            f'data-search="{escape(terms)}">'
            f'<span class="entry-category">{LABELS[category]}</span>'
            f'<h2>{escape(entry["title"])}</h2>'
            f'<p>{escape(entry["description"])}</p>'
            f'<ul class="entry-tags" aria-label="Topics">{tags}</ul>'
            f'<div class="entry-actions">'
            f'<a href="../{escape(entry["path"])}">{action} →'
            f'<span class="sr-only"> {escape(entry["title"])}</span></a>'
            f'<a href="{escape(source_link)}">source ↗'
            f'<span class="sr-only"> for {escape(entry["title"])}</span></a>'
            f'</div></article>'
        )
    template = (SITE / "directory.html").read_text()
    page = template.replace("<!-- ENTRIES -->", "\n".join(cards))
    page = page.replace("23 entries", f"{len(entries)} entries")
    (root / "directory").mkdir(parents=True, exist_ok=True)
    (root / "directory/index.html").write_text(page)
    (root / "assets").mkdir(parents=True, exist_ok=True)
    for asset in ("directory.css", "directory.js"):
        shutil.copyfile(SITE / asset, root / "assets" / asset)
    print(f"Built directory with {len(entries)} verified routes")


if __name__ == "__main__":
    build(Path(sys.argv[1]) if len(sys.argv) > 1 else SITE.parent / "target/pages")
