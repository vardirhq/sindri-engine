#!/usr/bin/env python3
"""Plan and assemble the published Sindri site: stable routes plus WIP previews.

GitHub Pages replaces the whole site on every deployment, so every deployment
is built from the full desired state instead of a delta:

* the newest stable site that a successful `Sindri Pages` run on `main`
  uploaded as the `stable-site` artifact, and
* one `/wip/<pr>/` directory per open pull request from this repository whose
  *current head* passed every pull request workflow and whose `Sindri Pages`
  run uploaded a non-expired `3d-labs-preview` artifact.

Because each run recomputes everything from live GitHub state, an older queued
run that concurrency cancels loses nothing, a later `main` deployment keeps
every active preview, and a merged or closed pull request drops out by itself.

Nothing here executes pull request code. Preview artifacts are only unpacked,
checked for unsafe entries and copied below `/wip/<pr>/`.

Usage:
  wip_previews.py plan --repo OWNER/REPO [--event-run ID] [--pr N]
                       [--site-url URL] [--plan-out FILE]
  wip_previews.py assemble --repo OWNER/REPO --plan FILE --out DIR
"""
from __future__ import annotations

import argparse
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from html import escape
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import urllib.request
from typing import Callable, Iterable

PREVIEW_ARTIFACT = "3d-labs-preview"
STABLE_ARTIFACT = "stable-site"
PAGES_WORKFLOW = ".github/workflows/pages.yml"
# Workflows a preview head must have run and passed. Every other pull request
# workflow that ran for the head (path-filtered ones such as Decay) must also
# have passed, but its absence is not an error: GitHub never creates a run for
# a workflow whose path filter does not match.
REQUIRED_WORKFLOWS = {".github/workflows/ci.yml": "CI", PAGES_WORKFLOW: "Sindri Pages"}
GREEN = {"success", "skipped", "neutral"}
# Routes every preview bundle must contain, each with a browser player.
PREVIEW_ROUTES = ("transform", "orbit", "character3d", "model-collision")


@dataclass
class Verdict:
    """Whether one pull request's current head may be published, and why."""

    pr: int
    title: str = ""
    sha: str = ""
    status: str = "rejected"  # ready | pending | rejected | ignored
    reason: str = ""
    run_id: int = 0
    artifact_id: int = 0
    checks: list[str] = field(default_factory=list)

    @property
    def ready(self) -> bool:
        return self.status == "ready"


# --------------------------------------------------------------------------- #
# GitHub access


def gh_api(path: str):
    """Call the REST API through the `gh` CLI (GH_TOKEN from the workflow)."""
    out = subprocess.run(
        ["gh", "api", "-H", "Accept: application/vnd.github+json", path],
        check=True, capture_output=True, text=True,
    )
    return json.loads(out.stdout)


def paged(api: Callable, path: str, key: str | None, limit: int = 1000) -> list:
    """Collect every page of a list endpoint (`key` names the wrapped list)."""
    items: list = []
    sep = "&" if "?" in path else "?"
    page = 1
    while len(items) < limit:
        body = api(f"{path}{sep}per_page=100&page={page}")
        chunk = body[key] if key else body
        items.extend(chunk)
        if len(chunk) < 100:
            break
        page += 1
    return items


# --------------------------------------------------------------------------- #
# Verification


