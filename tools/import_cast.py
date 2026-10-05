"""Import the authored cast without flattening skinning or baking away animation.

python3 tools/import_cast.py /tmp/rogue-art
Sources and selected clips are recorded in assets/models/cast-manifest.json.
Only the four runtime clips and their referenced binary data are shipped.
"""

import base64
import copy
import hashlib
import json
import math
import struct
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets/models"


def read_gltf(path):
    raw = path.read_bytes()
    if raw[:4] == b"glTF":
        length = struct.unpack_from("<I", raw, 12)[0]
        data = json.loads(raw[20 : 20 + length])
        offset = 20 + length
        binary = raw[offset + 8 : offset + 8 + struct.unpack_from("<I", raw, offset)[0]]
        buffers = [binary]
    else:
        data = json.loads(raw)
        buffers = []
        for buffer in data["buffers"]:
            uri = buffer["uri"]
            buffers.append(
                base64.b64decode(uri.split(",", 1)[1])
                if uri.startswith("data:")
                else (path.parent / uri).read_bytes()
            )
    # Embed image URIs as buffer views so each shipped model is self-contained.
    for image in data.get("images", []):
        if "uri" not in image:
            continue
        uri = image.pop("uri")
        blob = (
            base64.b64decode(uri.split(",", 1)[1])
            if uri.startswith("data:")
            else (path.parent / uri).read_bytes()
        )
        image["mimeType"] = "image/png" if blob.startswith(b"\x89PNG") else "image/jpeg"
        image["bufferView"] = len(data["bufferViews"])
        data["bufferViews"].append({"buffer": len(buffers), "byteLength": len(blob)})
        buffers.append(blob)
    return data, buffers


def write_gltf(path, data, buffers):
    # Drop unreachable mesh data (weapon alternatives) and animation accessors.
    mesh_ids = sorted({n["mesh"] for n in data["nodes"] if "mesh" in n})
    mesh_map = {old: new for new, old in enumerate(mesh_ids)}
    data["meshes"] = [data["meshes"][i] for i in mesh_ids]
    for node in data["nodes"]:
        if "mesh" in node:
            node["mesh"] = mesh_map[node["mesh"]]
    refs = []
    for mesh in data["meshes"]:
        for primitive in mesh["primitives"]:
            refs.extend((primitive["attributes"], k) for k in primitive["attributes"])
            if "indices" in primitive:
                refs.append((primitive, "indices"))
            for target in primitive.get("targets", []):
                refs.extend((target, k) for k in target)
    for skin in data.get("skins", []):
        if "inverseBindMatrices" in skin:
            refs.append((skin, "inverseBindMatrices"))
    for animation in data.get("animations", []):
        for sampler in animation["samplers"]:
            refs.extend([(sampler, "input"), (sampler, "output")])
    accessor_ids = sorted({obj[key] for obj, key in refs})
    accessor_map = {old: new for new, old in enumerate(accessor_ids)}
    for obj, key in refs:
        obj[key] = accessor_map[obj[key]]
    data["accessors"] = [data["accessors"][i] for i in accessor_ids]
    view_refs = [(a, "bufferView") for a in data["accessors"] if "bufferView" in a]
    for a in data["accessors"]:
        if "sparse" in a:
            view_refs.extend(
                (a["sparse"][k], "bufferView") for k in ["indices", "values"]
            )
    view_refs.extend((i, "bufferView") for i in data.get("images", []))
    view_ids = sorted({obj[key] for obj, key in view_refs})
    view_map = {old: new for new, old in enumerate(view_ids)}
    binary = bytearray()
    views = []
    for i in view_ids:
        view = copy.deepcopy(data["bufferViews"][i])
        start = view.get("byteOffset", 0)
        blob = buffers[view["buffer"]][start : start + view["byteLength"]]
        binary.extend(b"\0" * (-len(binary) % 4))
        view["byteOffset"] = len(binary)
        view["buffer"] = 0
        binary.extend(blob)
        views.append(view)
    for obj, key in view_refs:
        obj[key] = view_map[obj[key]]
    data["bufferViews"] = views
    data["buffers"] = [{"byteLength": len(binary)}]
    binary.extend(b"\0" * (-len(binary) % 4))
    js = json.dumps(data, separators=(",", ":")).encode()
    js += b" " * (-len(js) % 4)
    path.write_bytes(
        struct.pack("<III", 0x46546C67, 2, 28 + len(js) + len(binary))
        + struct.pack("<II", len(js), 0x4E4F534A)
        + js
        + struct.pack("<II", len(binary), 0x004E4942)
        + binary
    )


