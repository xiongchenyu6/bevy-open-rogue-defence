# Authored model sources

All shipped third-party model packs listed here use CC0-1.0 (public domain). Credit is retained voluntarily. Character and summon runtime models are repacked versions; `cast-manifest.json` records their output models, source filename and SHA-256, selected animation clips, tint, scale and orientation. `tools/import_cast.py` preserves authored geometry, skinning and selected animation tracks, embeds textures, and removes unused animation data.

## Kay Lousberg / KayKit

| Pack | Pinned source | Files | License evidence |
| --- | --- | --- | --- |
| Adventurers | [KayKit-Game-Assets/KayKit-Character-Pack-Adventures-1.0@672074b73ba2](https://github.com/KayKit-Game-Assets/KayKit-Character-Pack-Adventures-1.0/tree/672074b73ba276876a19e8816ecdc5241817ab47/addons/kaykit_character_pack_adventures/Characters/gltf) | Barbarian, Knight, Mage, Rogue, Rogue_Hooded | [Upstream license](https://github.com/KayKit-Game-Assets/KayKit-Character-Pack-Adventures-1.0/blob/672074b73ba276876a19e8816ecdc5241817ab47/LICENSE.txt); [local copy](../licenses/KayKit-Adventurers-CC0.txt) |
| Skeletons | [KayKit-Game-Assets/KayKit-Character-Pack-Skeletons-1.0@15b62b9bad12](https://github.com/KayKit-Game-Assets/KayKit-Character-Pack-Skeletons-1.0/tree/15b62b9bad122f72926c10fb14d622c73819fa54/addons/kaykit_character_pack_skeletons/Characters/gltf) | Skeleton_Mage, Skeleton_Minion, Skeleton_Rogue, Skeleton_Warrior | [Upstream license](https://github.com/KayKit-Game-Assets/KayKit-Character-Pack-Skeletons-1.0/blob/15b62b9bad122f72926c10fb14d622c73819fa54/LICENSE.txt); [local copy](../licenses/KayKit-Skeletons-CC0.txt) |

## Quaternius

| Pack | Models available to importer | Official license evidence |
| --- | --- | --- |
| Ultimate Animated Animals | Wolf | [Official pack page: CC0](https://quaternius.com/packs/ultimateanimatedanimals.html) |
| Ultimate Monsters | AlienBlob, ArmabeeEvolved, BlueDemon, Demon, Dino, DragonEvolved, GhostSkull, Goleling, GolelingEvolved, GreenSpikyBlob, Mushnub, MushroomKing, Orc, Squidle, WizardBlob, Yeti; also demon and ghost | [Official pack page: CC0](https://quaternius.com/packs/ultimatemonsters.html) |
| Animated Monster Pack | Bat, Dragon, Slime | [Official pack page: CC0](https://quaternius.com/packs/animatedmonster.html) |

The author describes these packs as usable in personal and commercial projects. The license is [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/).

Download mirrors used:

- Ultimate Monsters: [Clockwork Coliseum bestiary](https://clockworkcoliseum.com/monsters), explicitly labeled “Quaternius CC0 creatures”; [public source manifest](https://clockworkcoliseum.com/models/quaternius/manifest.json). Exact URLs, hashes, original clip names and mesh bounds are retained in [creature-source-manifest.json](../licenses/creature-source-manifest.json). The self-rigged collection on that site is not used.
- Wolf: [benjaminpjones/catch-the-animal](https://github.com/benjaminpjones/catch-the-animal), an embedded-buffer glTF mirror of the Quaternius animal. Exact commit and hash are in the source manifest.
- Original Dragon and Slime alternatives: [cairnfall-public credits](https://github.com/euuuuuuan/cairnfall-public/blob/main/CREDITS.md), which identify Quaternius, the CC0 license and FBX-to-GLB conversion. These alternatives have already-renamed clips; the importer records the actual selected names.
- Bat, demon and ghost alternatives: [warptracker asset manifest](https://github.com/ilrein/warptracker/blob/main/ASSETS.md), which identifies the Quaternius source pack and CC0 license.

Character and summon models referenced by `cast-manifest.json` are shipped; environment assets use their separate manifests below. Source manifests also list evaluated candidates for reproducibility. `CreepCreature` was rejected because its author/license could not be established and is not imported.

## Environment props

Dungeon props are from [KayKit Dungeon Remastered](https://github.com/KayKit-Game-Assets/KayKit-Dungeon-Remastered-1.0). See [environment-models.json](../licenses/environment-models.json) for pinned URLs and per-model import transforms, and [the upstream license copy](../licenses/KayKit-Dungeon-Remastered-CC0.txt).

Provenance checked on 2026-09-28.

## Rebuilding and checking

```sh
python3 tools/import_environment_models.py
python3 tools/import_catapult_model.py
python3 tools/import_cast.py /tmp/rogue-art --download
python3 tools/verify_models.py
blender -b -t 3 --python tools/render_models.py -- assets/models screenshots/model-cast-review.png
```

Downloads are checked against the source SHA-256 manifests. The two wolf variants use the author's colored materials rather than an image texture. Twelve heroes share five authored base bodies, with selected weapons and tinted materials; they are not twelve separately sculpted characters. The mechanical turret retains the KayKit catapult geometry and adds recoil/shudder tracks. Runtime animations use gameplay cadence and virtual time, and freeze when paused or when pooled actors retire.

Game captures accept a final optional viewport size, for example `capture still screenshots/models-mobile.png 300 summons 0xA11CE 844x390`.

## Original adventure creatures

The three `adventure_*` GLBs are original Blender models generated by [build_adventure_models.py](../../tools/build_adventure_models.py). They use colored matte materials, real skinning and four sampled skeletal clips; no ordinary enemy or main-boss mesh is reused. Geometry parameters, author, CC0 license and output hashes are recorded in [adventure-manifest.json](adventure-manifest.json).

## Cavern entrance

`environment_demon_cavern.glb` derives from [LowPoly Cave Entrance](https://opengameart.org/content/lowpoly-cave-entrance), by OrbitStudios / Fernando Ferreira, submitted by zisongbr, CC0-1.0. The original474 triangles are retained with global transforms and a new1K UV/PBR bake. Source/output hashes and transforms are in [demon-cavern-source.json](../licenses/demon-cavern-source.json); [source credit and license note](../licenses/Demon-Cavern-Source.txt).

The stone texture is original project-generated material, not a third-party CC0 texture. Original mesh/license files, material generation provenance and the actual failed/passing clearance checks are retained in [the asset verification record](../../docs/verification/intact-cave-C-asset-2026-11-06/README.md). Actual controlled native entrance views are in [the native record](../../docs/verification/intact-cave-C-native-2026-11-06/README.md). These records do not substitute for normal-play acceptance of a new production camera.