def verify_pr(api: Callable, repo: str, pr: dict) -> Verdict:
    """Decide whether `pr` (a pulls API object) is publishable right now.

    Workflow runs are matched on the pull request *head* SHA. A `pull_request`
    run checks out the synthetic merge commit (`refs/pull/N/merge`, the PR's
    `merge_commit_sha`), but the run object, its check runs and its artifacts
    are all recorded against the head SHA, and only the head SHA identifies the
    revision the author pushed. A merge SHA never matches any run.
    """
    verdict = Verdict(pr=pr["number"], title=pr.get("title", ""), sha=pr["head"]["sha"])
    sha = verdict.sha
    if pr["state"] != "open":
        verdict.status, verdict.reason = "ignored", f"pull request is {pr['state']}"
        return verdict
    head_repo = (pr["head"].get("repo") or {}).get("full_name")
    if head_repo != repo:
        verdict.status = "ignored"
        verdict.reason = f"head is in {head_repo or 'a deleted fork'}; fork previews are never published"
        return verdict

    runs = [
        run for run in paged(api, f"repos/{repo}/actions/runs?head_sha={sha}&event=pull_request", "workflow_runs")
        if run["head_sha"] == sha and (run.get("head_repository") or {}).get("full_name") == repo
    ]
    if not runs:
        verdict.reason = (
            f"no pull_request workflow ran for head {sha[:12]}. GitHub creates no runs when the "
            "pull request conflicted with its base when this head was pushed, or when a workflow "
            "token pushed it; push a commit or use 'Update branch' to start CI for this head"
        )
        return verdict

    # Re-runs reuse the run id; a new run of the same workflow supersedes older ones.
    latest: dict[str, dict] = {}
    for run in sorted(runs, key=lambda r: (r["created_at"], r["id"])):
        latest[run["path"]] = run
    for run in sorted(latest.values(), key=lambda r: r["name"]):
        verdict.checks.append(f"{run['name']}: {run['conclusion'] or run['status']} (run {run['id']})")

    missing = [name for path, name in REQUIRED_WORKFLOWS.items() if path not in latest]
    unfinished = [r["name"] for r in latest.values() if r["status"] != "completed"]
    failed = [
        f"{r['name']} ({r['conclusion']})" for r in latest.values()
        if r["status"] == "completed" and r["conclusion"] not in GREEN
    ]
    if failed:
        verdict.reason = "failed checks on head: " + ", ".join(sorted(failed))
        return verdict
    if unfinished:
        verdict.status = "pending"
        verdict.reason = "waiting for " + ", ".join(sorted(unfinished))
        return verdict
    if missing:
        verdict.reason = "required workflow never ran for head: " + ", ".join(missing)
        return verdict

    # Check runs from other apps and legacy commit statuses can fail a head
    # whose Actions workflows all passed.
    foreign = [
        c for c in paged(api, f"repos/{repo}/commits/{sha}/check-runs?filter=latest", "check_runs")
        if (c.get("app") or {}).get("slug") != "github-actions"
    ]
    bad = [f"{c['name']} ({c['conclusion']})" for c in foreign
           if c["status"] == "completed" and c["conclusion"] not in GREEN]
    if bad:
        verdict.reason = "failed external checks on head: " + ", ".join(bad)
        return verdict
    if any(c["status"] != "completed" for c in foreign):
        verdict.status = "pending"
        verdict.reason = "waiting for external checks: " + ", ".join(
            c["name"] for c in foreign if c["status"] != "completed")
        return verdict
    status = api(f"repos/{repo}/commits/{sha}/status")
    if status.get("total_count") and status.get("state") != "success":
        verdict.status = "pending" if status["state"] == "pending" else "rejected"
        verdict.reason = f"commit status is {status['state']}"
        return verdict

    pages = latest[PAGES_WORKFLOW]
    artifacts = [
        a for a in paged(api, f"repos/{repo}/actions/runs/{pages['id']}/artifacts", "artifacts")
        if a["name"] == PREVIEW_ARTIFACT
    ]
    live = [a for a in artifacts if not a["expired"]
            and (a.get("workflow_run") or {}).get("head_sha", sha) == sha]
    if not live:
        verdict.status = "ignored" if not artifacts else "rejected"
        verdict.reason = (
            f"Sindri Pages run {pages['id']} uploaded no {PREVIEW_ARTIFACT} artifact"
            if not artifacts else
            f"{PREVIEW_ARTIFACT} from Sindri Pages run {pages['id']} has expired; re-run it"
        )
        return verdict
    artifact = max(live, key=lambda a: a["id"])
    verdict.status, verdict.reason = "ready", "all checks passed on head"
    verdict.run_id, verdict.artifact_id = pages["id"], artifact["id"]
    return verdict


def latest_stable(api: Callable, repo: str, branch: str = "main") -> dict | None:
    """The newest successful Sindri Pages run on `branch` with a stable site."""
    runs = api(
        f"repos/{repo}/actions/workflows/pages.yml/runs?branch={branch}&status=success&per_page=20"
    )["workflow_runs"]
    for run in runs:
        if run["event"] == "pull_request" or run["head_branch"] != branch:
            continue
        if (run.get("head_repository") or {}).get("full_name") != repo:
            continue
        for artifact in paged(api, f"repos/{repo}/actions/runs/{run['id']}/artifacts", "artifacts"):
            if artifact["name"] == STABLE_ARTIFACT and not artifact["expired"]:
                return {"run_id": run["id"], "sha": run["head_sha"], "artifact_id": artifact["id"]}
    return None


