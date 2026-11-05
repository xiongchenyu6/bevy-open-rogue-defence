# Empty armory illustration

`empty-rack.png` is original project art generated with the built-in `image_gen` tool on 2026-11-06. No third-party game artwork or model was imported. It depicts an explicitly empty rack with bare hooks and shelf; it does not represent owned gear, a reward or a selectable item.

The two generation prompts, source paths, tool and absence of an exposed seed are recorded in `generation.json`. Source and runtime hashes, alpha trim and proportional packing are in `manifest.json`. This is AI-generated project artwork, not a third-party CC0 asset; no external license attribution is being asserted.

Only mechanical alpha-bounds trim, proportional Lanczos downsampling and centered transparent packing were performed after generation. The first output's translucent backdrop was corrected by a second image-generation edit, not an invented manual cutout. The runtime PNG is 384×256 with alpha and transparent gutter. PNG is retained because the existing engine enables PNG decoding; a WebP dependency or duplicated shipped bitmap is unnecessary for this small asset. Existing atlases and models are untouched.

The image is decorative and hidden until its image asset is available. The original empty-state text stays visible even if the file is missing, without a white placeholder or a gameplay loading gate.

Read-only checks: `uv run --with pillow python tools/verify_empty_armory_art.py`.
