# Strict current-market preview polling — 2026-11-05

040ce5c r3 chapter6 route stopped after303.29seconds during the first black-market visit. The pointer had moved to Arc Lens (tile180), but the actual detail preview still showed Wind Boots. The old five-second polling bound consumed nearly its whole time reading the full screen and stopped after only one stale observation. No purchase, campaign victory or loss is inferred.

Measured on that unchanged screenshot: full-scene OCR4.011s; exact current BLACK MARKET title crop0.153s; tile/price/preview crops0.438s. The original stale screenshot, positive initial panel, measured text/times and failure log are preserved.

Both natural-forge and campaign routes now share one poller. Each fresh screenshot must show the exact market header, then exact matching tile name, price, category and detail preview (plus unchanged expected identity before buying). Stale previews are recorded and retried; the20second per-offer bound is clipped to the existing remaining visit deadline and ten observations. It never clicks to select, guesses prices, accepts battle/modal images, or skips an offer.

64 tests passed (`tests.log`), including actual positive/stale/negative pixels, two-observation stale→matched progression, wrong price/identity and deadline rejection. Obsolete duplicate polling code and unused imports were removed. These parser/driver regressions do not establish a new complete live route; fresh natural acquisition/crafting/resume/quit and campaign settlement remain required.
