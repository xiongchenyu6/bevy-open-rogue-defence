# On-demand downloads and small updates

The launcher prepares the engine, menu, font, icons and chapter configuration.
Combat models, audio and world resources wait until the player selects a chapter.
All six current chapters use the same combat assets, so they share one pack; selecting
another chapter does not download another copy. The contextual preparation dialog
shows missing bytes and offers download, cancel and retry. The existing settings
panel opens Downloads & offline play, including explicit full offline preparation
and removal of extras. Active expeditions protect their resources; saves are untouched.

The local release contains 18,851,184 bytes of startup content, 23,360,980 bytes of
shared combat content and 115,160 bytes of optional supporting files. These are
unique resource bytes with the gzip engine selected, not browser request overhead.

Every resource has a SHA-256 digest and immutable URL. Updates fetch missing or
changed files only, validate them before activation, and preserve the old release
on failure. Updates do not acquire packs that the player has not installed. Pages
retains immutable resources for the current and two previous releases, allowing
supported old open clients to fetch their own original assets.

Chapter balance is external JSON (733 bytes), validated both at build and runtime.
Menu, enemies and wave timeline consume the same catalog. Changing this file does
not require a new engine. Build inputs are fingerprinted; content-only deployments
reuse the exact verified published JS/WASM bytes. This avoids unnecessary compiler
drift. A real engine change still downloads the entire compressed WASM file.

Fixed-size WASM chunking was evaluated against the preceding engine. All 276
256-KiB chunks and all 69 1-MiB chunks changed. Independently compressed chunks
increased transfer size compared with the 14,544,538-byte whole gzip, so this release
does not ship fixed chunking. This is not a claim about content-defined chunking or
binary patches. See wasm-chunk-evaluation.json.

## Verification

- Native library: 843 passed, 0 failed, 1 existing statistical report ignored.
- WASM library check and Rust formatting passed.
- Browser regressions: 10 cache, 11 demand and 17 dialog scenarios passed.
- Python: 2 manifest, 3 retained-release and 9 engine-reuse tests passed.
- Engine tests include the real build wrapper in an isolated fixture, checking
  content-only reuse and rebuilding changed compiled inputs.
- Dialog screenshots: chapter-download-mobile.png and offline-manager-mobile.png.
- Actual built Bevy game: fresh menu made zero combat-resource requests; chapter
  preparation completed before recruitment. Closed the browser process and opened
  the same profile offline before navigation: recruitment and battle rendered with
  no failed resource requests or engine errors. Screenshots require both upper
  health bars and lower action controls; online and offline images were inspected.
  See local-game-results.json, battle-online.png and battle-offline.png.
- The real settings screen displays the downloads entry (settings-entry.png).
  The published build also opens and closes the real settings downloads dialog
  (published-settings-downloads.png).
- The local engine predates the final configuration-loading label marker fix;
  the final public-build test below verifies the deployed engine separately.

The browser may evict website storage or the player may clear it. Missing files
then need downloading again. Physical Android installed-PWA testing remains outside
these Chromium tests.

## First-install activation race

The first public probe reached the real menu but had 1,642,646 startup bytes
missing. It was rejected as a cache pass. Instrumentation showed that preparation
returned false: the first worker activated while its STATUS reply was pending,
and reading `registration.waiting` again returned null. The fix keeps the inspected
worker reference across the await. A deterministic browser regression simulates
that transition and checks that a non-rendered core file is prepared before Start.
All 17 startup UI scenarios pass after the fix. This is a web-only change; the
published engine can be reused byte-for-byte.

## Published incremental release

[Pages run 43700375306](https://github.com/xiongchenyu6/bevy-open-rogue-defence/actions/runs/43700375306)
completed successfully for `a7d867d5b67f1b8fecf42ce971cc21aba643436c`.
The prior engine was reused with verified hashes. Comparing the two published
resource manifests shows exactly one changed game file: `game-cache.js`, **9,807
bytes**. No engine, configuration, icon, model or audio file changed. This excludes
service-worker/version-check metadata overhead. See published-small-update.json,
published-engine-reuse.json and published-build.json. The release index retains
both releases (a third will be retained once a third release exists).

The final fresh-profile test against the public URL identifies the exact release
above. Startup core is complete before the menu (18,684,040 bytes with gzip); no
combat requests occur before chapter selection. Preparing the shared 23,360,980-byte
battle pack leads to actual recruitment and combat. The real settings button opens
and closes Downloads & offline play. After closing the entire browser, restarting
the same profile with networking disabled still reaches recruitment and combat.
Core and combat report zero missing bytes; there are **zero failed resource
requests and zero engine/page errors**. Optional supporting files remain unrequested.

See published-game-results.json and the published battle screenshots. The offline
HUD detail is a crop of that same screenshot; upper health bars and lower action
controls are visible. This is Chromium/SwiftShader testing, not a physical Android
installation test.
