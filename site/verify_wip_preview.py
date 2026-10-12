#!/usr/bin/env python3
"""Authorize an untrusted PR preview without executing code from its branch.

Print GitHub Actions outputs. Fail closed if any required check is unfinished.
"""
import json
import os
import subprocess
import sys

repo, event_run, event_sha = sys.argv[1:]
def api(path):
    return json.loads(subprocess.check_output(
        ["gh", "api", "--paginate", path], text=True))

def output(**values):
    for key, value in values.items():
        print(f"{key}={value}")

run = api(f"repos/{repo}/actions/runs/{event_run}")
if run["event"] != "pull_request" or run["conclusion"] not in ("success", "skipped"):
    output(ready="false")
    sys.exit(0)
prs = run.get("pull_requests") or []
if len(prs) != 1:
    output(ready="false")
    sys.exit(0)
pr = api(f"repos/{repo}/pulls/{prs[0]['number']}")
sha = pr["head"]["sha"]
if pr["state"] != "open" or pr["head"]["repo"]["full_name"] != repo or sha != event_sha:
    output(ready="false")
    sys.exit(0)
checks = api(f"repos/{repo}/commits/{sha}/check-runs?per_page=100")
check_runs = checks["check_runs"] if isinstance(checks, dict) else []
# Required workflow names are the existing CI suite; none can be pending or failing.
required = {"Sindri Pages", "CI", "Dependencies", "Shapes Lab", "Decay", "Decay VS Code", "Autofix"}
runs = api(f"repos/{repo}/actions/runs?head_sha={sha}&per_page=100")
matching = [r for r in runs["workflow_runs"] if r["event"] == "pull_request" and r["name"] in required]
latest = {}
for r in sorted(matching, key=lambda item: item["created_at"]):
    latest[r["name"]] = r
if set(latest) != required or any(r["status"] != "completed" or r["conclusion"] not in ("success", "skipped") for r in latest.values()):
    output(ready="false")
    sys.exit(0)
pages_run = latest["Sindri Pages"]["id"]
artifacts = api(f"repos/{repo}/actions/runs/{pages_run}/artifacts")
if not any(a["name"] == "3d-labs-preview" and not a["expired"] for a in artifacts["artifacts"]):
    output(ready="false")
    sys.exit(0)
output(ready="true", pr=pr["number"], sha=sha, preview_run=pages_run)
