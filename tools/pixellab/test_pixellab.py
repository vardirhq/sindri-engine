"""Tests for pixellab.py against a local stand-in for the API.

    python3 -m unittest tools/pixellab/test_pixellab.py

The stand-in speaks the shapes the real API's OpenAPI document describes (a 202
with a job id, 423 while a result is being made, 429 once, base64 images, a ZIP),
so what is checked is the client's handling of them: authentication, waiting,
retrying, saving, and refusing a ZIP that would write outside its folder. None of
it touches the real service, and none of it says what the real service returns.
"""

import base64
import contextlib
import http.server
import importlib.util
import io
import json
import os
import pathlib
import tempfile
import threading
import unittest
import zipfile

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("pixellab", HERE / "pixellab.py")
pixellab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pixellab)

# A 1x1 transparent PNG.
PIXEL = base64.b64encode(
    bytes.fromhex(
        "89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000d49444154789c6360000002000001"
        "e221bc330000000049454e44ae426082"
    )
).decode()


def zipped(files):
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        for name, content in files.items():
            archive.writestr(name, content)
    return buffer.getvalue()


class Fake(http.server.BaseHTTPRequestHandler):
    """State is on the class: how many times each path was asked, and what was sent."""

    seen = {}
    sent = []
    zip_files = {"rotations/south.png": b"png"}

    def log_message(self, *args):
        pass

    def reply(self, status, body=b"", kind="application/json"):
        if not isinstance(body, bytes):
            body = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", kind)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def handle_any(self, method):
        path = self.path
        count = Fake.seen[(method, path)] = Fake.seen.get((method, path), 0) + 1
        length = int(self.headers.get("Content-Length") or 0)
        body = json.loads(self.rfile.read(length)) if length else None
        Fake.sent.append((method, path, self.headers.get("Authorization"), body))
        if path != "/openapi.json" and self.headers.get("Authorization") != "Bearer secret":
            return self.reply(401, {"detail": "Invalid API token"})
        if (method, path) == ("GET", "/balance"):
            # Rate limited once, to be retried.
            if count == 1:
                return self.reply(429, {"detail": "slow down"})
            return self.reply(200, {"credits": {"usd": 1.5}, "subscription": {"tier": "free"}})
        if (method, path) == ("POST", "/create-tileset"):
            return self.reply(202, {"background_job_id": "job1", "tileset_id": "ts1", "status": "processing"})
        if (method, path) == ("GET", "/tilesets/ts1"):
            if count < 3:
                return self.reply(423, b"still generating", "text/plain")
            return self.reply(
                200, {"tileset": {"tiles": [{"image": {"type": "base64", "base64": PIXEL}}]}, "metadata": {"seed": 7}}
            )
        if (method, path) == ("POST", "/create-character-with-4-directions"):
            return self.reply(200, {"background_job_id": "job2", "character_id": "ch1", "status": "processing"})
        if (method, path) == ("GET", "/background-jobs/job2"):
            status = "processing" if count == 1 else "completed"
            return self.reply(200, {"id": "job2", "status": status, "created_at": "now", "last_response": None})
        if (method, path) == ("GET", "/background-jobs/bad"):
            return self.reply(200, {"id": "bad", "status": "failed", "created_at": "now", "last_response": {"error": "no credits"}})
        if (method, path) == ("GET", "/characters/ch1/zip"):
            if count == 1:
                return self.reply(423, b"", "text/plain")
            return self.reply(200, zipped(Fake.zip_files), "application/zip")
        if (method, path) == ("POST", "/characters/animations"):
            return self.reply(200, {"background_job_ids": ["job2", "job4"], "status": "processing"})
        if (method, path) == ("GET", "/background-jobs/job4"):
            return self.reply(200, {"id": "job4", "status": "completed", "created_at": "now", "last_response": {"direction": "west"}})
        if (method, path) == ("POST", "/create-1-direction-object"):
            return self.reply(200, {"background_job_id": "job3", "object_id": "ob1", "status": "queued", "n_frames": 16})
        if (method, path) == ("GET", "/background-jobs/job3"):
            return self.reply(200, {"id": "job3", "status": "completed", "created_at": "now", "last_response": {"frame_count": 16}})
        if (method, path) == ("POST", "/objects/ob1/select-frames"):
            made = [f"ob{n + 2}" for n, _ in enumerate(body["indices"])]
            return self.reply(200, {"created_object_ids": made})
        if method == "GET" and path.startswith("/objects/ob") and path.endswith("/spritesheet"):
            return self.reply(200, zipped({"tree.png": b"png", "tree.json": b"{}"}), "application/zip")
        if (method, path) == ("POST", "/animate-character"):
            return self.reply(200, {"background_job_ids": ["job2"], "directions": ["south"], "status": "processing"})
        if (method, path) == ("GET", "/openapi.json"):
            return self.reply(200, OPENAPI)
        return self.reply(404, {"detail": f"no {method} {path}"})

    def do_GET(self):
        self.handle_any("GET")

    def do_POST(self):
        self.handle_any("POST")


