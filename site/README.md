# Published site directory

`directory.json` lists every published project and documentation route, with
its category, source location, description and searchable topics.

Pages builds the ordinary exports and docs first, then runs
`python site/build_directory.py target/pages`. The builder refuses duplicate
routes, missing targets and published routes absent from the catalog. Adding a
published route requires adding its directory entry in the same change.

The generated page is published at `https://sindri.vardir.no/directory/`.
The landing page, the play pages and every documentation page link to it. Search and category
filters work together and are stored in `?q=...&type=...` for sharing. Without
JavaScript, all entries remain accessible.

`node scripts/browser/directory.mjs target/pages` tests the assembled page at
desktop and portrait sizes, including search, category filters, shared URLs,
empty-result recovery, overflow and the no-script fallback. It uses the existing
Playwright dependency in `scripts/browser`; Pages retains both screenshots.

## Play pages

Every catalog entry with a `play` block gets a page at `/play/<route>/` that
embeds its export from `/examples/<route>/` in an iframe, with what the
project is, how to play it and what it proves. `python site/build_play.py
target/pages` writes them from `play.html`, `play.css` and `play.js`, after the
exports and before the directory. It also fills the landing page's game and
example grids and its counts from the same catalog, so the landing page cannot
list a game that is not published or miss one that is. It refuses an unknown
`kind` or input, a missing `about` or `proves`, a named screenshot absent from
`site/screenshots/`, and a play page with no export behind it.

A `play` block holds:

- `kind`: `Flagship`, `Genre showcase`, `Feature example`, `Lab` or `Tool`.
  Flagships are featured on the landing page; genre showcases and the rest
  fill its two grids.
- `shot`: a file in `site/screenshots/`, used for the poster and thumbnails.
- `about` (paragraphs), `proves` (short phrases), `controls` (pairs of input
  and action), `input` (`keyboard`, `mouse`, `touch`, `gamepad`), and
  optionally `note` and `needs` (defaults to WebGPU).

The game loads only when **Play** is pressed. It then plays inline, or over
the whole window when **Full page** is on. The switch defaults to on for a
phone-sized or touch screen and off on a desktop, and is remembered per
browser. **Expand** enters full page at any time; full page shows only a
restart and a close button over the game, asks a phone for real fullscreen
where the browser allows it, and Back leaves full page rather than the page.
A directory entry with a `play` block links to its play page.

`node scripts/browser/play.mjs target/pages` checks the landing page links
every playable project, that each play page exists and does not load its game
early, that full page fills the window on desktop and phone, that Back and the
close button leave it, that the switch is remembered, and the no-script link.

## Deployment and WIP previews

Pages replaces the whole site on every deployment, so exactly one workflow
deploys it: **Sindri Site Publisher** (`.github/workflows/site-publisher.yml`).
`Sindri Pages` only builds. On `main` it uploads its build as the `stable-site`
artifact (kept 90 days). On a pull request it proves the site assembles, and a
pull request also uploads a `3d-labs-preview` artifact of the 3D labs. In the
merge queue it builds the optimised site for the exact commit `main` becomes and
uploads it as `queued-site`; the push to `main` then promotes that artifact to
`stable-site` instead of building it again (`scripts/ci_reuse.py pages`). A
`queued-site` is never served: the publisher reads only `stable-site` from
`main`.

Each publisher run computes the whole site from live GitHub state with
`python3 site/wip_previews.py plan`, then `assemble`:

- **Stable routes** come from the newest successful `Sindri Pages` run on
  `main` that has a non-expired `stable-site`. Nothing is rebuilt. The publisher
  never deploys without one, so it cannot erase `/examples/`.
- **`/wip/<pr>/`** is added for every open pull request from this repository
  whose *current head* passes all of these:
  - `CI` and `Sindri Pages` ran for that head;
  - every other `pull_request` workflow that ran for it passed (path-filtered
    workflows such as Decay count only when they ran);
  - no other app's check run or commit status failed;
  - that head's `Sindri Pages` run uploaded a non-expired `3d-labs-preview`.

  Runs are matched on the PR **head SHA**. A `pull_request` run checks out the
  synthetic merge commit, but GitHub records the run, its checks and its
  artifacts against the head SHA, so a merge SHA never matches. Fork PRs are
  never published.
- `/wip/` lists the live previews. `/wip/manifest.json` records the stable run
  and each preview's PR, head SHA and artifact.

Triggers: completion of any workflow listed in the publisher (on main or a PR
branch), a pull request closing, or a manual run. A manual run can name a
`pr_number`, and fails visibly if that PR is not published. Each run writes a
job summary with one verdict per open PR and the reason. It deploys only when
the planned site differs from the live manifest. It also skips the deploy when
the only change would be dropping a preview whose new head is still being
checked, so a push does not take a working preview down. Runs share one
concurrency group that never cancels a deploy in progress. Only the newest
queued run survives (older queued runs show as cancelled). Every run replans
from scratch, so nothing is lost.

Nothing from a pull request is executed with deployment credentials. Preview
artifacts are unpacked, checked for symlinks and the four labs (page,
manifest, player JS and WASM, `<base href="/wip/<pr>/<lab>/">`), and copied
below `/wip/<pr>/`.

### Why a preview is missing

Read the summary of the latest publisher run. Its most common reason is "no
pull_request workflow ran for head …". GitHub creates no `pull_request` runs
for a push made while the PR conflicts with its base, or made with a workflow
token. Push a commit, or use **Update branch**, so CI runs for the head. When
it is green, the preview publishes by itself.

### Merging a previewed PR

When the PR merges or closes, the next publisher run drops `/wip/<pr>/`.
For a merge, that run is started by `Sindri Pages` finishing on `main`. To
promote its demos to stable routes, the PR adds them to the stable export loop
in `pages.yml` (for example `"examples/transform:transform"`) and to
`directory.json` before merging. The merge's main build then serves them under
`/examples/<route>/`.

`python3 -m unittest discover -s site/tests` covers verification (head versus
merge SHA, missing, expired and stale artifacts, failed, pending and missing
checks, forks, manual dispatch) and assembly. CI runs these tests.