# Role, source, chosen equipment, tint, body scale. Equipment alternatives are
# removed, not just hidden: glTF has no portable per-node visibility property.
HEROES = [
    ("storm_savant", "Mage", ["2H_Staff"], [0.50, 0.95, 1.0]),
    ("ember_ranger", "Rogue_Hooded", ["2H_Crossbow"], [1.0, 0.65, 0.36]),
    ("stone_warden", "Knight", ["1H_Sword", "Rectangle_Shield"], [0.65, 0.87, 0.78]),
    ("spirit_keeper", "Mage", ["1H_Wand", "Spellbook"], [0.70, 1.0, 0.64]),
    ("tide_walker", "Rogue", ["Knife", "Knife_Offhand"], [0.46, 0.86, 1.0]),
    ("star_wagerer", "Mage", ["1H_Wand", "Spellbook_open"], [1.0, 0.78, 0.48]),
    ("frost_oracle", "Mage", ["2H_Staff", "Spellbook"], [0.72, 0.91, 1.0]),
    ("iron_berserker", "Barbarian", ["2H_Axe"], [1.0, 0.65, 0.55]),
    ("rift_dancer", "Rogue_Hooded", ["Knife", "Knife_Offhand"], [0.88, 0.48, 1.0]),
    ("grove_artificer", "Rogue", ["1H_Crossbow", "Knife_Offhand"], [0.63, 1.0, 0.55]),
    ("solar_vanguard", "Knight", ["1H_Sword", "Badge_Shield"], [1.0, 0.85, 0.40]),
    ("void_reaper", "Barbarian", ["2H_Axe"], [0.65, 0.48, 0.90]),
]
EQUIPMENT = {
    "Spellbook",
    "Spellbook_open",
    "1H_Wand",
    "2H_Staff",
    "1H_Sword_Offhand",
    "Badge_Shield",
    "Rectangle_Shield",
    "Round_Shield",
    "Spike_Shield",
    "1H_Sword",
    "2H_Sword",
    "1H_Axe_Offhand",
    "Barbarian_Round_Shield",
    "1H_Axe",
    "2H_Axe",
    "Mug",
    "Knife_Offhand",
    "1H_Crossbow",
    "2H_Crossbow",
    "Knife",
    "Throwable",
}


def import_model(
    source, output, clips, tint=(1, 1, 1), equipment=(), scale=0.85, rotate=True
):
    data, buffers = read_gltf(source)
    available = {a["name"]: a for a in data["animations"]}
    data["animations"] = [copy.deepcopy(available[name]) for name in clips]
    for animation, name in zip(data["animations"], ["Idle", "Walk", "Attack", "Hit"]):
        animation["name"] = name
    for node in data["nodes"]:
        if node.get("name") in EQUIPMENT and node["name"] not in equipment:
            node.pop("mesh", None)
            node.pop("skin", None)
    for material in data.get("materials", []):
        pbr = material.setdefault("pbrMetallicRoughness", {})
        color = pbr.get("baseColorFactor", [1, 1, 1, 1])
        pbr["baseColorFactor"] = [color[i] * tint[i] for i in range(3)] + [color[3]]
        pbr["metallicFactor"] = min(pbr.get("metallicFactor", 0), 0.25)
        pbr["roughnessFactor"] = 0.72
    scene = data["scenes"][data.get("scene", 0)]
    root = {"name": "CastScale", "children": scene["nodes"], "scale": [scale] * 3}
    if rotate:
        root["rotation"] = [0, 1, 0, 0]
    scene["nodes"] = [len(data["nodes"])]
    data["nodes"].append(root)
    data.setdefault("asset", {})["copyright"] = (
        "CC0 — Kay Lousberg / Quaternius; see cast-manifest.json and SOURCES.md"
    )
    write_gltf(OUT / (output + ".glb"), data, buffers)
    return {
        "file": output + ".glb",
        "source": source.name,
        "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
        "clips": dict(zip(["Idle", "Walk", "Attack", "Hit"], clips)),
        "tint": tint,
        "scale": scale,
        "forward_rotation_y_pi": rotate,
        "bytes": (OUT / (output + ".glb")).stat().st_size,
    }


