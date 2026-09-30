# Published site directory

`directory.json` lists every published project and documentation route, with
its category, source location, description and searchable topics.

Pages builds the ordinary exports and docs first, then runs
`python site/build_directory.py target/pages`. The builder refuses duplicate
routes, missing targets and published routes absent from the catalog. Adding a
published route requires adding its directory entry in the same change.

The generated page is published at `https://sindri.vardir.no/directory/`.
The landing page and every documentation page link to it. Search and category
filters work together and are stored in `?q=...&type=...` for sharing. Without
JavaScript, all entries remain accessible.

`node scripts/browser/directory.mjs target/pages` tests the assembled page at
desktop and portrait sizes, including search, category filters, shared URLs,
empty-result recovery, overflow and the no-script fallback. It uses the existing
Playwright dependency in `scripts/browser`; Pages retains both screenshots.
