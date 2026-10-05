# Environment artwork

The arena uses authored CC0 models by Kay Lousberg, downloaded from the official
[KayKit Dungeon Remastered repository](https://github.com/KayKit-Game-Assets/KayKit-Dungeon-Remastered-1.0)
at commit `b0ca9bd96a8072ab36a3a5464f00ed1e06a16d07`. The original license is
`KayKit-Dungeon-Remastered-CC0.txt`; source/output hashes and dimensions are in
`environment-models.json`.

`python tools/import_environment_models.py` recreates the environment GLBs.
The original meshes, UVs, and embedded gradient atlas are preserved. A scene
wrapper centers each prop horizontally, places its feet at Y=0, and sets its
height to 1 so the game can size each placement. The core is the pack's golden
sword-and-shield relic with a subtle turquoise emissive accent. Fortress walls,
banners, stone pillars, broken walls, rubble, lit torches, individual paving tiles
and low stone barriers replace procedural
solid props. Spell rings, translucent ground effects and magical motes remain
procedural effects.

The mechanical catapult comes from the official
[KayKit Medieval Hexagon Pack](https://github.com/KayKit-Game-Assets/KayKit-Medieval-Hexagon-Pack-1.0)
at commit `84fa4e91af6a88989be7c99e0891cede11f2ca38`.
`python tools/import_catapult_model.py` embeds its mesh buffer and atlas into a
single unit-height `environment_catapult.glb`. The turret and arm remain separate
nodes for animation. See `catapult-model.json` and
`KayKit-Medieval-Hexagon-CC0.txt` for provenance and the original CC0 license.