CREATURES = [
    # Output role, source model, target authoring scale, tint.
    ("enemy_rift_sapper", "Skeleton_Rogue.glb", 0.60, (0.85, 0.56, 0.92)),
    ("enemy_gnawling", "Dino.gltf", 0.42, (1, 0.85, 0.75)),
    ("enemy_shell_guard", "ArmabeeEvolved.gltf", 0.52, (0.90, 0.88, 0.75)),
    ("enemy_void_caster", "WizardBlob.gltf", 0.48, (0.83, 0.58, 1)),
    ("enemy_core_rusher", "BlueDemon.gltf", 0.55, (1, 0.64, 0.50)),
    ("enemy_chanting_acolyte", "Skeleton_Mage.glb", 0.60, (0.73, 0.52, 1)),
    ("enemy_iron_colossus", "GolelingEvolved.gltf", 0.62, (0.72, 0.80, 0.87)),
    ("enemy_burrow_stalker", "Wolf.gltf", 0.45, (0.72, 0.58, 0.73)),
    ("enemy_field_mender", "Mushnub.gltf", 0.48, (0.60, 1, 0.67)),
    ("enemy_split_spore", "GreenSpikyBlob.gltf", 0.40, (0.93, 0.78, 1)),
    ("enemy_null_eye", "AlienBlob.gltf", 0.48, (0.76, 0.54, 1)),
    ("enemy_siege_lobber", "Orc.gltf", 0.58, (0.88, 0.73, 0.50)),
    ("boss_cinder_colossus", "Demon.gltf", 1.10, (1, 0.85, 0.65)),
    ("boss_abyssal_leviathan", "Squidle.gltf", 1.05, (0.52, 0.82, 1)),
    ("boss_dread_veil", "GhostSkull.gltf", 1.0, (0.85, 0.65, 1)),
    ("boss_sun_crown_magus", "WizardBlob.gltf", 1.0, (1, 0.85, 0.45)),
    ("boss_green_regent", "MushroomKing.gltf", 0.96, (0.62, 1, 0.67)),
    ("summon_bone_sentry", "Skeleton_Warrior.glb", 0.85, (0.63, 0.95, 1)),
    ("summon_spirit_hound", "Wolf.gltf", 0.65, (0.55, 1, 0.86)),
    ("summon_tide_elemental", "GreenSpikyBlob.gltf", 0.58, (0.40, 0.82, 1)),
    ("summon_frost_wyrm", "DragonEvolved.gltf", 0.85, (0.58, 0.87, 1)),
    ("summon_ancestor_colossus", "Yeti.gltf", 0.92, (0.65, 0.92, 0.82)),
]


def creature_clips(path):
    data, _ = read_gltf(path)
    names = {a["name"] for a in data["animations"]}
    if path.stem == "Wolf":
        return ["Idle", "Walk", "Attack", "Idle_HitReact1"]
    if path.stem.startswith("Skeleton"):
        return ["Idle", "Walking_A", "Spellcast_Shoot", "Hit_A"]
    if path.stem == "Demon":
        return ["Idle", "Walk", "Weapon", "HitReact"]
    if "Flying_Idle" in names:
        return ["Flying_Idle", "Fast_Flying", "Headbutt", "HitReact"]
    if "Bite_Front" in names:
        return ["Idle", "Walk", "Bite_Front", "HitRecieve"]
    return ["Idle", "Walk", "Punch", "HitReact"]


