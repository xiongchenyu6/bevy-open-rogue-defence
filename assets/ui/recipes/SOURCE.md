# Crafted equipment illustrations

`recipe-atlas.png` contains six original AI-generated fantasy inventory illustrations created for Rogue Commander on 2026-11-05 using the built-in `image_gen` tool. No third-party game artwork was imported. `generation.json` records the exact six prompts, transparency requests, tool, generated-file paths and absence of an exposed seed. `manifest.json` records the source hashes, trim bounds, packing, final atlas hash and runtime regions.

The atlas is **768×512 RGBA**, a strict three-column, two-row grid of 256px cells. Row-major order matches `RecipeId`: Bloodreaver, AstralPrism, StormBastion; Suncoil, NightCrown, WorldEngine. Each item is proportionally scaled to fit 224×224 and centered, retaining at least a 16px fully transparent outer gutter. Runtime can use the exact full-cell rectangles from the manifest without sampling neighboring subjects.

The subjects use distinctive silhouettes reflecting their effects: a blood-channel cleaver, orbital violet-blue prism, winged protective shield, sun-and-copper-coil device, dark garnet crown, and orbital crystal engine. They deliberately do not reuse the silhouettes of the ordinary ingredients. The style matches the project's painted worn-metal equipment set with restrained jewel highlights.

Postprocessing is mechanical alpha-bounds trimming, proportional Lanczos resampling and transparent atlas composition only. There is no hand repainting or inferred recoloring. The final PNG is retained because this project's existing Bevy build enables PNG but does not enable WebP decoding; adding an engine dependency for six small icons is unnecessary. Runtime ships one 411,408-byte atlas rather than six full-resolution generated files or duplicate image formats.

Rebuild (when the original generated paths recorded in `generation.json` are available): `uv run --with pillow python tools/build_recipe_art.py`. Read-only validation: `uv run --with pillow python tools/verify_recipe_art.py`.

The contact sheet was manually inspected, and bitmap/catalog validation passes for all six unique nonblank illustrations, exact cells, retained alpha, and transparent gutters. Runtime integration and1540controlled native updates are now verified in `docs/verification/recipe-art-2026-11-05/`, including both desktop/compact languages, real crafting/effect checks and missing-atlas retry. Bitmap checks alone do not establish that runtime scope.
