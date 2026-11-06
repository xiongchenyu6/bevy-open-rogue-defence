# Cave spatial prototype: native technical pass, art not accepted

This is a controlled capture fixture, not a normal-play or packaged-game acceptance. Production assets and the default arena camera remain unchanged.

The optional `den-preview-front-board` camera looks from (-9, 25.2, 21) toward (5, 1.2, 0), vertical extent 28. Existing optional camera poses keep their numerical values. The three actual enemy models are controlled moving actors; this does not prove natural wave balance.

Two native runs completed 400 updates each, exit 0, full capture PASS, nine saved PNGs per case, no ERROR or panic. Strict release capture Clippy and the capture build exited 0. Source and assets stayed frozen throughout those commands. `source-inputs.json` pins the tested inputs; `capture-source.rs` is the tested camera source. The frozen executable hash is in `binary.json`; its binary remains in target and is not a portable game package.

Root inspected front-board frames 90, 180 and 350. Empty entrance and actors after exit are readable; the player, gate and camp controls remain visible. The gray structure still resembles a concrete hangar with oversized planar walls. The decorative upper roof is cropped. This is useful spatial evidence, not accepted final modeling/materials, production framing, or interaction validation.

The independent exported-geometry check reports 96 height bands, continuous spawn Z [-4.25, 4.25], and zero collisions against the sampled animation envelope. That envelope samples 25 animation phases; it does not prove all interpolated poses. See `independent-path.json`. The candidate metadata predates this native run and retains its earlier pending-native wording; this README and `results.json` record the later outcome without rewriting the original receipt.

Prototype source: CC0 LowPoly Cave Entrance, OrbitStudios / Fernando Ferreira, submitted by zisongbr: https://opengameart.org/content/lowpoly-cave-entrance . The included GLB is a gray spatial adaptation, not finished production art. Original source/license and local geometry adaptation details remain in target/verification/handcrafted-cave-prototype-flare-2026-11-06.
