#!/usr/bin/env python3
"""Authorize publication only for the current PR head with a green CI suite.

Usage: verify_wip_preview.py OWNER/REPO [EVENT_RUN] [EVENT_HEAD] [PR_NUMBER]
Print GitHub Actions outputs. Fail closed when checks or artifact are missing.
"""
import json
import subprocess
import sys

repo = sys.argv[1]
event_run = sys.argv[2] if len(sys.argv) > 2 else ""
event_sha = sys.argv[3] if len(sys.argv) > 3 else ""
pr_number = sys.argv[4] if len(sys.argv) > 4 else ""

def api(path):
    return json.loads(subprocess.check_output(["gh", "api", path], text=True))

def output(**values):
    for key, value in values.items():
        print(f"{key}={value}")

if event_run:
    run = api(f"repos/{repo}/actions/runs/{event_run}")
    if run["event"] != "pull_request" or run["conclusion"] not in ("success", "skipped"):
        output(ready="false")
        sys.exit(0)
    if not pr_number:
        prs = run.get("pull_requests") or []
        if len(prs) == 1:
            pr_number = str(prs[0]["number"])
        else:
            # GitHub sometimes omits pull_requests for workflow_run payloads.
            candidates = api(f"repos/{repo}/commits/{event_sha}/pulls")
            matches = [p for p in candidates if p["state"] == "open" and p["head"]["sha"] == event_sha]
            if len(matches) == 1:
                pr_number = str(matches[0]["number"])
if not pr_number or not pr_number.isdecimal():
    output(ready="false")
    sys.exit(0)

pr = api(f"repos/{repo}/pulls/{pr_number}")
sha = pr["head"]["sha"]
if pr["state"] != "open" or pr["head"]["repo"]["full_name"] != repo or (event_sha and sha != event_sha):
    output(ready="false")
    sys.exit(0)

required = {"Sindri Pages", "CI", "Dependencies", "Shapes Lab", "Decay", "Decay VS Code", "Autofix"}
runs = api(f"repos/{repo}/actions/runs?head_sha={sha}&per_page=100")
matching = [r for r in runs["workflow_runs"] if r["event"] == "pull_request" and r["name"] in required and r["head_sha"] == sha]
latest = {}
for item in sorted(matching, key=lambda x: (x["created_at"], x["id"])):
    latest[item["name"]] = item
if set(latest) != required or any(
    r["status"] != "completed" or r["conclusion"] not in ("success", "skipped")
    for r in latest.values()
):
    output(ready="false")
    sys.exit(0)
pages_run = latest["Sindri Pages"]["id"]
artifacts = api(f"repos/{repo}/actions/runs/{pages_run}/artifacts")
if not any(a["name"] == "3d-labs-preview" and not a["expired"] for a in artifacts["artifacts"]):
    output(ready="false")
    sys.exit(0)
output(ready="true", pr=pr["number"], sha=sha, preview_run=pages_run)
