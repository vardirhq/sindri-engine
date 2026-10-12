"""Tests for the site publisher's verification and assembly (no network)."""
from pathlib import Path
import json
import re
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import wip_previews as wp  # noqa: E402

REPO = "vardirhq/sindri-engine"
HEAD = "7e9fe4f420411a1e3b980dda7da23b8662cff630"
MERGE = "a481ea95091ee71cc67d11c78f4a65efb8faa6e6"
OLD = "b9a593ea0000000000000000000000000000000"


def pr(number=507, sha=HEAD, state="open", head_repo=REPO, title="3D labs"):
    return {"number": number, "title": title, "state": state,
            "head": {"sha": sha, "repo": {"full_name": head_repo} if head_repo else None},
            "merge_commit_sha": MERGE}


def run(rid, name, path, sha=HEAD, event="pull_request", status="completed",
        conclusion="success", created="2026-10-12T12:00:00Z", branch="ccr", prs=(507,)):
    return {"id": rid, "name": name, "path": f".github/workflows/{path}", "head_sha": sha,
            "event": event, "status": status, "conclusion": conclusion, "created_at": created,
            "head_branch": branch, "head_repository": {"full_name": REPO},
            "pull_requests": [{"number": n} for n in prs]}


def green_runs(sha=HEAD):
    return [run(1, "CI", "ci.yml", sha), run(2, "Sindri Pages", "pages.yml", sha),
            run(3, "Autofix", "autofix.yml", sha)]


def artifact(aid, name=wp.PREVIEW_ARTIFACT, expired=False, sha=HEAD):
    return {"id": aid, "name": name, "expired": expired, "workflow_run": {"head_sha": sha}}


class FakeGitHub:
    """Routes REST paths to canned data, honouring the filters the code relies on."""

    def __init__(self, prs, runs, artifacts=None, check_runs=None, status=None,
                 stable_runs=None):
        self.prs = {p["number"]: p for p in prs}
        self.runs = runs
        self.artifacts = artifacts or {}
        self.check_runs = check_runs or {}
        self.status = status or {}
        self.stable_runs = stable_runs if stable_runs is not None else [
            run(900, "Sindri Pages", "pages.yml", sha="main1", event="push", branch="main", prs=())]
        self.calls = []

    def __call__(self, path):
        self.calls.append(path)
        route, _, query = path.partition("?")
        params = dict(kv.split("=", 1) for kv in query.split("&") if kv)
        page = int(params.get("page", 1))
        if route == f"repos/{REPO}/pulls":
            return [p for p in self.prs.values() if p["state"] == "open"] if page == 1 else []
        if m := re.fullmatch(rf"repos/{REPO}/pulls/(\d+)", route):
            return self.prs[int(m[1])]
        if route == f"repos/{REPO}/actions/runs":
            hits = [r for r in self.runs if r["head_sha"] == params["head_sha"]
                    and r["event"] == params.get("event", r["event"])]
            return {"workflow_runs": hits if page == 1 else []}
        if m := re.fullmatch(rf"repos/{REPO}/actions/runs/(\d+)", route):
            return next(r for r in self.runs + self.stable_runs if r["id"] == int(m[1]))
        if m := re.fullmatch(rf"repos/{REPO}/actions/runs/(\d+)/artifacts", route):
            return {"artifacts": self.artifacts.get(int(m[1]), []) if page == 1 else []}
        if route == f"repos/{REPO}/actions/workflows/pages.yml/runs":
            return {"workflow_runs": self.stable_runs}
        if m := re.fullmatch(rf"repos/{REPO}/commits/(\w+)/check-runs", route):
            return {"check_runs": self.check_runs.get(m[1], []) if page == 1 else []}
        if m := re.fullmatch(rf"repos/{REPO}/commits/(\w+)/status", route):
            return self.status.get(m[1], {"state": "pending", "total_count": 0})
        raise AssertionError(f"unexpected API call {path}")


def stable_artifacts():
    return {900: [artifact(9000, wp.STABLE_ARTIFACT, sha="main1")]}


