#!/usr/bin/env python3
"""Build a play page for every playable project and the landing page's grids.

Every entry in `directory.json` with a `play` block gets `/play/<route>/`,
which embeds the exported project at `/examples/<route>/` and says what it is,
how to play it and what it proves. The landing page's game and example grids
are written from the same catalog, so they cannot drift from what is
published. Run after the exports and before `build_directory.py`.
"""

from html import escape
import json
from pathlib import Path
import shutil
import sys

SITE = Path(__file__).resolve().parent
SHOTS = SITE / "screenshots"
KINDS = {
    "Flagship": "flagship",
    "Genre showcase": "genre",
    "Feature example": "examples",
    "Lab": "examples",
    "Tool": "examples",
}
INPUTS = {"keyboard": "Keyboard", "mouse": "Mouse", "touch": "Touch", "gamepad": "Gamepad"}
GROUPS = {
    "flagship": ("Games", "#games"),
    "genre": ("Games", "#games"),
    "examples": ("Examples", "#examples"),
}


def route_of(entry: dict) -> str:
    return entry["path"].split("/")[1]


def playable(entries: list) -> list:
    out = []
    for entry in entries:
        play = entry.get("play")
        if play is None:
            continue
        if not entry["path"].startswith("examples/") or entry["path"].count("/") != 2:
            raise SystemExit(f"A playable entry must be an export route: {entry['path']}")
        if play["kind"] not in KINDS:
            raise SystemExit(f"Unknown kind {play['kind']!r} for {entry['title']}")
        if unknown := set(play.get("input", [])) - set(INPUTS):
            raise SystemExit(f"Unknown input {sorted(unknown)} for {entry['title']}")
        if not play.get("about") or not play.get("proves"):
            raise SystemExit(f"{entry['title']} needs about and proves")
        shot = play.get("shot")
        if shot and not (SHOTS / shot).is_file():
            raise SystemExit(f"Missing screenshot site/screenshots/{shot} for {entry['title']}")
        out.append(entry)
    return out


def thumb(entry: dict, prefix: str) -> str:
    shot = entry["play"].get("shot")
    if not shot:
        return f'<div class="thumb blank" aria-hidden="true"><span>{escape(entry["title"])}</span></div>'
    return (
        f'<div class="thumb"><img src="{prefix}assets/screenshots/{escape(shot)}" alt="" '
        f'loading="lazy" decoding="async"></div>'
    )


def card(entry: dict, prefix: str) -> str:
    play = entry["play"]
    route = route_of(entry)
    role = KINDS[play["kind"]] if play["kind"] != "Tool" else "tool"
    verb = "Open" if play["kind"] == "Tool" else "Play"
    return (
        f'      <article class="card">'
        f'{thumb(entry, prefix)}'
        f'<div class="body"><span class="role {role}">{escape(play["kind"])}</span>'
        f'<h3>{escape(entry["title"])}</h3><p>{escape(entry["description"])}</p>'
        f'<span class="go" aria-hidden="true">{verb} →</span></div>'
        f'<a class="cover" href="{prefix}play/{route}/">'
        f'<span class="sr-only">{verb} {escape(entry["title"])}</span></a></article>'
    )


def feature(entry: dict, index: int) -> str:
    play = entry["play"]
    route = route_of(entry)
    shot = play.get("shot")
    media = (
        f'<img src="./assets/screenshots/{escape(shot)}" alt="{escape(entry["title"])}" loading="lazy">'
        if shot else ""
    )
    chips = "".join(f"<li>{escape(item)}</li>" for item in play["proves"][:5])
    return (
        f'    <article class="feature{" flip" if index % 2 else ""}">\n'
        f'      <a class="feature-media" href="./play/{route}/" tabindex="-1" aria-hidden="true">'
        f'{media}<span class="play-badge">Play in browser</span></a>\n'
        f'      <div class="feature-body"><span class="role flagship">{escape(play["kind"])}</span>'
        f'<h3>{escape(entry["title"])}</h3><p>{escape(play["about"][0])}</p>'
        f'<ul class="chips" aria-label="What it proves">{chips}</ul>'
        f'<div class="actions"><a class="button primary" href="./play/{route}/">Play {escape(entry["title"])}</a>'
        f'<a class="button" href="https://github.com/vardirhq/sindri-engine/tree/main/{escape(entry["source"])}">Source ↗</a></div>'
        f'</div>\n    </article>'
    )


def controls_table(play: dict) -> str:
    rows = play.get("controls", [])
    if not rows:
        return '<div><h2>Controls</h2><p class="open-in-editor">Nothing to press: it plays by itself.</p></div>'
    body = "".join(
        f"<tr><th scope=\"row\">{escape(keys)}</th><td>{escape(action)}</td></tr>" for keys, action in rows
    )
    return f'<div><h2>Controls</h2><table class="controls">{body}</table></div>'


