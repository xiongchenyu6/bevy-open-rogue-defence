# Parley 0.9.0 CJK word segmentation

This directory contains the published Parley 0.9.0 library source and its original
Apache-2.0/MIT licenses. The project still uses Bevy 0.19.0, whose Parley 0.9
requirement cannot use the later `complex-scripts` feature.

The only library source change is in `src/analysis/mod.rs`: construct the word
segmenter with `WordSegmenter::new_dictionary` instead of the non-complex-script
constructor. This loads the compiled CJK dictionary already supported by ICU
2.2. It follows the dictionary constructor selection shipped behind
`complex-scripts` in Parley 0.10.0. Normal Chinese line breaking stays unchanged.
No logs are filtered and no fallback shim is added.

Sources:
- https://crates.io/crates/parley/0.9.0
- https://github.com/linebender/parley
- https://github.com/bevyengine/bevy/pull/25674

`UPSTREAM_SHA256.json` records hashes of the original published files. Only the
analysis module should differ. Package metadata, public API, and version are
unchanged. When upgrading the engine to a release exposing the upstream feature,
use that feature and remove this local dependency override entirely.

Ship `LICENSE-APACHE` and `LICENSE-MIT` with the game's third-party notices.