def plan(api: Callable, repo: str, live: dict | None, requested: int | None = None,
         event_run: int | None = None) -> dict:
    """Compute the desired site and whether it differs from what is live."""
    notes: list[str] = []
    trigger = None
    if event_run:
        run = api(f"repos/{repo}/actions/runs/{event_run}")
        trigger = {"id": run["id"], "name": run["name"], "event": run["event"],
                   "sha": run["head_sha"], "branch": run["head_branch"],
                   "conclusion": run["conclusion"],
                   "prs": [p["number"] for p in run.get("pull_requests") or []]}
        notes.append(
            f"Triggered by {run['name']} run {run['id']} ({run['event']}, "
            f"{run['head_sha'][:12]}, {run['conclusion']})."
        )

    prs = paged(api, f"repos/{repo}/pulls?state=open", None)
    if requested and all(p["number"] != requested for p in prs):
        prs.append(api(f"repos/{repo}/pulls/{requested}"))
    verdicts = [verify_pr(api, repo, pr) for pr in sorted(prs, key=lambda p: p["number"])]
    if trigger and trigger["event"] == "pull_request":
        heads = {v.sha: v.pr for v in verdicts}
        if trigger["sha"] not in heads:
            notes.append(
                f"Run {trigger['id']} tested {trigger['sha'][:12]}, which is no longer the head "
                "of any open pull request; its results are ignored."
            )

    stable = latest_stable(api, repo)
    ready = [v for v in verdicts if v.ready]
    desired = {
        "stable": stable,
        "previews": [{"pr": v.pr, "sha": v.sha, "run_id": v.run_id,
                      "artifact_id": v.artifact_id, "title": v.title} for v in ready],
    }
    deploy, why = decide(desired, live, verdicts)
    if requested:
        mine = next(v for v in verdicts if v.pr == requested)
        if not mine.ready:
            notes.append(f"Requested PR #{requested} is not publishable: {mine.reason}.")
    return {
        "deploy": deploy, "why": why, "desired": desired, "trigger": trigger,
        "requested": requested, "verdicts": [asdict(v) for v in verdicts], "notes": notes,
    }


def _key(manifest: dict | None):
    if not manifest:
        return None
    stable = manifest.get("stable") or {}
    return (stable.get("run_id"),
            tuple(sorted((p["pr"], p["sha"], p["artifact_id"]) for p in manifest.get("previews", []))))


def decide(desired: dict, live: dict | None, verdicts: Iterable[Verdict]) -> tuple[bool, str]:
    """Deploy only when it changes what visitors get, and never without stable routes."""
    if not desired["stable"]:
        return False, (
            f"no successful Sindri Pages run on main has a non-expired {STABLE_ARTIFACT} "
            "artifact; refusing to deploy a site without its stable routes"
        )
    if live is None:
        return True, "the live site has no readable /wip/manifest.json"
    if _key(desired) == _key(live):
        return False, "the live site already serves exactly this stable build and these previews"
    if (desired["stable"]["run_id"] == (live.get("stable") or {}).get("run_id")):
        pending = {v.pr for v in verdicts if v.status == "pending"}
        live_prs = {p["pr"]: p for p in live.get("previews", [])}
        wanted = {p["pr"]: p for p in desired["previews"]}
        dropped = set(live_prs) - set(wanted)
        changed = {pr for pr in wanted if pr not in live_prs or
                   (live_prs[pr]["sha"], live_prs[pr]["artifact_id"]) !=
                   (wanted[pr]["sha"], wanted[pr]["artifact_id"])}
        if not changed and dropped and dropped <= pending:
            return False, (
                "only previews whose new head is still being checked would change "
                f"({', '.join(f'#{n}' for n in sorted(dropped))}); leaving the live site as is"
            )
    return True, "the desired site differs from the live site"


def fetch_live(site_url: str) -> dict | None:
    try:
        # The query string sidesteps the CDN's cached copy of an older deploy.
        url = f"{site_url.rstrip('/')}/wip/manifest.json?run={os.environ.get('GITHUB_RUN_ID', '0')}"
        with urllib.request.urlopen(url, timeout=20) as response:
            return json.load(response)
    except Exception as error:  # noqa: BLE001 - any failure means "unknown"
        print(f"::notice::Could not read the live WIP manifest ({error}); a deploy is needed.",
              file=sys.stderr)
        return None


