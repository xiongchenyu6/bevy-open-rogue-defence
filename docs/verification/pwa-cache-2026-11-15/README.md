# Persistent game resources and explicit updates

The previous worker was network-only. This release adds a SHA-256 file manifest,
content-addressed CacheStorage, and a startup-only update offer. A resource is
validated before storage; identical bytes are reused across releases. Metadata-only
revision changes do not create resource updates. Gzip-capable clients cache only
the compressed engine, not both compressed and raw WASM. The build uses `gzip -n`
to exclude timestamps/filenames; identical engine content produces identical
compressed bytes (verified after changing the input file timestamp).

Start saves the complete selected resource set before initializing Bevy. Subsequent
starts read local resources. The worker's small update check still uses the network;
loading and decompressing local WASM is not another asset download.

Updates require Download & update. Later preserves the committed version, including
after every window closes and the browser naturally activates a new service worker.
The new version is committed only after every required resource passes integrity
validation. Existing tabs remain bound to their own resource manifest. Interrupted
or corrupt updates retain the previous complete version. The game does not reload
in the middle of a run or add a persistent battle HUD control.

The browser may refuse persistent storage or evict website data. Cache completeness
is checked on startup; missing resources require downloading again. Browser user
data clearing is not something an app can override.

## Verification

- Generator: 2 unit tests cover hashes, reproducibility, metadata-only releases,
  WASM variants and invalid builds. JS and build-script syntax checked.
- Real service-worker and UI integration: see [browser tests](browser-tests.md).
- **Actual game, not an engine stub:** served the current published JS/WASM with
  the new startup/SW files and actual local game assets. First Start cached
  **42,110,681 bytes**. Closed the entire browser process, relaunched with network
  disabled before navigation, then clicked Start. Bevy/WebGL2 reached the real
  chapter selection screen. No page or console errors. [Result](game-results.json).
- [First online start](game-first-online.png), [cold offline start](game-cold-offline.png).

The browser tests use Chromium, including narrow mobile viewports; they do not
claim physical Android home-screen testing. Local reproduction:

```sh
python3 -m unittest discover -s tools -p test_generate_web_manifest.py
node tools/test_pwa_cache.cjs
node tools/test_pwa_update_ui.cjs
PWA_GAME_URL=http://localhost:8767/ node tools/test_game_offline.cjs
```

The Node tests require Playwright and Chromium. Set NODE_PATH when using a shared
Playwright installation and CHROMIUM_EXECUTABLE_PATH when Chromium is outside its
default installation. The last test requires a built game served on localhost/HTTPS.

## Published release verification

[Pages run 43686196543](https://github.com/xiongchenyu6/bevy-open-rogue-defence/actions/runs/43686196543)
completed successfully. Public build receipt and 241-file manifest identify
`3bb9e32c4fb1df092997d7b99a6386dc21aa41a8`.

The first probe immediately after publication entered the game but did not report
a complete resource cache; it was not accepted as a cache pass. A new-browser
rerun against the public release saved **42,112,843 bytes**, reported zero missing
bytes, then restarted the entire browser with networking disabled before navigation.
The actual game rendered chapter selection with no page or console errors.
[Published cache status](public-cache-status.json), [result](public-game-results.json),
[offline screenshot](public-cold-offline.png). The initial miss is consistent with
stale startup content during propagation, but its exact cause was not proven.
