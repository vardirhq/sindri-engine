#!/usr/bin/env python3
"""Find a result CI already verified, so a push to main does not repeat it.

A pull request's CI tests the merge of its head into main, and a merge queue
tests the exact commit main is about to become. When that pull request merges
with main unchanged, the commit main ends up on has the *same tree* — the same
bytes in every file, workflows included — as the one already tested, and
running the same checks over the same bytes can only repeat their answer.

Two lookups, one per workflow that re-ran on every merge:

* ``ci``: CI records each fully green run as an artifact named
  ``ci-verified-<tree>``. A push to main whose tree has such a record from a
  run in this repository skips its jobs. A merge that changed a dependency
  manifest never skips, because main's run is what saves the build cache every
  pull request restores from.

* ``pages``: a Sindri Pages run in the merge queue builds the optimised site
  for the exact commit main becomes and uploads it as ``queued-site``. A push
  to main promotes that artifact instead of rebuilding it -- unless it changed
  a dependency manifest, for the same cache reason as ``ci``.

Usage:
  ci_reuse.py ci --repo OWNER/REPO --repo-id ID --tree TREE --changed FILE
  ci_reuse.py pages --repo OWNER/REPO --sha SHA --changed FILE

Each prints its findings and, under GitHub Actions, writes its answer to
``$GITHUB_OUTPUT`` (``verified`` or ``run_id``) and a line to the job summary.
Any failure to reach the API answers "nothing to reuse", so the worst outcome
is the full run that happened before this existed.
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import PurePosixPath
from typing import Callable

CI_WORKFLOW = ".github/workflows/ci.yml"
PAGES_WORKFLOW = ".github/workflows/pages.yml"
VERIFIED_PREFIX = "ci-verified-"
QUEUED_SITE = "queued-site"

# Files whose change alters what the Rust build cache holds. rust-cache keys on
# these, so main has to run in full to save the cache pull requests restore.
CACHE_INPUTS = ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "rust-toolchain")


def gh_api(path: str):
    """Call the REST API through the `gh` CLI (GH_TOKEN from the workflow)."""
    out = subprocess.run(
        ["gh", "api", "-H", "Accept: application/vnd.github+json", path],
        check=True, capture_output=True, text=True,
    )
    return json.loads(out.stdout)


def changes_cache_inputs(changed: list[str]) -> list[str]:
    """The changed paths that would alter the Rust build cache's key."""
    return [path for path in changed if PurePosixPath(path).name in CACHE_INPUTS]


def verified_ci_run(api: Callable, repo: str, repo_id: int, tree: str) -> dict | None:
    """A green CI run in this repository that recorded `tree` as verified.

    The record's name is only a claim: a workflow edited in a fork's pull
    request could upload an artifact of any name. So the record counts only
    when the run that uploaded it belongs to this repository, is CI itself, and
    finished green.
    """
    artifacts = api(f"repos/{repo}/actions/artifacts?name={VERIFIED_PREFIX}{tree}&per_page=20")
    for artifact in artifacts.get("artifacts", []):
        source = artifact.get("workflow_run") or {}
        if artifact.get("expired") or source.get("head_repository_id") != repo_id:
            continue
        run = api(f"repos/{repo}/actions/runs/{source['id']}")
        if (run.get("path") == CI_WORKFLOW and run.get("conclusion") == "success"
                and (run.get("head_repository") or {}).get("full_name") == repo):
            return run
    return None


def queued_site_run(api: Callable, repo: str, sha: str) -> dict | None:
    """A green merge-queue Pages run for `sha` with a live `queued-site`."""
    runs = api(
        f"repos/{repo}/actions/workflows/pages.yml/runs"
        f"?head_sha={sha}&event=merge_group&status=success&per_page=20"
    ).get("workflow_runs", [])
    for run in runs:
        if run.get("head_sha") != sha or run.get("path") != PAGES_WORKFLOW:
            continue
        if (run.get("head_repository") or {}).get("full_name") != repo:
            continue
        artifacts = api(f"repos/{repo}/actions/runs/{run['id']}/artifacts?per_page=100")
        if any(a["name"] == QUEUED_SITE and not a["expired"]
               for a in artifacts.get("artifacts", [])):
            return run
    return None


def report(outputs: dict[str, str], summary: str) -> None:
    """Print the answer and hand it to GitHub Actions when running there."""
    print(summary)
    for name, value in outputs.items():
        print(f"{name}={value}")
    if path := os.environ.get("GITHUB_OUTPUT"):
        with open(path, "a", encoding="utf-8") as out:
            for name, value in outputs.items():
                out.write(f"{name}={value}\n")
    if path := os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(path, "a", encoding="utf-8") as out:
            out.write(summary + "\n")


def main(argv: list[str] | None = None, api: Callable = gh_api) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    ci = sub.add_parser("ci")
    ci.add_argument("--repo", required=True)
    ci.add_argument("--repo-id", required=True, type=int)
    ci.add_argument("--tree", required=True)
    ci.add_argument("--changed", required=True, help="file listing the paths the push changed")
    pages = sub.add_parser("pages")
    pages.add_argument("--repo", required=True)
    pages.add_argument("--sha", required=True)
    pages.add_argument("--changed", required=True, help="file listing the paths the push changed")
    args = parser.parse_args(argv)
    empty = {"verified": "false"} if args.command == "ci" else {"run_id": ""}

    try:
        with open(args.changed, encoding="utf-8") as listing:
            changed = [line.strip() for line in listing if line.strip()]
        if inputs := changes_cache_inputs(changed):
            report(empty, "Running in full: this push changes " + ", ".join(inputs)
                   + ", so main saves a new build cache.")
            return 0
        if args.command == "ci":
            run = verified_ci_run(api, args.repo, args.repo_id, args.tree)
            if run:
                report({"verified": "true"},
                       f"Tree {args.tree[:12]} already passed CI in run "
                       f"[{run['id']}]({run.get('html_url', '')}) ({run.get('event')}); "
                       "every job is skipped.")
            else:
                report({"verified": "false"},
                       f"No green CI run has verified tree {args.tree[:12]}; running in full.")
        else:
            run = queued_site_run(api, args.repo, args.sha)
            if run:
                report({"run_id": str(run["id"])},
                       f"Promoting the site the merge queue built for {args.sha[:12]} in run "
                       f"[{run['id']}]({run.get('html_url', '')}).")
            else:
                report({"run_id": ""},
                       f"No merge-queue build of {args.sha[:12]}; building the site.")
    except (subprocess.CalledProcessError, KeyError, ValueError, OSError) as error:
        report(empty, f"Could not look for a reusable result ({error}); running in full.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
