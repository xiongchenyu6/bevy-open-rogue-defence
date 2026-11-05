# Public panel toggle and same-image observation — 2026-11-05

040ce5c chapter6 attempt stopped after572.55seconds on its third market visit. The preserved images show battle18:17, battle18:10, actual market18:10(wallet0262), then a fresh battle18:02 image. Input trace records two B requests before the market image. Game source closes an open market on a new toggle; exact OS/engine input latency was not measured, so it is not assigned a precise causal delay.

The definite harness issues were allowing another toggle merely because the battle clock advanced, and discarding the positively observed market image to read a new unclassified screenshot as a market wallet. No wallet-font or production market change was made.

The opener now allows one toggle per observed battle epoch. Only an explicitly recognized intervening choice followed by settled advancing battle can permit another epoch, within the existing deadline/request cap. It returns the exact observed image and rows; callers read that image. Current offer screenshots independently require a market panel before inspection. Unknown states stop input.

59 tests passed (`tests.log`), including the real sequence and strict one-toggle/same-image behavior. The successful opening screenshot has wallet262; the later ordinary battle is rejected as a market. This does not prove complete campaign/crafting routes.