# --------------------------------------------------------------------------- #
# Reporting


ICON = {"ready": "✅", "pending": "⏳", "rejected": "❌", "ignored": "➖"}


def summary(result: dict) -> str:
    lines = ["## Sindri site publisher", ""]
    lines += [f"- {note}" for note in result["notes"]]
    stable = result["desired"]["stable"]
    lines.append(
        f"- Stable site: Sindri Pages run {stable['run_id']} on main ({stable['sha'][:12]})."
        if stable else f"- Stable site: **none available** ({STABLE_ARTIFACT} artifact missing)."
    )
    lines.append(f"- **{'Deploying' if result['deploy'] else 'Not deploying'}**: {result['why']}.")
    lines += ["", "| PR | Head | Verdict | Reason |", "|---|---|---|---|"]
    for v in result["verdicts"]:
        lines.append(
            f"| #{v['pr']} | `{v['sha'][:12]}` | {ICON[v['status']]} {v['status']} | "
            f"{v['reason'].replace('|', '/')} |"
        )
    for v in result["verdicts"]:
        if v["checks"]:
            lines += ["", f"<details><summary>#{v['pr']} workflow runs</summary>", ""]
            lines += [f"- {c}" for c in v["checks"]] + ["", "</details>"]
    return "\n".join(lines) + "\n"


def annotate(result: dict) -> None:
    """Workflow-command annotations so a skipped deploy is never a silent success."""
    for v in result["verdicts"]:
        level = "notice"
        if v["status"] == "rejected" and (
            v["pr"] == result["requested"] or v["pr"] in (result["trigger"] or {}).get("prs", [])
        ):
            level = "warning"
        if v["status"] != "ignored" or v["pr"] == result["requested"]:
            print(f"::{level} title=PR #{v['pr']} {v['status']}::{v['reason']}")
    print(f"::notice title={'Deploying' if result['deploy'] else 'Not deploying'}::{result['why']}")


# --------------------------------------------------------------------------- #
# Assembly


def safe_extract_check(root: Path) -> None:
    """Reject symlinks and anything that is not a regular file or directory."""
    for path in root.rglob("*"):
        if path.is_symlink() or not (path.is_file() or path.is_dir()):
            raise SystemExit(f"Refusing unsafe artifact entry: {path}")
        resolved = path.resolve()
        if root.resolve() not in resolved.parents:
            raise SystemExit(f"Refusing artifact entry outside its directory: {path}")


def check_preview(root: Path, pr: int, base_path: str = "") -> None:
    """A preview must contain its landing page and every lab with a player."""
    problems = []
    if not (root / "index.html").is_file():
        problems.append("index.html")
    for route in PREVIEW_ROUTES:
        for needed in ("index.html", "assets/sindri.manifest",
                       "pkg/sindri_player.js", "pkg/sindri_player_bg.wasm"):
            if not (root / route / needed).is_file():
                problems.append(f"{route}/{needed}")
        page = root / route / "index.html"
        base = f'<base href="{base_path}/wip/{pr}/{route}/">'
        if page.is_file() and base not in page.read_text(encoding="utf-8", errors="replace"):
            problems.append(f"{route}/index.html lacks {base}")
    if problems:
        raise SystemExit(f"Preview for PR #{pr} is incomplete: " + ", ".join(problems))


