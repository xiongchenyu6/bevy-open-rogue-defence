# Draft screen artwork

These images are local Blender renders of the project's authored CC0 models. They require no external image service or additional purchased assets. The original model attribution, pinned source URLs and licenses are recorded in [model sources](../models/SOURCES.md).

## Hero portraits

`portraits/hero_*.png` contains twelve transparent 384 × 480 PNGs. Each filename matches its source `assets/models/hero_*.glb`. Source bodies and equipment are by Kay Lousberg / KayKit, under [CC0](../licenses/KayKit-Adventurers-CC0.txt); the [cast manifest](../models/cast-manifest.json) identifies the original model and project tint/weapon variants. The portraits depict the existing five base bodies and twelve game variants, rather than twelve newly sculpted characters.

[The portrait renderer](../../tools/render_hero_portraits.py) selects frame 8 of each model's idle animation, uses an orthographic three-quarter camera, and adds warm key, cool fill and rim lighting. Cycles renders 32 samples with denoising and AgX color management. ImageMagick fits the alpha silhouette within 344 × 440 pixels on the transparent canvas, leaving a shared 20-pixel bottom margin.

From the repository root, with Blender and ImageMagick (`magick`) on `PATH`:

```sh
blender -b -t 4 --python tools/render_hero_portraits.py
```

To reproduce one portrait:

```sh
blender -b -t 4 --python tools/render_hero_portraits.py -- hero_storm_savant
```

The initial renders used Blender 5.2.2 LTS and ImageMagick 7.1.2-31. The twelve PNGs were checked for their expected dimensions and RGBA format, and reviewed together for silhouette framing, visible faces and complete equipment.

## Recruitment hall backdrop

`draft_backdrop.png` is a 1600 × 1000 render of the project's KayKit Dungeon Remastered environment models, including stone pillars, walls, floor tiles, rubble, blue banners and torches. The project adds scene layout, lighting, a foundation plane and atmospheric haze. Original models are by Kay Lousberg / KayKit under [CC0](../licenses/KayKit-Dungeon-Remastered-CC0.txt). The [environment source manifest](../licenses/environment-models.json) records upstream files and import transforms.

[The backdrop renderer](../../tools/render_draft_backdrop.py) recreates the composition and render settings:

```sh
blender -b -t 4 --python tools/render_draft_backdrop.py
```

Both scripts read the checked-in GLBs directly and write their corresponding PNGs into this directory. Rendering requires no source-model downloads. Regeneration overwrites the selected output images.

## Skill, bond and HUD icons

`icons/` contains 56 transparent 256 × 256 PNGs: all 30 skill IDs and ten bond IDs from `src/game/content.rs`, plus 16 HUD/action pictograms. [The manifest](icons/manifest.json) enumerates the required filenames by category. Content icons use their canonical definition ID; action icons use `gold`, `wood`, `skull`, `shield`, `energy`, `tower`, `skill`, `bond`, `market`, `dodge`, `ultimate`, `refresh`, `skip`, `later`, `confirm` and `abandon`.

[The icon renderer](../../tools/render_game_icons.py) constructs original beveled 3D silhouettes, weapons, elemental emblems and resource symbols in Blender, with metallic edges and a shared studio lighting setup. These procedural designs are project-created artwork. There are no embedded text labels or downloaded icon images. Six summon skill icons instead render the matching `assets/models/summon_*.glb` with the authored idle pose; their KayKit/Quaternius CC0 provenance is recorded in [model sources](../models/SOURCES.md) and the [cast manifest](../models/cast-manifest.json).

The renderer uses Cycles with 20 samples, denoising, AgX color management and transparent film. ImageMagick trims and fits each silhouette into 228 × 228 pixels with a safe transparent border. To regenerate the full icon set or a selected icon:

```sh
blender -b -t 4 --python tools/render_game_icons.py
blender -b -t 4 --python tools/render_game_icons.py -- arc_chain
```

The script checks that every current skill and bond has a defined icon before rendering and writes `icons/manifest.json`. Requirements are Blender and ImageMagick on `PATH`, matching the portrait workflow above.
