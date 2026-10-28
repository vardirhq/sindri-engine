#!/usr/bin/env python3
"""A command-line client for the Pixel Lab API (https://api.pixellab.ai/v2).

    export PIXELLAB_API_KEY=...        # from your Pixel Lab account page
    python3 tools/pixellab/pixellab.py balance
    python3 tools/pixellab/pixellab.py spec                  # every endpoint
    python3 tools/pixellab/pixellab.py spec /create-tileset  # one endpoint's fields
    python3 tools/pixellab/pixellab.py tileset --lower "grass" --upper "dirt path" --out art/grass
    python3 tools/pixellab/pixellab.py character --description "a hooded lamplighter" --out art/wick
    python3 tools/pixellab/pixellab.py animate --character <id> --name walk \\
        --description "walking carrying a lantern" --out art/wick
    python3 tools/pixellab/pixellab.py call POST /create-image-pixen --json '{"description": "..."}'

Standard library only, so it runs anywhere the art scripts do. Every command
takes `--dry-run`, which prints the request instead of sending it. Anything an
endpoint accepts that a command has no flag for goes in with `--set key=value`
(the value is read as JSON when it parses, else as text), and `spec <path>` lists
what an endpoint accepts straight from the API's own OpenAPI document.

Generation is asynchronous: a request answers with a job id, which is polled until
it finishes, and a result still being made answers 423, which is waited out. Any
image a response carries inline (`{"type": "base64", "base64": ...}`) is saved as
a PNG beside a `response.json` with the image data left out. Rate limits (429) and
transient server errors are retried with backoff. Nothing here is specific to
Sindri; turning what comes back into `.sheet` files is the game's own art script.
"""

import argparse
import base64
import io
import json
import os
import pathlib
import sys
import time
import urllib.error
import urllib.request
import zipfile

BASE = os.environ.get("PIXELLAB_API_URL", "https://api.pixellab.ai/v2").rstrip("/")


class ApiError(Exception):
    """The API refused a request, or a job it was asked to run failed."""

    def __init__(self, status, detail):
        super().__init__(f"{status}: {detail}")
        self.status = status
        self.detail = detail


class Pending(Exception):
    """The API answered 423: what was asked for is still being made."""


def token():
    found = os.environ.get("PIXELLAB_API_KEY", "").strip()
    if not found:
        raise SystemExit("set PIXELLAB_API_KEY to the token on your Pixel Lab account page")
    return found


def poll_seconds():
    return float(os.environ.get("PIXELLAB_POLL_SECONDS", "5"))


def request(method, path, body=None, *, auth=True, retries=4):
    """One API call. Returns parsed JSON, or the bytes of anything that is not JSON.

    Raises Pending on 423 and ApiError on any other refusal. Rate limits and
    transient server errors are retried, waiting longer each time.
    """
    data = None if body is None else json.dumps(body).encode()
    headers = {"Accept": "application/json, application/zip"}
    if auth:
        headers["Authorization"] = "Bearer " + token()
    if data is not None:
        headers["Content-Type"] = "application/json"
    for attempt in range(retries + 1):
        req = urllib.request.Request(BASE + path, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req, timeout=180) as response:
                payload = response.read()
                if "json" in response.headers.get("Content-Type", ""):
                    return json.loads(payload) if payload else None
                return payload
        except urllib.error.HTTPError as error:
            detail = error.read().decode("utf-8", "replace")[:2000]
            if error.code == 423:
                raise Pending(detail) from None
            if error.code in (429, 502, 503, 504) and attempt < retries:
                time.sleep(min(60.0, 2.0**attempt) * (poll_seconds() / 5 if poll_seconds() < 5 else 1))
                continue
            raise ApiError(error.code, detail) from None
        except urllib.error.URLError as error:
            if attempt < retries:
                time.sleep(min(30.0, 2.0**attempt) * (poll_seconds() / 5 if poll_seconds() < 5 else 1))
                continue
            raise ApiError(0, f"cannot reach {BASE}: {error.reason}") from None


def wait_for(fetch, label, timeout=1800.0):
    """Calls `fetch` until it stops raising Pending, and returns what it returned."""
    started = time.monotonic()
    while True:
        try:
            return fetch()
        except Pending:
            if time.monotonic() - started > timeout:
                raise ApiError(0, f"{label} is still being made after {timeout:.0f} s") from None
            print(f"  waiting for {label} ...", file=sys.stderr)
            time.sleep(poll_seconds())


