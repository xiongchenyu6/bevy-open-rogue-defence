# Public timer pixel recognition — 2026-11-05

040ce5c natural-forge attempt stopped after112.19seconds while the real battle screenshot displayed23:39. Original600% general OCR returned `oe`; the same screenshot is preserved as clock2339.png. This is an observation failure, not a game loss or crafting result.

The common timer reader now uses only its existing100×28 screen rectangle, decimal/colon whitelist and single-line OCR at100/200/600percent. At least two distinct scales must return the same complete MM:SS; malformed seconds, disagreement and only one valid reading stop input. Actual native100/200percent reads both returned23:39;600percent remained empty. No neighboring clock, expected progress, resource or game state is used.

56 tests passed in the isolated runtime image (`tests.log`), including this unchanged PNG, older native time fixtures, actual covered recruitment screenshots, ambiguous/insufficient readings, shared overlay precedence and exact countdown format. Complete fresh natural acquisition/crafting/resume/quit still needs a runtime result.
