# Combat framing and impact feedback — 2026-11-06

Production source adopts the authored cavern and responsive fixed-angle camera; movement and dodge follow camera axes. Narrow portrait toolbar/Touch footer avoid the reviewed overlaps. Surviving critical contacts request a 25ms hit-stop at 1–2x; ordinary hits and normal kills retain their old behavior, and fast-forward skips hit-stop. Same-frame requests use their maximum and the existing 0.5s cooldown prevents continuous freezing. Damage coefficients, cooldowns and attack contact phases are not tuned here.

## Actual verification

Strict release all-target Clippy: exit0. Full release library: **726 passed, 0 failed, 1 ignored**. Development capture build: exit0. Five Python model-scanner tests also passed; all53 authored GLBs passed the runtime model check. The test-only Rust module scanner fix preserves actual runtime references.

18 native cases completed5320 updates and184 PNGs, with exit0/no ERROR or panic. Four viewport sizes cover EN1280×720, ZH667×375, EN390×844 and ZH375×667; each runs the production cavern, controlled max-scale scene and keyboard/Touch four-corner camera fixtures. Two additional impact fixtures run at1x/8x with100 same-frame critical events. `review.json` lists the ten R6 images actually viewed by root; only those ten are copied here, plus the rejected R5 footer images. R5 technical checks had passed but its pixels failed: a compressed joystick and an avatar under XP/skill controls. The R6 pixels show a round joystick and a clear avatar in those cases.

The impact inputs are **synthetic presentation events**, not real damage-resolver or subjective feel acceptance. Capture uses fixed1/30s updates: a25ms request is visibly quantized to one33.333ms update. Logs record1x→.06x→1x, ordinary frame90 at1x and8x remaining8x throughout. Units test continuous critical cooldowns,100-event aggregation, pause/finish, user speed changes, resizing and input transforms. No new full campaign, hardware audio, physical touch/controller, or standalone package acceptance is claimed here.

The max-scale fixture asserts root passage and sampled geometry has a separate [asset clearance proof](../intact-cave-C-asset-2026-11-06/README.md); it does not assert whole-body clearance for every animation pose. World labels can still overlap world content at edge positions; this acceptance is limited to the reported fixed-HUD regressions. The camera currently assumes one full primary-window viewport.

## Recheck

`python3 docs/verification/combat-framing-feedback-2026-11-06/verify.py` verifies archived hashes and recorded outcomes, not a fresh graphics run. Add `--repo` to also verify all frozen runtime source/asset bytes against this checkout. The recorded managers contain the actual commands and input manifests; reruns must use new output directories. No binary is stored in Git; its actual hash is in `checks/binary.json`.