def wait_for_job(job_id):
    """Polls a background job until it completes, and returns its last response."""

    def fetch():
        job = request("GET", f"/background-jobs/{job_id}")
        status = (job or {}).get("status", "")
        if status == "failed":
            raise ApiError(0, f"job {job_id} failed: {json.dumps((job or {}).get('last_response'))[:1500]}")
        if status != "completed":
            raise Pending(status)
        return job

    return wait_for(fetch, f"job {job_id}")


def dump(value):
    print(json.dumps(value, indent=2))


def parse_pairs(pairs):
    """`--set key=value` pairs as a dict, each value JSON when it parses and text when it does not."""
    out = {}
    for pair in pairs or []:
        key, sep, value = pair.partition("=")
        if not sep or not key:
            raise SystemExit(f"--set wants key=value, not {pair!r}")
        try:
            out[key] = json.loads(value)
        except json.JSONDecodeError:
            out[key] = value
    return out


def size(text):
    """`16` or `48x64` as {"width": .., "height": ..}."""
    width, _, height = text.partition("x")
    return {"width": int(width), "height": int(height or width)}


def save_images(value, out, prefix="image"):
    """Writes every inline base64 image under `value` to `out`, and returns `value` with them replaced by file names."""
    out = pathlib.Path(out)
    out.mkdir(parents=True, exist_ok=True)
    counter = [0]

    def walk(node, trail):
        if isinstance(node, dict):
            if isinstance(node.get("base64"), str):
                counter[0] += 1
                name = f"{prefix}-{'-'.join(trail) or counter[0]}.{node.get('format', 'png')}"
                (out / name).write_bytes(base64.b64decode(node["base64"]))
                return {"saved": name}
            return {key: walk(item, trail + [str(key)]) for key, item in node.items()}
        if isinstance(node, list):
            return [walk(item, trail + [str(index)]) for index, item in enumerate(node)]
        return node

    return walk(value, [])


def save_result(value, out, name):
    """Saves a response's images and a `<name>.json` of the rest, and returns the files written."""
    out = pathlib.Path(out)
    stripped = save_images(value, out, name)
    out.mkdir(parents=True, exist_ok=True)
    (out / f"{name}.json").write_text(json.dumps(stripped, indent=2) + "\n")
    return sorted(path.name for path in out.iterdir() if path.name.startswith(name))


def save_zip(payload, out, name):
    """Saves a ZIP the API returned and unpacks it beside itself."""
    out = pathlib.Path(out)
    out.mkdir(parents=True, exist_ok=True)
    (out / f"{name}.zip").write_bytes(payload)
    try:
        with zipfile.ZipFile(io.BytesIO(payload)) as archive:
            for member in archive.namelist():
                target = (out / name / member).resolve()
                if not str(target).startswith(str((out / name).resolve())):
                    raise ApiError(0, f"{member} in the ZIP would be written outside {out / name}")
            archive.extractall(out / name)
    except zipfile.BadZipFile:
        raise ApiError(0, "the API did not return a ZIP: " + payload[:200].decode("utf-8", "replace")) from None
    return out / name


def send(args, method, path, body):
    """Sends, or with --dry-run shows, a request. Returns the response, or None when shown."""
    if args.dry_run:
        print(f"{method} {BASE}{path}")
        if body is not None:
            dump(body)
        return None
    return request(method, path, body)


# --- commands ---------------------------------------------------------------


def cmd_balance(args):
    dump(send(args, "GET", "/balance", None))


def schema_of(document, node):
    while isinstance(node, dict) and "$ref" in node:
        node = document["components"]["schemas"][node["$ref"].split("/")[-1]]
    return node or {}