class VerifyTests(unittest.TestCase):
    def verdict(self, gh, request=None):
        return wp.verify_pr(gh, REPO, request or gh.prs[507])

    def test_green_head_with_artifact_is_ready(self):
        gh = FakeGitHub([pr()], green_runs(), {2: [artifact(77)]})
        v = self.verdict(gh)
        self.assertEqual((v.status, v.run_id, v.artifact_id, v.sha), ("ready", 2, 77, HEAD))

    def test_runs_are_matched_on_head_sha_not_merge_sha(self):
        # Runs recorded against the synthetic merge commit never count.
        gh = FakeGitHub([pr()], green_runs(sha=MERGE), {2: [artifact(77, sha=MERGE)]})
        v = self.verdict(gh)
        self.assertEqual(v.status, "rejected")
        self.assertIn("no pull_request workflow ran for head 7e9fe4f42041", v.reason)
        self.assertIn("conflicted with its base", v.reason)
        self.assertTrue(any(f"head_sha={HEAD}" in c for c in gh.calls))
        self.assertFalse(any(MERGE in c for c in gh.calls))

    def test_head_without_any_runs_is_explained(self):
        # PR #507's actual state: green CI on an older commit, none on the head.
        gh = FakeGitHub([pr()], green_runs(sha=OLD), {2: [artifact(77, sha=OLD)]})
        v = self.verdict(gh)
        self.assertEqual(v.status, "rejected")
        self.assertIn("Update branch", v.reason)

    def test_stale_artifact_from_older_commit_is_rejected(self):
        gh = FakeGitHub([pr()], green_runs(), {2: [artifact(77, sha=OLD)]})
        v = self.verdict(gh)
        self.assertNotEqual(v.status, "ready")

    def test_missing_artifact(self):
        gh = FakeGitHub([pr()], green_runs(), {2: [artifact(5, "directory-captures")]})
        v = self.verdict(gh)
        self.assertEqual(v.status, "ignored")
        self.assertIn("uploaded no 3d-labs-preview", v.reason)

    def test_expired_artifact(self):
        gh = FakeGitHub([pr()], green_runs(), {2: [artifact(77, expired=True)]})
        v = self.verdict(gh)
        self.assertEqual(v.status, "rejected")
        self.assertIn("expired", v.reason)

    def test_failed_check_rejects(self):
        runs = green_runs() + [run(4, "Decay", "decay.yml", conclusion="failure")]
        v = self.verdict(FakeGitHub([pr()], runs, {2: [artifact(77)]}))
        self.assertEqual(v.status, "rejected")
        self.assertIn("Decay (failure)", v.reason)

    def test_cancelled_check_rejects(self):
        runs = green_runs()
        runs[0]["conclusion"] = "cancelled"
        v = self.verdict(FakeGitHub([pr()], runs, {2: [artifact(77)]}))
        self.assertEqual(v.status, "rejected")

    def test_rerun_supersedes_failed_attempt(self):
        runs = green_runs() + [run(0, "CI", "ci.yml", conclusion="failure",
                                   created="2026-10-12T11:00:00Z")]
        v = self.verdict(FakeGitHub([pr()], runs, {2: [artifact(77)]}))
        self.assertEqual(v.status, "ready")

    def test_path_filtered_workflows_are_optional(self):
        # Decay, Shapes Lab etc. only run when their paths change.
        v = self.verdict(FakeGitHub([pr()], green_runs(), {2: [artifact(77)]}))
        self.assertEqual(v.status, "ready")

    def test_missing_required_workflow(self):
        runs = [r for r in green_runs() if r["name"] != "CI"]
        v = self.verdict(FakeGitHub([pr()], runs, {2: [artifact(77)]}))
        self.assertEqual(v.status, "rejected")
        self.assertIn("CI", v.reason)

    def test_incomplete_run_is_pending(self):
        runs = green_runs()
        runs[0].update(status="in_progress", conclusion=None)
        v = self.verdict(FakeGitHub([pr()], runs, {2: [artifact(77)]}))
        self.assertEqual(v.status, "pending")
        self.assertIn("CI", v.reason)

    def test_failed_external_check_run_rejects(self):
        checks = {HEAD: [{"name": "lint-bot", "status": "completed", "conclusion": "failure",
                          "app": {"slug": "lint"}},
                         {"name": "Build", "status": "completed", "conclusion": "failure",
                          "app": {"slug": "github-actions"}}]}
        v = self.verdict(FakeGitHub([pr()], green_runs(), {2: [artifact(77)]}, checks))
        self.assertEqual(v.status, "rejected")
        self.assertIn("lint-bot", v.reason)
        self.assertNotIn("Build", v.reason)

    def test_failed_commit_status_rejects(self):
        gh = FakeGitHub([pr()], green_runs(), {2: [artifact(77)]},
                        status={HEAD: {"state": "failure", "total_count": 1}})
        self.assertEqual(self.verdict(gh).status, "rejected")

    def test_fork_and_closed_prs_are_ignored(self):
        gh = FakeGitHub([pr()], green_runs(), {2: [artifact(77)]})
        self.assertEqual(self.verdict(gh, pr(head_repo="evil/fork")).status, "ignored")
        self.assertEqual(self.verdict(gh, pr(head_repo=None)).status, "ignored")
        self.assertEqual(self.verdict(gh, pr(state="closed")).status, "ignored")

    def test_runs_from_fork_with_same_sha_do_not_count(self):
        runs = green_runs()
        for r in runs:
            r["head_repository"] = {"full_name": "evil/fork"}
        self.assertEqual(self.verdict(FakeGitHub([pr()], runs, {2: [artifact(77)]})).status,
                         "rejected")


