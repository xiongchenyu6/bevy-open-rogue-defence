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
- Browser regressions: 10 cache, 11 demand and 16 dialog scenarios passed.
- Python: 2 manifest, 3 retained-release and 9 engine-reuse tests passed.
- Engine tests include the real build wrapper in an isolated fixture, checking
  content-only reuse and rebuilding changed compiled inputs.
- Dialog screenshots: chapter-download-mobile.png and offline-manager-mobile.png.
- Actual built game online/cold-offline verification is recorded separately after
  completing the real renderer run. UI tests alone are not proof of combat rendering.

The browser may evict website storage or the player may clear it. Missing files
then need downloading again. Physical Android installed-PWA testing remains outside
these Chromium tests.
