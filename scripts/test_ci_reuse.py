#!/usr/bin/env python3
"""Tests for finding a result CI already verified (no network)."""
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts import ci_reuse

REPO = "vardirhq/sindri-engine"
REPO_ID = 42
TREE = "4b825dc642cb6eb9a060e54bf8d69288fbee4904"
SHA = "5a0806a7852f688150b828f67e01e93d560b85c6"


def ci_run(rid=7, conclusion="success", path=ci_reuse.CI_WORKFLOW, repo=REPO):
    return {"id": rid, "path": path, "conclusion": conclusion, "event": "pull_request",
            "html_url": f"https://example/{rid}", "head_repository": {"full_name": repo}}


def record(rid=7, repo_id=REPO_ID, expired=False):
    return {"name": ci_reuse.VERIFIED_PREFIX + TREE, "expired": expired,
            "workflow_run": {"id": rid, "head_repository_id": repo_id}}


class FakeApi:
    def __init__(self, routes):
        self.routes = routes
        self.calls = []

    def __call__(self, path):
        self.calls.append(path)
        route = path.partition("?")[0]
        if route not in self.routes:
            raise KeyError(route)
        return self.routes[route]


def artifacts_route(*records):
    return {f"repos/{REPO}/actions/artifacts": {"artifacts": list(records)}}


class CacheInputTests(unittest.TestCase):
    def test_a_manifest_anywhere_is_a_cache_input(self):
        self.assertEqual(
            ci_reuse.changes_cache_inputs(["crates/sindri-core/Cargo.toml", "docs/a.md"]),
            ["crates/sindri-core/Cargo.toml"],
        )

    def test_sources_and_assets_are_not(self):
        self.assertEqual(
            ci_reuse.changes_cache_inputs(["games/low-tide/assets/low-tide.scene", "src/lib.rs"]),
            [],
        )


class VerifiedCiRunTests(unittest.TestCase):
    def test_a_green_run_here_verifies_its_tree(self):
        api = FakeApi({**artifacts_route(record()), f"repos/{REPO}/actions/runs/7": ci_run()})
        self.assertEqual(ci_reuse.verified_ci_run(api, REPO, REPO_ID, TREE)["id"], 7)

    def test_a_record_from_a_fork_is_ignored(self):
        api = FakeApi({**artifacts_route(record(repo_id=99)),
                       f"repos/{REPO}/actions/runs/7": ci_run()})
        self.assertIsNone(ci_reuse.verified_ci_run(api, REPO, REPO_ID, TREE))
        self.assertNotIn(f"repos/{REPO}/actions/runs/7", api.calls)

    def test_a_record_from_another_workflow_is_ignored(self):
        api = FakeApi({**artifacts_route(record()),
                       f"repos/{REPO}/actions/runs/7": ci_run(path=".github/workflows/x.yml")})
        self.assertIsNone(ci_reuse.verified_ci_run(api, REPO, REPO_ID, TREE))

    def test_a_record_from_a_run_that_failed_after_is_ignored(self):
        api = FakeApi({**artifacts_route(record()),
                       f"repos/{REPO}/actions/runs/7": ci_run(conclusion="failure")})
        self.assertIsNone(ci_reuse.verified_ci_run(api, REPO, REPO_ID, TREE))

    def test_an_expired_record_is_ignored(self):
        api = FakeApi(artifacts_route(record(expired=True)))
        self.assertIsNone(ci_reuse.verified_ci_run(api, REPO, REPO_ID, TREE))


class QueuedSiteTests(unittest.TestCase):
    def routes(self, run=None, artifacts=None):
        run = run or {"id": 11, "head_sha": SHA, "path": ci_reuse.PAGES_WORKFLOW,
                      "head_repository": {"full_name": REPO}}
        return {
            f"repos/{REPO}/actions/workflows/pages.yml/runs": {"workflow_runs": [run]},
            f"repos/{REPO}/actions/runs/{run['id']}/artifacts": {"artifacts": artifacts or [
                {"name": ci_reuse.QUEUED_SITE, "expired": False}]},
        }

    def test_the_queue_build_of_this_commit_is_promoted(self):
        api = FakeApi(self.routes())
        self.assertEqual(ci_reuse.queued_site_run(api, REPO, SHA)["id"], 11)

    def test_a_build_of_another_commit_is_not(self):
        run = {"id": 11, "head_sha": "0" * 40, "path": ci_reuse.PAGES_WORKFLOW,
               "head_repository": {"full_name": REPO}}
        self.assertIsNone(ci_reuse.queued_site_run(FakeApi(self.routes(run)), REPO, SHA))

    def test_an_expired_site_is_not(self):
        api = FakeApi(self.routes(artifacts=[{"name": ci_reuse.QUEUED_SITE, "expired": True}]))
        self.assertIsNone(ci_reuse.queued_site_run(api, REPO, SHA))


class MainTests(unittest.TestCase):
    def run_ci(self, changed, api):
        with tempfile.TemporaryDirectory() as directory:
            listing = Path(directory) / "changed"
            listing.write_text("\n".join(changed))
            output = Path(directory) / "output"
            os.environ["GITHUB_OUTPUT"] = str(output)
            try:
                ci_reuse.main(["ci", "--repo", REPO, "--repo-id", str(REPO_ID),
                               "--tree", TREE, "--changed", str(listing)], api=api)
            finally:
                del os.environ["GITHUB_OUTPUT"]
            return output.read_text()

    def test_a_changed_manifest_runs_in_full_without_asking(self):
        api = FakeApi({})
        self.assertIn("verified=false", self.run_ci(["Cargo.toml"], api))
        self.assertEqual(api.calls, [])

    def test_a_verified_tree_skips(self):
        api = FakeApi({**artifacts_route(record()), f"repos/{REPO}/actions/runs/7": ci_run()})
        self.assertIn("verified=true", self.run_ci(["docs/a.md"], api))

    def test_a_changed_manifest_builds_the_site_too(self):
        with tempfile.TemporaryDirectory() as directory:
            listing = Path(directory) / "changed"
            listing.write_text("rust-toolchain.toml")
            output = Path(directory) / "output"
            os.environ["GITHUB_OUTPUT"] = str(output)
            api = FakeApi({})
            try:
                ci_reuse.main(["pages", "--repo", REPO, "--sha", SHA, "--changed", str(listing)],
                              api=api)
            finally:
                del os.environ["GITHUB_OUTPUT"]
            self.assertIn("run_id=\n", output.read_text())
            self.assertEqual(api.calls, [])

    def test_an_unreachable_api_runs_in_full(self):
        def broken(path):
            raise subprocess.CalledProcessError(1, "gh")
        self.assertIn("verified=false", self.run_ci(["docs/a.md"], broken))


if __name__ == "__main__":
    unittest.main()