class PlanTests(unittest.TestCase):
    def gh(self, **kw):
        arts = {2: [artifact(77)], **stable_artifacts()}
        return FakeGitHub([pr()], green_runs(), arts, **kw)

    def test_manual_dispatch_publishes_requested_pr(self):
        result = wp.plan(self.gh(), REPO, live=None, requested=507)
        self.assertTrue(result["deploy"])
        self.assertEqual([p["pr"] for p in result["desired"]["previews"]], [507])
        self.assertEqual(result["desired"]["stable"]["run_id"], 900)
        self.assertEqual(result["notes"], [])

    def test_manual_dispatch_explains_rejection(self):
        gh = FakeGitHub([pr()], green_runs(sha=OLD), stable_artifacts())
        result = wp.plan(gh, REPO, live=None, requested=507)
        self.assertEqual(result["desired"]["previews"], [])
        self.assertIn("Requested PR #507 is not publishable", result["notes"][0])
        text = wp.summary(result)
        self.assertIn("no pull_request workflow ran", text)

    def test_manual_dispatch_of_closed_pr(self):
        gh = FakeGitHub([pr(state="closed")], [], stable_artifacts())
        result = wp.plan(gh, REPO, live=None, requested=507)
        self.assertIn("is closed", result["notes"][0])

    def test_workflow_run_for_outdated_commit_is_noted(self):
        runs = green_runs() + [run(50, "CI", "ci.yml", sha=OLD)]
        gh = FakeGitHub([pr()], runs, {2: [artifact(77)], **stable_artifacts()})
        result = wp.plan(gh, REPO, live=None, event_run=50)
        self.assertTrue(any("no longer the head" in n for n in result["notes"]))
        # The current head is still published on its own merits.
        self.assertEqual(result["desired"]["previews"][0]["sha"], HEAD)

    def test_main_deploy_keeps_active_previews(self):
        live = {"stable": {"run_id": 800}, "previews": [
            {"pr": 507, "sha": HEAD, "artifact_id": 77}]}
        result = wp.plan(self.gh(), REPO, live=live,
                         event_run=900)
        self.assertTrue(result["deploy"])
        self.assertEqual([p["pr"] for p in result["desired"]["previews"]], [507])

    def test_no_redeploy_when_live_matches(self):
        live = {"stable": {"run_id": 900}, "previews": [
            {"pr": 507, "sha": HEAD, "artifact_id": 77}]}
        result = wp.plan(self.gh(), REPO, live=live)
        self.assertFalse(result["deploy"])
        self.assertIn("already serves", result["why"])

    def test_merged_pr_preview_is_removed(self):
        live = {"stable": {"run_id": 900}, "previews": [
            {"pr": 507, "sha": HEAD, "artifact_id": 77}]}
        gh = FakeGitHub([pr(state="closed")], green_runs(), {2: [artifact(77)], **stable_artifacts()})
        result = wp.plan(gh, REPO, live=live)
        self.assertTrue(result["deploy"])
        self.assertEqual(result["desired"]["previews"], [])

    def test_pending_new_head_does_not_tear_down_live_preview(self):
        live = {"stable": {"run_id": 900}, "previews": [
            {"pr": 507, "sha": OLD, "artifact_id": 70}]}
        runs = green_runs()
        runs[0].update(status="in_progress", conclusion=None)
        gh = FakeGitHub([pr()], runs, {2: [artifact(77)], **stable_artifacts()})
        result = wp.plan(gh, REPO, live=live)
        self.assertFalse(result["deploy"])
        self.assertIn("still being checked", result["why"])

    def test_failed_new_head_removes_live_preview(self):
        live = {"stable": {"run_id": 900}, "previews": [
            {"pr": 507, "sha": OLD, "artifact_id": 70}]}
        runs = green_runs()
        runs[0]["conclusion"] = "failure"
        gh = FakeGitHub([pr()], runs, {2: [artifact(77)], **stable_artifacts()})
        self.assertTrue(wp.plan(gh, REPO, live=live)["deploy"])

    def test_never_deploys_without_stable_site(self):
        gh = FakeGitHub([pr()], green_runs(), {2: [artifact(77)]}, stable_runs=[])
        result = wp.plan(gh, REPO, live=None, requested=507)
        self.assertFalse(result["deploy"])
        self.assertIn("stable routes", result["why"])

    def test_stable_ignores_pull_request_runs_and_expired_artifacts(self):
        stable = [run(901, "Sindri Pages", "pages.yml", sha="pr", event="pull_request", prs=()),
                  run(902, "Sindri Pages", "pages.yml", sha="m2", event="push", branch="main", prs=()),
                  run(900, "Sindri Pages", "pages.yml", sha="main1", event="push", branch="main", prs=())]
        arts = {901: [artifact(1, wp.STABLE_ARTIFACT)],
                902: [artifact(2, wp.STABLE_ARTIFACT, expired=True)],
                900: [artifact(3, wp.STABLE_ARTIFACT)]}
        gh = FakeGitHub([], [], arts, stable_runs=stable)
        self.assertEqual(wp.latest_stable(gh, REPO)["run_id"], 900)


class AssembleTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())

    def fake_fetch(self, broken=None):
        def fetch(repo, run_id, name, dest):
            dest.mkdir(parents=True, exist_ok=True)
            if name == wp.STABLE_ARTIFACT:
                (dest / "index.html").write_text("stable")
                (dest / "directory").mkdir()
                (dest / "directory/index.html").write_text("dir")
                (dest / "examples/physics/pkg").mkdir(parents=True)
                (dest / "examples/physics/index.html").write_text("physics")
                return
            (dest / "index.html").write_text("labs")
            for route in wp.PREVIEW_ROUTES:
                if route == broken:
                    continue
                (dest / route / "pkg").mkdir(parents=True)
                (dest / route / "assets").mkdir()
                (dest / route / "index.html").write_text(
                    f'<base href="/wip/507/{route}/"><script type=module>'
                    'import("./pkg/sindri_player.js")</script>')
                (dest / route / "assets/sindri.manifest").write_text("{}")
                (dest / route / "pkg/sindri_player.js").write_text("js")
                (dest / route / "pkg/sindri_player_bg.wasm").write_bytes(b"\0asm")
        return fetch

    def desired(self):
        return {"stable": {"run_id": 900, "sha": "main1", "artifact_id": 9000},
                "previews": [{"pr": 507, "sha": HEAD, "run_id": 2, "artifact_id": 77,
                              "title": "3D <labs>"}]}

    def test_assembles_stable_and_nested_preview(self):
        out = self.tmp / "pages"
        wp.assemble(REPO, self.desired(), out, self.tmp / "stage", fetch=self.fake_fetch())
        self.assertEqual((out / "examples/physics/index.html").read_text(), "physics")
        for route in wp.PREVIEW_ROUTES:
            self.assertTrue((out / f"wip/507/{route}/pkg/sindri_player_bg.wasm").is_file())
        index = (out / "wip/index.html").read_text()
        self.assertIn('href="./507/"', index)
        self.assertIn("3D &lt;labs&gt;", index)
        manifest = json.loads((out / "wip/manifest.json").read_text())
        self.assertEqual(manifest["stable"]["run_id"], 900)
        self.assertEqual(manifest["previews"][0]["sha"], HEAD)
        self.assertTrue((out / ".nojekyll").exists())

    def test_incomplete_preview_is_refused(self):
        with self.assertRaises(SystemExit) as caught:
            wp.assemble(REPO, self.desired(), self.tmp / "pages", self.tmp / "stage",
                        fetch=self.fake_fetch(broken="orbit"))
        self.assertIn("orbit/index.html", str(caught.exception))

    def test_wrong_base_path_is_refused(self):
        stage = self.tmp / "s"
        self.fake_fetch()(REPO, 2, wp.PREVIEW_ARTIFACT, stage)
        with self.assertRaises(SystemExit):
            wp.check_preview(stage, 508)

    def test_symlink_in_artifact_is_refused(self):
        root = self.tmp / "a"
        root.mkdir()
        (root / "leak").symlink_to("/etc/passwd")
        with self.assertRaises(SystemExit):
            wp.safe_extract_check(root)

    def test_empty_preview_list_still_writes_manifest(self):
        desired = self.desired()
        desired["previews"] = []
        out = self.tmp / "pages"
        wp.assemble(REPO, desired, out, self.tmp / "stage", fetch=self.fake_fetch())
        self.assertIn("No pull request previews", (out / "wip/index.html").read_text())
        self.assertEqual(json.loads((out / "wip/manifest.json").read_text())["previews"], [])


if __name__ == "__main__":
    unittest.main()
