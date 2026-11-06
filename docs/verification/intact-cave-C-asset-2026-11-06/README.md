# Intact cave C: frozen asset candidate, not production adoption

The frozen candidate is [intact-cave-7cc6.glb](candidate/intact-cave-7cc6.glb), SHA256 `7cc6fe0dfbda094226f5f051492f8aee8c4a449fc90844c574cedf36995b7176`, 1,287,652 bytes. Actual exported-payload checks and the sampled swept-body SAT pass. The separate [native record](../intact-cave-C-native-2026-11-06/README.md) confirms entrance readability in controlled C capture; final art, production camera and gameplay/input adoption remain unverified. This is the asset evidence record only; any later production/source adoption is separate and is not accepted by these checks.

## Independent archived verification

From this repository, run:

```sh
python3 docs/verification/intact-cave-C-asset-2026-11-06/verify.py
```

This read-only stdlib verifier needs neither Blender, Nix, CUDA, target history nor network. It verifies every archived file, the original CC0 ZIP/BLEND and generated material hashes, the single whole-source deformation, actual final GLB payload and the actual negative predecessor. It reruns the archived SAT code against the included frozen envelope, pins exactly the historical52 model names/hashes from the actual positive log and all11 sampled source-unit hashes, and never reads mutable game assets or Git HEAD. The complete folder can be copied outside the repository and verified there. The historical unchanged52 claim belongs to the recorded actual run; it does not claim current formal assets are still unchanged.

Actual acceptance:

- Final exported GLB: 474 triangles, 96 height bands, continuous spawn Z `[-4.25,4.25]`, zero triangle/body-box intersections, 52 pre-existing models unchanged.
- Original lateral9 candidate `02e8fda...`: actual SAT exits1, 30 bands27–56 (`Y1.35..2.85`) collide with positive-Z neck triangles309/350/397/459 (309 is the main reported face). Its complete GLB, JSON, original commands and failure logs remain under [negative](negative).
- The old decorative `5.25..6.55m` height guard also actually exits1. Only that arbitrary decorative size policy was replaced by finite sane bounds. The complete geometry/world transform/finite-path/SAT math and final collision assertion are AST-pinned unchanged; only source-hash lookup is resolved against the archived historical baseline rather than mutable assets. No body margin, pose sampling or finite path was weakened.
- Standard PBR: one embedded1024×1024 PNG, actual UV coordinates, roughness≈.97, metal0, opaque; no external textures, lights, cameras, skin, animation or required extensions.

The original target checker invocations and logs are retained verbatim under [logs](logs) and [executed](executed). The executed scripts preserve their original target paths as a historical run record; the supported portable verification entry point is the archive's `verify.py`, not those historical builder scripts.

## Geometry and baked material

The original CC0 mesh remains279 source vertices/474 triangles. Its final mapping is one global lateral9.5 / vertical2.8 / depth10 scale plus orientation/translation. The mouth faces world−X at approximately X14, and actual enemy roots still spawn X`[15.3,16.4]`, Z±4.25. The only change from the failed9 version multiplies every world-Z coordinate by9.5/9; every world-X and height coordinate is identical. There is no local aperture extrusion, local flare, cutaway change, rock-sheet assembly or collider/spawn change. Per-vertex before/after data are [archived](candidate/whole-source-lateral-change.json).

The original six rear-backing triangles remain dark textured stone. The original neck is real geometry, about10.218m deep; the material does not fill the opening with bright rock. The original generated seamless slate and actual geometry AO were baked into a real1K UV texture. Board and actors were hidden during the bake so their dynamic AO could not be imprinted. Blender-only BOX/AO nodes were removed before glTF export. [Actual exported glTF reimport](cpu/actual-reimport-C-after-exit.png) was rendered and visually inspected without white/missing material. The source texture is a generated seamless image, never a scanned-model UV atlas used as a tiling texture.

The reference C pose is eye`(-20,25.2,10)`, target`(4,1.2,0)`, fixed vertical28. CPU actor roots at X12.5 show the three static real GLB references outside the mouth. The [separate native fixture](../intact-cave-C-native-2026-11-06/README.md) checks moving actor roots belowX10; it does **not** assert all sampled bodies have crossed the more conservative SAT finite-exit bound7.3637, nor all animation interpolation. Native root reviewed C90/180/350 for readability, not complete physical input/camp-click acceptance. Default production camera was visually rejected for this candidate. Decorative mountain portions may crop; main-mouth/body/HUD usability still requires normal gameplay/UI validation before any production change.

## Source rights and boundaries

See [source credits](source/CREDITS.md), original author page and complete unmodified source ZIP. The source mesh is CC0. The independently generated stone material has separate provenance and is not mislabeled third-party CC0. This archive does not install the model into formal assets, alter global manifests, ship a new package or adopt C as production camera.