OPENAPI = {
    "paths": {
        "/create-tileset": {
            "post": {
                "summary": "Create top-down tileset",
                "requestBody": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/Req"}}}},
                "responses": {"202": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/Resp"}}}}},
            }
        }
    },
    "components": {
        "schemas": {
            "Req": {
                "required": ["lower_description"],
                "properties": {
                    "lower_description": {"type": "string"},
                    "mode": {"type": "string", "enum": ["standard", "pro"], "default": "standard"},
                    "seed": {"anyOf": [{"type": "integer"}, {"type": "null"}]},
                },
            },
            "Resp": {"properties": {"tileset_id": {"type": "string"}}},
        }
    },
}


class ClientTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Fake)
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()
        pixellab.BASE = f"http://127.0.0.1:{cls.server.server_port}"

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()

    def setUp(self):
        Fake.seen.clear()
        Fake.sent.clear()
        Fake.zip_files = {"rotations/south.png": b"png"}
        os.environ["PIXELLAB_API_KEY"] = "secret"
        os.environ["PIXELLAB_POLL_SECONDS"] = "0"
        self.folder = pathlib.Path(tempfile.mkdtemp())

    def run_cli(self, *argv):
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            pixellab.main([str(a) for a in argv])
        return out.getvalue()

    def test_every_request_is_authenticated_and_a_rate_limit_is_retried(self):
        out = self.run_cli("balance")
        self.assertEqual(json.loads(out)["credits"]["usd"], 1.5)
        self.assertEqual(Fake.seen[("GET", "/balance")], 2, "the 429 was retried")
        self.assertTrue(all(auth == "Bearer secret" for _, _, auth, _ in Fake.sent))

    def test_a_missing_key_is_said_not_sent(self):
        del os.environ["PIXELLAB_API_KEY"]
        with self.assertRaises(SystemExit) as raised:
            self.run_cli("balance")
        self.assertIn("PIXELLAB_API_KEY", str(raised.exception))
        self.assertEqual(Fake.sent, [])

    def test_a_refused_key_is_an_error_with_the_reason(self):
        os.environ["PIXELLAB_API_KEY"] = "wrong"
        with self.assertRaises(SystemExit) as raised:
            self.run_cli("balance")
        self.assertIn("401", str(raised.exception))
        self.assertIn("Invalid API token", str(raised.exception))

    def test_a_tileset_waits_out_423_and_saves_its_images(self):
        self.run_cli("tileset", "--lower", "grass", "--upper", "dirt", "--tile-size", "32", "--seed", "7", "--set", "mode=pro", "--out", self.folder)
        started = next(body for method, path, _, body in Fake.sent if path == "/create-tileset")
        self.assertEqual(started["lower_description"], "grass")
        self.assertEqual(started["tile_size"], {"width": 32, "height": 32})
        self.assertEqual(started["mode"], "pro", "--set values are read as JSON or text")
        self.assertEqual(started["seed"], 7)
        self.assertEqual(Fake.seen[("GET", "/tilesets/ts1")], 3, "polled until it was ready")
        saved = sorted(path.name for path in self.folder.iterdir())
        self.assertEqual(len(saved), 2, saved)
        png = next(path for path in self.folder.iterdir() if path.suffix == ".png")
        self.assertTrue(png.read_bytes().startswith(b"\x89PNG"))
        written = json.loads((self.folder / "tileset.json").read_text())
        self.assertNotIn("base64", json.dumps(written), "the image data is left out of the JSON")
        self.assertEqual(written["metadata"]["seed"], 7)

    def test_a_character_waits_for_its_job_then_unpacks_its_zip(self):
        self.run_cli("character", "--description", "a hooded lamplighter", "--size", "48x64", "--out", self.folder)
        started = next(body for method, path, _, body in Fake.sent if path == "/create-character-with-4-directions")
        self.assertEqual(started["image_size"], {"width": 48, "height": 64})
        self.assertEqual(Fake.seen[("GET", "/background-jobs/job2")], 2)
        self.assertEqual((self.folder / "character" / "rotations" / "south.png").read_bytes(), b"png")
        self.assertEqual((self.folder / "character-id.txt").read_text().strip(), "ch1")

    def test_animate_asks_for_every_direction_waits_for_each_job_then_downloads(self):
        self.run_cli(
            "animate", "--character", "ch1", "--name", "walk", "--description", "walking", "--frames", 6,
            "--directions", "south, west", "--seed", 3, "--out", self.folder,
        )
        sent = next(body for method, path, _, body in Fake.sent if path == "/characters/animations")
        self.assertEqual(
            sent,
            {
                "character_id": "ch1", "animation_name": "walk", "action_description": "walking",
                "frame_count": 6, "directions": ["south", "west"], "seed": 3,
            },
        )
        self.assertEqual(Fake.seen[("GET", "/background-jobs/job4")], 1, "the second direction's job is waited for too")
        self.assertTrue((self.folder / "character.zip").is_file())

    def test_animate_without_directions_leaves_it_to_the_service(self):
        self.run_cli("animate", "--character", "ch1", "--out", self.folder)
        sent = next(body for method, path, _, body in Fake.sent if path == "/characters/animations")
        self.assertEqual(sent, {"character_id": "ch1"})

    def test_call_wait_keeps_every_job_not_the_last(self):
        self.run_cli("call", "POST", "/characters/animations", "--json", '{"character_id": "ch1"}', "--wait", "--out", self.folder)
        saved = json.loads((self.folder / "response.json").read_text())
        self.assertEqual([job["id"] for job in saved["jobs"]], ["job2", "job4"])
        self.assertEqual(saved["jobs"][1]["last_response"], {"direction": "west"})

    def test_call_wait_with_one_job_keeps_its_old_shape(self):
        self.run_cli("call", "POST", "/create-1-direction-object", "--json", '{"description": "a"}', "--wait", "--out", self.folder)
        saved = json.loads((self.folder / "response.json").read_text())
        self.assertIn("job", saved)
        self.assertNotIn("jobs", saved)

    def test_an_object_promotes_the_candidates_asked_for_and_exports_each(self):
        self.run_cli(
            "object", "--description", "a dead tree", "--size", 64, "--keep", "0,3", "--out", self.folder
        )
        made = next(body for method, path, _, body in Fake.sent if path == "/create-1-direction-object")
        self.assertEqual(made, {"description": "a dead tree", "view": "top-down", "size": 64})
        chosen = next(body for method, path, _, body in Fake.sent if path == "/objects/ob1/select-frames")
        self.assertEqual(chosen, {"indices": [0, 3]})
        for position in (0, 1):
            self.assertEqual((self.folder / f"object-{position}" / "tree.png").read_bytes(), b"png")

    def test_an_object_keeps_only_the_first_candidate_by_default(self):
        self.run_cli("object", "--description", "a boulder", "--out", self.folder)
        chosen = next(body for method, path, _, body in Fake.sent if path == "/objects/ob1/select-frames")
        self.assertEqual(chosen, {"indices": [0]})
        self.assertFalse((self.folder / "object-1").exists())

    def test_an_object_dry_run_sends_nothing(self):
        out = self.run_cli("object", "--description", "a boulder", "--out", self.folder, "--dry-run")
        self.assertIn("/create-1-direction-object", out)
        self.assertEqual(Fake.sent, [])

    def test_a_failed_job_is_an_error_naming_why(self):
        with self.assertRaises(SystemExit) as raised:
            self.run_cli("job", "bad", "--wait")
        self.assertIn("no credits", str(raised.exception))

    def test_a_zip_that_would_write_outside_its_folder_is_refused(self):
        Fake.zip_files = {"../escape.txt": b"x"}
        with self.assertRaises(SystemExit) as raised:
            self.run_cli("download", "ch1", "--out", self.folder)
        self.assertIn("outside", str(raised.exception))
        self.assertFalse((self.folder.parent / "escape.txt").exists())

    def test_dry_run_prints_the_request_and_sends_nothing(self):
        out = self.run_cli("tileset", "--lower", "grass", "--upper", "dirt", "--out", self.folder, "--dry-run")
        self.assertIn("POST", out)
        self.assertIn("/create-tileset", out)
        self.assertIn('"lower_description": "grass"', out)
        self.assertEqual(Fake.sent, [])

    def test_call_sends_any_endpoint_and_merges_set_into_the_body(self):
        self.run_cli("call", "POST", "/create-tileset", "--json", '{"lower_description": "a"}', "--set", "seed=3", "--out", self.folder)
        body = next(body for method, path, _, body in Fake.sent if path == "/create-tileset")
        self.assertEqual(body, {"lower_description": "a", "seed": 3})

    def test_spec_lists_endpoints_and_describes_one_from_the_openapi_document(self):
        listing = self.run_cli("spec")
        self.assertIn("/create-tileset", listing)
        detail = self.run_cli("spec", "/create-tileset")
        self.assertIn("lower_description*: string", detail)
        self.assertIn("mode: string one of ['standard', 'pro'] (default 'standard')", detail)
        self.assertIn("seed: integer|null", detail)
        self.assertIn("tileset_id: string", detail)
        # Reading the spec needs no key.
        self.assertTrue(all(auth is None for method, path, auth, _ in Fake.sent if path == "/openapi.json"))

    def test_set_refuses_a_pair_without_an_equals_sign(self):
        with self.assertRaises(SystemExit):
            pixellab.parse_pairs(["seed"])


if __name__ == "__main__":
    unittest.main()
