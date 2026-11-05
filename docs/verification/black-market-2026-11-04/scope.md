# Black Market visual redesign evidence

The before screenshots are preserved from the native controller route recorded on 2026-10-14. They use fixture funds and deterministic stock, not earned equipment progression. They show the old text-only offer layout at 1280×720 and 667×375.

Final verification completed on 2026-11-05. The `en-1280x720`, `en-667x375`, `zh-1280x720` and `zh-667x375` folders contain actual NVIDIA RTX 3070 Vulkan renders from the changed source, using fixture funds and seeded production stock. They are not evidence of naturally earned economic progression.

Each folder records normal offers (frame35), hover preview (50), marked auto-buy (70), purchase feedback (96), unaffordable inspection (125), and controller focus (145). The native probe checks five visible tiles and controls, touch inspection without spending, exact displayed-price deduction, no unaffordable spending, real D-pad navigation into an unaffordable offer, clock frozen while open and resumed by Close. The primary agent reviewed desktop/compact and English/Chinese representative states, including the clean thin Skill Tuner silhouette and a single controller focus. Item art is shared with inventory and forge.

`controller-route.log` completes all 80 steps through actual synthetic gamepad events, buys the five fixture offers via preview/Buy, forges Bloodreaver and returns to the chapter screen. `10-forge-ready.png` and `11-forged.png` preserve the inventory/forge presentation. This is synthetic native input verification, not a physical-controller or Steam Input hardware test.

`atlas-readiness.log` exercises a real controlled missing atlas: Loading, Failed with battle gated, actual Retry click, then Loaded with all 25 required assets ready. It caught a repeated per-frame `load` call silently restarting failed requests; production now reuses the retained asset handle. `atlas-retry.png` shows the restored battle after recruitment. The generated atlas uses explicit measured UV rectangles, after native screenshots caught neighboring-art fragments in equal-grid slicing. `tools/verify_market_art.py` checks all 24 runtime identities/UVs against the source image, manifest and hash.

Full library suite: **658 passed, zero failed, one ignored** (`tests-visual.log`). Strict Clippy passed (`clippy-final.log`). The unoptimized visual executable uses dev/dependency opt-level 0, debug info 0, one Cargo job. It is not a release build or performance benchmark. Binary and relevant source hashes are in `evidence.json`.

Reproduce with isolated campaign/tutorial/audio save environment variables, `VK_ICD_FILENAMES` pointing to the NVIDIA Vulkan ICD, and:

```sh
nix develop --command cargo build --locked --bin capture -j1 \
  --target-dir target/market-visual --config 'profile.dev.opt-level=0' \
  --config 'profile.dev.package."*".opt-level=0' --config 'profile.dev.debug=0'
ROGUE_COMMANDER_LANG=zh ROGUE_CAPTURE_SAVE_FRAMES=35,50,70,96,125,145 \
  nix develop --command target/market-visual/debug/capture frames \
  target/verification/market-art-2026-11-04/zh-667x375 \
  160 market-art 0xA11CE 667x375
```

Repeat for English and 1280x720. Additional routes are `market-atlas-readiness`
(still, 150 frames) and `controller-input` (frames, 2400 frames). The controller
route has a finite 2200-frame stall guard, extended for the five added explicit
preview purchases. Captures are accepted only with their PASS/completion log
lines and no panic, not merely a process exit status.