def write_index(wip: Path, previews: list[dict], stable: dict) -> None:
    items = "".join(
        f'<li><a href="./{p["pr"]}/">PR #{p["pr"]}: {escape(p["title"] or "preview")}</a> '
        f'<code>{escape(p["sha"][:12])}</code></li>'
        for p in previews
    ) or "<li>No pull request previews are published right now.</li>"
    wip.mkdir(parents=True, exist_ok=True)
    (wip / "index.html").write_text(
        '<!doctype html><html lang="en"><meta charset="utf-8">'
        '<meta name="viewport" content="width=device-width,initial-scale=1">'
        '<meta name="robots" content="noindex">'
        '<title>Sindri WIP previews</title>'
        '<style>body{font:1.1rem system-ui;max-width:50rem;margin:4rem auto;'
        'padding:0 1.5rem;background:#111827;color:#f9fafb}a{color:#93c5fd}'
        'li{margin:.75rem 0}</style>'
        '<h1>Work in progress</h1>'
        '<p>Unmerged pull requests whose current head passed every check. '
        'These are development previews, not stable releases. '
        '<a href="../">Stable site</a>.</p>'
        f'<ul>{items}</ul></html>',
        encoding="utf-8",
    )
    manifest = {
        "schema": 1,
        "published_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "stable": stable,
        "previews": previews,
    }
    (wip / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def download(repo: str, run_id: int, name: str, dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    subprocess.run(["gh", "run", "download", str(run_id), "--repo", repo,
                    "--name", name, "--dir", str(dest)], check=True)


def assemble(repo: str, desired: dict, out: Path, staging: Path,
             fetch: Callable[[str, int, str, Path], None] = download,
             base_path: str = "") -> list[dict]:
    """Build `out` = stable site + every desired preview. Returns what was published."""
    stable = desired["stable"]
    if not stable:
        raise SystemExit("No stable site to assemble from")
    if out.exists():
        shutil.rmtree(out)
    fetch(repo, stable["run_id"], STABLE_ARTIFACT, out)
    safe_extract_check(out)
    for needed in ("index.html", "directory/index.html"):
        if not (out / needed).is_file():
            raise SystemExit(f"Stable site artifact lacks {needed}")
    if (out / "wip").exists():
        raise SystemExit("Stable site must not contain /wip/; previews are added here")
    published = []
    for preview in desired["previews"]:
        pr = int(preview["pr"])
        stage = staging / str(pr)
        if stage.exists():
            shutil.rmtree(stage)
        fetch(repo, preview["run_id"], PREVIEW_ARTIFACT, stage)
        safe_extract_check(stage)
        check_preview(stage, pr, base_path)
        shutil.copytree(stage, out / "wip" / str(pr))
        published.append(preview)
        print(f"Added /wip/{pr}/ from run {preview['run_id']} ({preview['sha'][:12]})")
    write_index(out / "wip", published, stable)
    (out / ".nojekyll").touch()
    return published


# --------------------------------------------------------------------------- #
# Command line


def write_outputs(**values) -> None:
    target = os.environ.get("GITHUB_OUTPUT")
    lines = "".join(f"{key}={value}\n" for key, value in values.items())
    if target:
        with open(target, "a", encoding="utf-8") as handle:
            handle.write(lines)
    else:
        sys.stdout.write(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    p_plan = sub.add_parser("plan")
    p_plan.add_argument("--repo", required=True)
    p_plan.add_argument("--event-run", default="")
    p_plan.add_argument("--pr", default="")
    p_plan.add_argument("--site-url", default="")
    p_plan.add_argument("--plan-out", default="")
    p_asm = sub.add_parser("assemble")
    p_asm.add_argument("--repo", required=True)
    p_asm.add_argument("--plan", required=True)
    p_asm.add_argument("--out", required=True)
    p_asm.add_argument("--staging", default="target/wip-staging")
    p_asm.add_argument("--base-path", default="")
    args = parser.parse_args(argv)

    if args.command == "plan":
        if args.pr and not args.pr.strip().isdecimal():
            print(f"::error::pr_number must be a pull request number, got {args.pr!r}")
            return 2
        live = fetch_live(args.site_url) if args.site_url else None
        result = plan(gh_api, args.repo, live,
                      requested=int(args.pr) if args.pr.strip() else None,
                      event_run=int(args.event_run) if args.event_run else None)
        text = summary(result)
        print(text)
        if os.environ.get("GITHUB_STEP_SUMMARY"):
            with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
                handle.write(text)
        annotate(result)
        if args.plan_out:
            Path(args.plan_out).write_text(json.dumps(result, indent=2), encoding="utf-8")
        requested_ok = not result["requested"] or any(
            v["pr"] == result["requested"] and v["status"] == "ready" for v in result["verdicts"])
        write_outputs(
            deploy=str(result["deploy"]).lower(),
            previews=",".join(str(p["pr"]) for p in result["desired"]["previews"]),
            requested_ok=str(requested_ok).lower(),
        )
        return 0

    result = json.loads(Path(args.plan).read_text(encoding="utf-8"))
    assemble(args.repo, result["desired"], Path(args.out), Path(args.staging),
             base_path=args.base_path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