def describe_fields(document, schema, indent="  "):
    schema = schema_of(document, schema)
    required = set(schema.get("required", []))
    for name, raw in schema.get("properties", {}).items():
        field = schema_of(document, raw)
        kind = field.get("type") or "|".join(
            schema_of(document, option).get("type") or schema_of(document, option).get("title", "?")
            for option in raw.get("anyOf", field.get("anyOf", []))
        )
        line = f"{indent}{name}{'*' if name in required else ''}: {kind}"
        if field.get("enum"):
            line += f" one of {field['enum'][:10]}"
        default = field.get("default", raw.get("default"))
        if default is not None and not isinstance(default, (dict, list)):
            line += f" (default {default!r})"
        if isinstance(default, dict):
            line += f" (default {json.dumps(default)})"
        print(line)


def cmd_spec(args):
    document = request("GET", "/openapi.json", auth=False)
    paths = document["paths"]
    if not args.path:
        for path, operations in sorted(paths.items()):
            for method, operation in operations.items():
                print(f"{method.upper():6} {path:52} {operation.get('summary', '')}")
        return
    matches = [path for path in paths if path == args.path or (args.filter and args.path in path)]
    if not matches:
        raise SystemExit(f"no endpoint {args.path!r}; run `spec` to list them")
    for path in matches:
        for method, operation in paths[path].items():
            print(f"\n{method.upper()} {path}: {operation.get('summary', '')}")
            body = operation.get("requestBody", {}).get("content", {}).get("application/json")
            if body:
                print(" request (* = required):")
                describe_fields(document, body["schema"])
            for status, response in operation.get("responses", {}).items():
                content = response.get("content", {}).get("application/json")
                if status.startswith("2") and content and "$ref" in content.get("schema", {}):
                    print(f" response {status}:")
                    describe_fields(document, content["schema"])


def cmd_call(args):
    body = None
    if args.json:
        body = json.loads(args.json)
    elif args.json_file:
        body = json.loads(pathlib.Path(args.json_file).read_text())
    if args.set:
        body = {**(body or {}), **parse_pairs(args.set)}
    response = send(args, args.method.upper(), args.path, body)
    if response is None:
        return
    if isinstance(response, bytes):
        name = pathlib.Path(args.path).name or "response"
        target = pathlib.Path(args.out or ".")
        target.mkdir(parents=True, exist_ok=True)
        (target / f"{name}.bin").write_bytes(response)
        print(f"wrote {target / (name + '.bin')} ({len(response)} bytes)")
        return
    jobs = []
    if isinstance(response, dict):
        jobs = [response["background_job_id"]] if response.get("background_job_id") else list(response.get("background_job_ids") or [])
    if args.wait:
        for job in jobs:
            response = {"job": wait_for_job(job), "request": response}
    if args.out:
        print("wrote", ", ".join(save_result(response, args.out, "response")), "in", args.out)
    else:
        dump(save_images(response, "pixellab-out") if "base64" in json.dumps(response) else response)


def cmd_tileset(args):
    body = {
        "lower_description": args.lower,
        "upper_description": args.upper,
        "tile_size": size(args.tile_size),
        **({"transition_description": args.transition} if args.transition else {}),
        **({"view": args.view} if args.view else {}),
        **({"seed": args.seed} if args.seed is not None else {}),
        **parse_pairs(args.set),
    }
    started = send(args, "POST", "/create-tileset", body)
    if started is None:
        return
    tileset_id = started["tileset_id"]
    print(f"tileset {tileset_id} (job {started.get('background_job_id')})", file=sys.stderr)
    result = wait_for(lambda: request("GET", f"/tilesets/{tileset_id}"), f"tileset {tileset_id}")
    files = save_result(result, args.out, "tileset")
    print(f"wrote {', '.join(files)} in {args.out}")


def download_character(character_id, out, spritesheet=False):
    path = f"/characters/{character_id}/{'spritesheet' if spritesheet else 'zip'}"
    payload = wait_for(lambda: request("GET", path), f"character {character_id}")
    where = save_zip(payload, out, "spritesheet" if spritesheet else "character")
    print(f"wrote {where}")


def cmd_character(args):
    path = "/create-character-with-8-directions" if args.directions == 8 else "/create-character-with-4-directions"
    body = {
        "description": args.description,
        "image_size": size(args.size),
        **({"view": args.view} if args.view else {}),
        **({"seed": args.seed} if args.seed is not None else {}),
        **parse_pairs(args.set),
    }
    started = send(args, "POST", path, body)
    if started is None:
        return
    character_id = started["character_id"]
    print(f"character {character_id}", file=sys.stderr)
    if started.get("background_job_id"):
        wait_for_job(started["background_job_id"])
    download_character(character_id, args.out)
    (pathlib.Path(args.out) / "character-id.txt").write_text(character_id + "\n")


