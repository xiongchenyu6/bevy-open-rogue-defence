# Tiered enemy hit flash: controlled before/after evidence

The completed change is confined to `src/game/hit_feedback/enemy_flash.rs`.
All 388 recorded source/assets/Cargo input paths are identical except that file;
after-input hashes also match the completed after checks. The exact before bytes
come from `git show ee9b5a6:src/game/hit_feedback/enemy_flash.rs`; both source
copies and their digests are archived. The tested after bytes were an uncommitted
working change at capture time, not a claim that the old commit contains them.

Ordinary contacts now blend a modest tint with the original imported base color
instead of replacing the body with white/emission. Critical contacts retain a
stronger warm **whole-body** flash and additive RGB emission. Original alpha and
source/shared materials remain unchanged. This is not a new localized shader or
per-limb effect; no damage, critical roll, attack timing or balance tuning was
intended.

## Actual completed checks

All three recorded units (baseline, after, portrait supplement) ended inactive
with `Result=success`, `ExecMainStatus=0`. Final library tests: **744 passed,
0 failed, 1 ignored**; strict all-target release Clippy exited 0. Focused and new
simultaneous ordinary/critical tests, capture builds and all native children
exited 0. Actual receipts and full lossless logs are retained.

| Evidence | Actual scope/result |
| --- | --- |
| Synthetic before/after, 180 updates | Injected presentation HitEvent fixture; not real damage or HP resolution. Separate ordinary/dense/critical/heavy-kill sequences and recovery assertions completed. |
| Desktop real before/after, 250 updates | Real basic sword producer/resolver with learned skills/ultimate/Autoplay suppressed. Each side: 60 contact groups, 59 ordinary + 1 critical, 80 attack-timing rows, 60 resolver rows. |
| Portrait real before/after, 250 updates | Same real fixture at 720×1280; each side also has 60 contacts/59 ordinary/1 critical and 80 timing rows. Fifteen saved PNGs per side, not 250 PNGs. |

The independent portable verifier recomputes frame + HP/damage contact counters,
all 80 numeric animation/health/damage timing rows, and all 60 resolver counters
for **both** desktop and portrait pairs. It preserves multiplicity, frame,
critical-roll/HP bits, RNG transitions, damage values, elapsed bits, candidate
ordinal, progress bits and DamageProfile. Only transient entity identities and
the outer query ordinal are omitted from those cross-run comparisons. All four
runs match their respective before/after pair; this is not a claim of complete
hidden state or global replay determinism.

## Material comparison and pixels

`compare-native.py` is the final local full-desktop script, revision
`r2-color-space-contact-join`. It additionally requires the complete 250 actual
PNGs per desktop side at its original target location. Its actual final
`comparison.json` is PASS: 60 same-run actor/frame/HP-bit-matched contact groups,
600 material observations per side, no unmatched contact groups or unclassified
fresh swaps. LinearRgba base colors are converted to sRGB for palette comparison;
emissive remains linear and alpha is unchanged. Observed handle transitions are
reported separately: missing original-handle logs after probe100 do **not** prove
an unobserved recovery. `portrait-comparison.json` preserves root's independent
portrait contact/selected bitfield check; the portable verifier also checks the
stronger complete logged numeric/profile counters.

The retained `comparison-r1.json` had matching gameplay counters but 240/250
unclassified material swaps; its top-level PASS must not be interpreted as full
material-tier classification. The final color-space/contact-join analysis
supersedes that incomplete material interpretation without discarding it.

Root actually viewed synthetic frames9/89/119 and real desktop/portrait19/102
before/after; restoration109 was viewed on after-desktop and after-portrait,
not on the baseline109 sides. This archive includes exactly the requested
24 selected PNGs: synthetic9,18,89,98,119,128 each side; desktop19,102,109 each side;
portrait19,102,109 each side. Portable verification checks their exact hashes,
fileset and dimensions, **not** all 840 local PNGs or subjective visual quality.
The capture's contact update numbering and saved PNG numbering differ; do not
infer a universal exact first-render/contact timestamp from the filename.

`presentation-before-left-after-right.mp4` is an audio-free hstack of original
synthetic baseline/after PNG00000–00154, 155 frames at a reconstructed 30 fps:

```
ffmpeg -v error -framerate 30 -i baseline/frame%05d.png \
  -framerate 30 -i after/frame%05d.png \
  -filter_complex '[0:v][1:v]hstack=inputs=2[v]' -map '[v]' \
  -frames:v 155 -c:v libx264 -crf 20 -pix_fmt yuv420p \
  presentation-before-left-after-right.mp4
```

This is fixed-step presentation playback, not wall-clock video, audio-output or
hardware-performance evidence.

## Historical negatives retained

- `r1-format-scope`: an intentionally stopped build after broad formatting
  touched unrelated files. Its reason, stopped-unit state and logs are retained;
  no behavioral failure or passing completed build is inferred.
- `r2-emissive-alpha`: seven old focused flash tests passed, then the new tier
  test actually failed because adding emission also changed alpha to2 instead
  of1. The failed source/receipts/logs are retained. The final code adds RGB only;
  later successful tests are separate evidence.

## Portable verification and omitted local bulk

Run `python3 verify.py` without the simulator, assets, Cargo, GPU or target tree.
`logs/` contains every original log losslessly gzipped with raw byte hashes;
receipts, drivers, final and r1 comparators, frozen manifests and before/after
source are retained. `SHA256.json` covers exact archive bytes, and local
`.gitattributes` disables all text conversion for this directory.

The original 840 PNGs and two native capture binaries are **not** duplicated.
Their retained local paths/sizes/hashes are listed in
`local-full-frame-binary-manifest.json`; those paths are not portable evidence.
The local full comparator checked desktop250 PNG files at capture time; the
portable verifier intentionally cannot repeat that omitted-files check.

These controlled fixtures are not public physical input/audio tests, a full
campaign, Steam-client acceptance or hardware performance validation.