def input_hint(play: dict) -> str:
    inputs = play.get("input", [])
    if not inputs:
        return "Just watch: this one plays by itself."
    names = [INPUTS[i].lower() for i in inputs]
    joined = names[0] if len(names) == 1 else ", ".join(names[:-1]) + " or " + names[-1]
    hint = f"Plays with {joined}."
    if "keyboard" in inputs:
        hint += '<span class="keys-hint"> Click the game to give it the keyboard.</span>'
    return hint


def page(template: str, entry: dict, more: list) -> str:
    play = entry["play"]
    group = KINDS[play["kind"]]
    label, anchor = GROUPS[group]
    shot = play.get("shot")
    title = escape(entry["title"])
    values = {
        "title": title,
        "description": escape(entry["description"]),
        "path": escape(entry["path"]),
        "source": escape(entry["source"]),
        "kind": escape(play["kind"]),
        "role_class": "flagship" if group == "flagship" else ("tool" if play["kind"] == "Tool" else ""),
        "group_label": label,
        "group_anchor": anchor,
        "needs": escape(play.get("needs", "WebGPU")),
        "inputs": escape(", ".join(INPUTS[i] for i in play.get("input", [])) or "None, it plays by itself"),
        "input_hint": input_hint(play),
        "og_image": (
            f'<meta property="og:image" content="../../assets/screenshots/{escape(shot)}">' if shot else ""
        ),
        "poster_class": "" if shot else "blank",
        "poster": (
            f'<img src="../../assets/screenshots/{escape(shot)}" alt="" decoding="async">'
            if shot else f'<span class="poster-title" aria-hidden="true">{title}</span>'
        ),
        "about": "\n      ".join(f"<p>{escape(p)}</p>" for p in play["about"]),
        "note": f'<p class="note">{escape(play["note"])}</p>' if play.get("note") else "",
        "proves": "".join(f"<li>{escape(item)}</li>" for item in play["proves"]),
        "controls": controls_table(play),
        "more": "\n".join(card(other, "../../") for other in more),
    }
    out = template
    for key, value in values.items():
        out = out.replace("{{" + key + "}}", value)
    if "{{" in out:
        raise SystemExit(f"Unfilled placeholder in the play page for {entry['title']}")
    return out


def neighbours(entries: list, index: int, count: int = 4) -> list:
    """The next few entries of the same group, wrapping, then anything else."""
    group = KINDS[entries[index]["play"]["kind"]]
    ring = entries[index + 1:] + entries[:index]
    same = [e for e in ring if KINDS[e["play"]["kind"]] == group]
    rest = [e for e in ring if e not in same]
    return (same + rest)[:count]


def build(root: Path, verify_exports: bool = True) -> None:
    entries = json.loads((SITE / "directory.json").read_text())
    games = playable(entries)
    template = (SITE / "play.html").read_text()
    for index, entry in enumerate(games):
        route = route_of(entry)
        if verify_exports and not (root / entry["path"] / "index.html").is_file():
            raise SystemExit(f"{entry['title']} has a play page but no export at {entry['path']}")
        destination = root / "play" / route / "index.html"
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(page(template, entry, neighbours(games, index)))

    landing = root / "index.html"
    html = landing.read_text()
    by_group = {group: [e for e in games if KINDS[e["play"]["kind"]] == group] for group in set(KINDS.values())}
    replacements = {
        "<!-- PLAY:flagships -->": "\n".join(feature(e, i) for i, e in enumerate(by_group["flagship"])),
        "<!-- PLAY:genre -->": "\n".join(card(e, "./") for e in by_group["genre"]),
        "<!-- PLAY:examples -->": "\n".join(card(e, "./") for e in by_group["examples"]),
        "<!-- COUNT:games -->": str(sum(e["category"] == "games" for e in games)),
        "<!-- COUNT:demos -->": str(sum(e["category"] != "games" for e in games)),
    }
    for marker, value in replacements.items():
        if marker not in html:
            raise SystemExit(f"The landing page lacks {marker}")
        html = html.replace(marker, value)
    landing.write_text(html)

    (root / "assets").mkdir(parents=True, exist_ok=True)
    for asset in ("play.css", "play.js"):
        shutil.copyfile(SITE / asset, root / "assets" / asset)
    print(f"Built {len(games)} play pages and the landing page's grids")


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    build(
        Path(args[0]) if args else SITE.parent / "target/pages",
        verify_exports="--no-verify-exports" not in sys.argv,
    )