def cmd_animate(args):
    body = {
        "character_id": args.character,
        **({"animation_name": args.name} if args.name else {}),
        **({"action_description": args.description} if args.description else {}),
        **({"frame_count": args.frames} if args.frames else {}),
        **parse_pairs(args.set),
    }
    started = send(args, "POST", "/animate-character", body)
    if started is None:
        return
    for job in started.get("background_job_ids") or []:
        wait_for_job(job)
    download_character(args.character, args.out)


def cmd_download(args):
    download_character(args.character, args.out, args.spritesheet)


def cmd_job(args):
    dump(wait_for_job(args.id) if args.wait else send(args, "GET", f"/background-jobs/{args.id}", None))


def build_parser():
    parser = argparse.ArgumentParser(description="Pixel Lab API client", formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)

    def command(name, handler, help):
        sub = commands.add_parser(name, help=help)
        sub.add_argument("--dry-run", action="store_true", help="print the request instead of sending it")
        sub.set_defaults(handler=handler)
        return sub

    command("balance", cmd_balance, "credits and subscription")

    sub = command("spec", cmd_spec, "list endpoints, or one endpoint's fields, from the OpenAPI document")
    sub.add_argument("path", nargs="?")
    sub.add_argument("--filter", action="store_true", help="match endpoints containing PATH, not equal to it")

    sub = command("call", cmd_call, "any endpoint")
    sub.add_argument("method")
    sub.add_argument("path")
    sub.add_argument("--json", help="the request body, as JSON")
    sub.add_argument("--json-file")
    sub.add_argument("--set", action="append", metavar="KEY=VALUE")
    sub.add_argument("--wait", action="store_true", help="poll the job(s) the response names")
    sub.add_argument("--out", help="a folder to save the response and its images in")

    sub = command("tileset", cmd_tileset, "a top-down tileset between two terrains")
    sub.add_argument("--lower", required=True, help="the lower terrain, e.g. 'grass'")
    sub.add_argument("--upper", required=True, help="the upper terrain, e.g. 'dirt path'")
    sub.add_argument("--transition", help="what lies between them")
    sub.add_argument("--tile-size", default="16", help="16, 32, or WxH (default 16)")
    sub.add_argument("--view", choices=["low top-down", "high top-down"])
    sub.add_argument("--seed", type=int)
    sub.add_argument("--set", action="append", metavar="KEY=VALUE")
    sub.add_argument("--out", required=True)

    sub = command("character", cmd_character, "a character with 4 or 8 directions, downloaded as a ZIP")
    sub.add_argument("--description", required=True)
    sub.add_argument("--size", default="48", help="each rotation's size, N or WxH (default 48)")
    sub.add_argument("--directions", type=int, choices=[4, 8], default=4)
    sub.add_argument("--view", help="e.g. 'low top-down'")
    sub.add_argument("--seed", type=int)
    sub.add_argument("--set", action="append", metavar="KEY=VALUE")
    sub.add_argument("--out", required=True)

    sub = command("animate", cmd_animate, "animate an existing character, then download it")
    sub.add_argument("--character", required=True, help="a character id, from `character` or the web app")
    sub.add_argument("--name", help="the animation's name, e.g. walk")
    sub.add_argument("--description", help="what the animation does")
    sub.add_argument("--frames", type=int)
    sub.add_argument("--set", action="append", metavar="KEY=VALUE")
    sub.add_argument("--out", required=True)

    sub = command("download", cmd_download, "download an existing character")
    sub.add_argument("character")
    sub.add_argument("--spritesheet", action="store_true", help="the packed sheet and its layout, not the ZIP of frames")
    sub.add_argument("--out", required=True)

    sub = command("job", cmd_job, "a background job's status")
    sub.add_argument("id")
    sub.add_argument("--wait", action="store_true")
    return parser


def main(argv=None):
    args = build_parser().parse_args(argv)
    try:
        args.handler(args)
    except ApiError as error:
        raise SystemExit(f"pixellab: {error}") from None


if __name__ == "__main__":
    main()
