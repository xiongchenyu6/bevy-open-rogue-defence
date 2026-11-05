# Black Market equipment art

`equipment-atlas.png` is original AI-generated inventory art created for Rogue Commander on 2026-11-04 with the built-in image generation tool. It contains 24 item illustrations on an alpha background, 1536×1024 pixels, arranged in six columns by four rows. No third-party game art was imported.

SHA-256: `ea9c22a92ce0d3c251e928e04464d76b3a422fa8a677d02dcd80f933774e2160`.

Row-major order follows the explicit `MarketItem` catalog: Gate Patch, Skill Tuner, Bond Compass, Slow Hourglass, Revival Seed, Serrated Disc; Arc Lens, Hardened Plate, Spirit Drum, Wind Boots, Echo Core, Ember Heart; Storm Coil, Frost Mirror, Threefold Relic, Blood Contract, Market Badge, Guardian Seed; Ascendant Crown, Chaos Die, World Anchor, Legion Standard, Prismatic Engine, Last Oath.

Generation brief: a consistent hand-painted fantasy inventory set, textured worn metals and leather, crisp recognizable silhouettes, restrained magical jewels, no text or frames, transparent gaps. Native inspection revealed that some silhouettes crossed nominal 256px boundaries. The image remains unmodified; `manifest.json` records explicit subject rectangles with one pixel of padding, measured from the dominant alpha silhouettes. Rendering uses those rectangles and preserves each subject's aspect ratio, rather than cutting an equal grid and including neighboring fragments. The read-only validator checks those regions against the source image. Integration still requires native rendered UI screenshots; generation alone is not acceptance.