def import_catapult():
    source = OUT / "environment_catapult.glb"
    data, buffers = read_gltf(source)
    data["animations"] = []

    def accessor(values, shape):
        flat = [v for row in values for v in row] if shape == "VEC4" else values
        blob = struct.pack("<" + "f" * len(flat), *flat)
        view = len(data["bufferViews"])
        data["bufferViews"].append({"buffer": len(buffers), "byteLength": len(blob)})
        buffers.append(blob)
        index = len(data["accessors"])
        value = {
            "bufferView": view,
            "componentType": 5126,
            "count": len(values),
            "type": shape,
        }
        if shape == "SCALAR":
            value.update(min=[min(values)], max=[max(values)])
        data["accessors"].append(value)
        return index

    arm = next(
        i for i, n in enumerate(data["nodes"]) if n.get("name") == "catapult_arm_blue"
    )
    for name, times, angles in [
        ("Idle", [0, 1], [0, 0]),
        ("Walk", [0, 1], [0, 0]),
        ("Attack", [0, 0.12, 0.2, 0.45], [0, -0.8, 0.32, 0]),
        ("Hit", [0, 0.06, 0.14, 0.24], [0, 0.12, -0.08, 0]),
    ]:
        rotations = [[math.sin(a / 2), 0, 0, math.cos(a / 2)] for a in angles]
        data["animations"].append(
            {
                "name": name,
                "samplers": [
                    {
                        "input": accessor(times, "SCALAR"),
                        "output": accessor(rotations, "VEC4"),
                        "interpolation": "LINEAR",
                    }
                ],
                "channels": [
                    {"sampler": 0, "target": {"node": arm, "path": "rotation"}}
                ],
            }
        )
    # Existing model is unit-height. The gameplay summon scales this to 0.82.
    data["nodes"].append(
        {
            "name": "TurretScale",
            "children": data["scenes"][0]["nodes"],
            "scale": [2.15] * 3,
            "rotation": [0, 1, 0, 0],
        }
    )
    data["scenes"][0]["nodes"] = [len(data["nodes"]) - 1]
    output = OUT / "summon_bolt_turret.glb"
    write_gltf(output, data, buffers)
    return {
        "file": output.name,
        "source": source.name,
        "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
        "clips": {
            "Idle": "mechanical rest",
            "Walk": "mechanical rest",
            "Attack": "arm recoil",
            "Hit": "arm shudder",
        },
        "bytes": output.stat().st_size,
    }


def main():
    directory = Path(sys.argv[1])
    if "--download" in sys.argv:
        used_creatures = {
            item[1] for item in CREATURES if not item[1].startswith("Skeleton")
        }
        for filename, folder in [
            ("cast-source-manifest.json", "sources"),
            ("creature-source-manifest.json", "creatures"),
        ]:
            for record in json.loads((ROOT / "assets/licenses" / filename).read_text()):
                if folder == "creatures" and record["file"] not in used_creatures:
                    continue
                target = directory / folder / record["file"]
                target.parent.mkdir(parents=True, exist_ok=True)
                expected = record["sha256"]
                raw = (
                    target.read_bytes()
                    if target.exists()
                    else urllib.request.urlopen(record["url"], timeout=60).read()
                )
                assert hashlib.sha256(raw).hexdigest() == expected, (
                    f"Source hash mismatch: {target}"
                )
                target.write_bytes(raw)
    manifest = []
    for name, source, equipment, tint in HEROES:
        if source == "Mage":
            attack = "Spellcast_Shoot"
        elif "Crossbow" in " ".join(equipment):
            attack = (
                "2H_Ranged_Shoot" if "2H_Crossbow" in equipment else "1H_Ranged_Shoot"
            )
        elif source == "Barbarian":
            attack = "2H_Melee_Attack_Chop"
        elif "Knife" in equipment:
            attack = "Dualwield_Melee_Attack_Slice"
        else:
            attack = "1H_Melee_Attack_Chop"
        manifest.append(
            import_model(
                directory / "sources" / (source + ".glb"),
                "hero_" + name,
                ["Idle", "Walking_A", attack, "Hit_A"],
                tint,
                equipment,
            )
        )
    for output, source, scale, tint in CREATURES:
        path = (
            directory
            / ("sources" if source.startswith("Skeleton") else "creatures")
            / source
        )
        manifest.append(
            import_model(path, output, creature_clips(path), tint, scale=scale)
        )
    manifest.append(import_catapult())
    (OUT / "cast-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(
        f"Imported {len(manifest)} authored animated models: {sum(x['bytes'] for x in manifest):,} bytes"
    )


if __name__ == "__main__":
    main()
