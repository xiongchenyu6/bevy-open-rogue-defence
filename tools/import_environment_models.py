#!/usr/bin/env python3
"""Import pinned CC0 KayKit dungeon props as self-contained, foot-centered GLBs."""

import hashlib
import json
import struct
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
COMMIT = "b0ca9bd96a8072ab36a3a5464f00ed1e06a16d07"
BASE = f"https://raw.githubusercontent.com/KayKit-Game-Assets/KayKit-Dungeon-Remastered-1.0/{COMMIT}/"
MODELS = [
    "wall_window_closed",
    "wall",
    "wall_gated",
    "pillar_decorated",
    "pillar",
    "rubble_large",
    "rubble_half",
    "torch_lit",
    "banner_blue",
    "wall_broken",
    "sword_shield_gold",
    "floor_tile_small",
    "barrier",
]


def main():
    output = ROOT / "assets/models"
    records = []
    for name in MODELS:
        source = f"addons/kaykit_dungeon_remastered/Assets/gltf/{name}.gltf.glb"
        raw = urllib.request.urlopen(BASE + source, timeout=60).read()
        assert raw[:4] == b"glTF"
        json_length = struct.unpack_from("<I", raw, 12)[0]
        doc = json.loads(raw[20 : 20 + json_length])
        binary = raw[20 + json_length + 8 :]
        assert all("uri" not in item for item in doc["buffers"] + doc.get("images", []))
        # These static source meshes have no node transforms. Wrap the scene in a
        # normalization node; original geometry, UVs, and embedded atlas are retained.
        assert all(
            not any(k in node for k in ["matrix", "scale", "rotation", "translation"])
            for node in doc["nodes"]
        )
        accessors = [
            doc["accessors"][p["attributes"]["POSITION"]]
            for m in doc["meshes"]
            for p in m["primitives"]
        ]
        low = [min(a["min"][axis] for a in accessors) for axis in range(3)]
        high = [max(a["max"][axis] for a in accessors) for axis in range(3)]
        height = high[1] - low[1]
        scale = 1 / height
        center = [(low[0] + high[0]) / 2, low[1], (low[2] + high[2]) / 2]
        scene = doc["scenes"][doc.get("scene", 0)]
        node = {
            "name": "FootCenteredUnitHeight",
            "children": scene["nodes"],
            "scale": [scale] * 3,
            "translation": [-x * scale for x in center],
        }
        scene["nodes"] = [len(doc["nodes"])]
        doc["nodes"].append(node)
        if name == "sword_shield_gold":
            for material in doc["materials"]:
                material["emissiveFactor"] = [0.06, 0.22, 0.15]
        encoded = json.dumps(doc, separators=(",", ":")).encode()
        encoded += b" " * (-len(encoded) % 4)
        binary += b"\0" * (-len(binary) % 4)
        result = (
            struct.pack("<4sII", b"glTF", 2, 28 + len(encoded) + len(binary))
            + struct.pack("<I4s", len(encoded), b"JSON")
            + encoded
            + struct.pack("<I4s", len(binary), b"BIN\0")
            + binary
        )
        filename = {
            "sword_shield_gold": "environment_core.glb",
            "floor_tile_small": "environment_floor.glb",
            "barrier": "environment_border.glb",
        }.get(name, f"environment_{name}.glb")
        target = output / filename
        target.write_bytes(result)
        records.append(
            {
                "file": target.name,
                "source": source,
                "source_sha256": hashlib.sha256(raw).hexdigest(),
                "sha256": hashlib.sha256(result).hexdigest(),
                "source_dimensions": [round(high[a] - low[a], 4) for a in range(3)],
                "normalized_dimensions": [
                    round((high[a] - low[a]) / height, 5) for a in range(3)
                ],
                "vertices": sum(a["count"] for a in accessors),
            }
        )
        print(
            f"{target.name}: {len(result)} bytes, dimensions {records[-1]['source_dimensions']}"
        )
    licenses = ROOT / "assets/licenses"
    licenses.mkdir(exist_ok=True)
    (licenses / "KayKit-Dungeon-Remastered-CC0.txt").write_bytes(
        urllib.request.urlopen(BASE + "LICENSE.txt", timeout=60).read()
    )
    (licenses / "environment-models.json").write_text(
        json.dumps(
            {
                "repository": BASE,
                "commit": COMMIT,
                "normalization": "Y-up, height 1, horizontal center at origin, lowest vertex at Y=0; original mesh and embedded PNG atlas preserved",
                "models": records,
            },
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    main()
