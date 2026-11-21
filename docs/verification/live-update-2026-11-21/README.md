# Live installed-client update availability — 2026-11-21

## Concrete bug and change

`discover()` returned immediately after the engine started. The online listener
also stopped checking then. Downloads settings had no update controls. A long-lived
installed client could therefore continue running an old engine without seeing
an available release. This gap is verified independently of the reported hero
outside-road screenshot; the screenshot's running revision is unknown.

The launcher now checks again when it becomes visible, reconnects, and every five
minutes while visible. Checks install only the worker/manifest, not new game
resources. A new release produces one noninteractive ten-second notice and is
available through the existing Downloads settings. There is no new permanent HUD
button. Settings shows current/available short revisions, an explicit update
button, and blocks activation while an expedition is active. Returning to the
chapter menu enables it. A rapid Start that races the initial status check offers
an already-waiting update first; Later still deliberately starts the installed
version. Failed updates keep the existing version playable.

## Acceptance

The production HTML/CSS/JavaScript and service worker ran in real Chromium with
real HTTP, CacheStorage, persistent profiles and revision changes. The heavyweight
Bevy engine is **stubbed in these delivery tests**. These are not WASM gameplay or
physical Android/iOS tests. Separate native gameplay evidence is in
`../road-boundary-2026-11-21/README.md`.

- 20 startup/update UI scenarios passed (`update-ui.log`, `results.json`).
- 10 cache/offline/integrity scenarios passed (`cache.log`).
- 11 on-demand/group-update scenarios passed (`demand.log`).
- Both new landscape-phone screenshots inspected: explicit update is visible,
  disabled during the expedition and enabled after returning to chapters. The
  existing dialog scrolls to the remaining download/back actions.

The new cases assert that a running v1 client learns about v2 without reloading or
downloading engine/assets; even a synthetic click cannot activate during the
expedition; after chapter release explicit activation starts v2; unchanged WASM
is fetched zero additional times. A deterministic delayed initial STATUS test
verifies rapid Start cannot bypass an already-waiting update and Later still
runs v2 when v3 is available.

## Reproduce

```sh
NODE_PATH=/tmp/rogue-pwa-test-deps/node_modules \
CHROMIUM_EXECUTABLE_PATH=/etc/profiles/per-user/freeman.xiong/bin/google-chrome-stable \
node tools/test_pwa_update_ui.cjs
```

Use the same environment with `tools/test_pwa_cache.cjs` and
`tools/test_pwa_demand.cjs`. Playwright is installed only in `/tmp`, not added to
the repository. Native gameplay movement was not changed in this patch.
