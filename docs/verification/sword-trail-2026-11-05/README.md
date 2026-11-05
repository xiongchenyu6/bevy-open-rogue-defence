# Starter sword motion trail

The initial Stone Warden slash was a wide, opaque sawtooth strip. It stayed solid throughout recovery and disappeared at expiration, covering enemy silhouettes. Levels1–9 now use a narrower smooth tapered arc and six prebuilt transparent phases (alpha0.78→0.64→0.46→0.28→0.12→0.03). Starter Solar keeps its original smooth geometry and uses the same fading phases. Each live trail switches immutable shared material handles; a new attack cannot brighten an old trail and no materials are allocated per swing. Other weapons and level10/20/40 styles retain their existing mesh/material paths.

## Verification

- Optimized full library: **662 passed, 0 failed, 1 ignored**. New production-system tests cover independently aging trails, pause freezing, finish cleanup, repeated swing material stability and hero/level boundaries. Initial run had661pass/1failure because an existing fixed startup asset-budget assertion still expected12meshes/48materials. The correct fixed budget is13meshes/60materials; no runtime allocation cap was removed.
- Strict optimized all-target Clippy with `-D warnings` passed.
- Four real-attack native before/after scenarios: Stone sword, Solar sword, level40sword and archer. The after runs each completed250updates with16 saved frames; baseline ordinary cases each completed210updates, with an additional250update upgraded case to reach its existing frame230assertion. No panic, actual damage and material-finish checks passed. These use the real damage resolver and authored animations against durable stationary fixture targets, without learned skills, automatic ultimates or Autoplay. They are not natural earned progression, performance or campaign tests.
- `compare.py` compares the actual first80 synchronous damage/HP/animation observations per scenario. All320observations are identical before/after. Starter traces additionally record monotonically fading per-entity opacity0.78→0.03; upgraded trails remainOpaque/alpha1. Archer has no sword trails.
- Root inspected initial Stone contact/recovery before/after, Solar recovery, upgraded sword and archer control frames. PNG requests are asynchronous; use the synchronous logs for timing and opacity, not screenshot filenames alone. This improves one starter effect and does not certify all combat art or subjective feel.

## Reproduction

Build the native capture binary with the repository's Nix shell, target`target/market-visual`, dev and dependency opt-level0 and debug info0. Set isolated campaign/tutorial/audio/detail/display preference files, actual native display and Vulkan ICD. Run `ROGUE_CAPTURE_ATTACK_TRACE=1 ROGUE_CAPTURE_SAVE_FRAMES=18,19,20,21,22,23,24,25,30,35,40,45,50,55,60,65 nix develop --command target/market-visual/debug/capture frames OUTPUT 250 attack-sword 0xA11CE 1280x720`. Repeat`attack-solar`,`weapon-growth-sword-upgraded`,`attack-ember`. Run `python3 docs/verification/sword-trail-2026-11-05/compare.py` for the archived observations.
