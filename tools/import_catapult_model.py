#!/usr/bin/env python3
"""Embed the CC0 KayKit catapult textures and center/normalize the authored model."""

import hashlib
import json
import struct
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
COMMIT = "84fa4e91af6a88989be7c99e0891cede11f2ca38"
BASE = f"https://raw.githubusercontent.com/KayKit-Game-Assets/KayKit-Medieval-Hexagon-Pack-1.0/{COMMIT}/"
SOURCE = "addons/kaykit_medieval_hexagon_pack/Assets/gltf/buildings/blue/"


def main():
    def fetch(path):
        return urllib.request.urlopen(BASE + path, timeout=60).read()

    raw = fetch(SOURCE + "building_tower_catapult_blue.gltf")
    doc = json.loads(raw)
    data = bytearray(fetch(SOURCE + doc["buffers"][0].pop("uri")))
    for image in doc["images"]:
        uri = image.pop("uri")
        texture = fetch(SOURCE + uri)
        data.extend(b"\0" * (-len(data) % 4))
        image["bufferView"] = len(doc["bufferViews"])
        doc["bufferViews"].append(
            {"buffer": 0, "byteOffset": len(data), "byteLength": len(texture)}
        )
        data.extend(texture)
    low = [float("inf")] * 3
    high = [float("-inf")] * 3

    def bounds(index, translation):
        node = doc["nodes"][index]
        assert not any(k in node for k in ["matrix", "scale", "rotation"])
        offset = [
            translation[a] + node.get("translation", [0, 0, 0])[a] for a in range(3)
        ]
        if "mesh" in node:
            for primitive in doc["meshes"][node["mesh"]]["primitives"]:
                accessor = doc["accessors"][primitive["attributes"]["POSITION"]]
                for axis in range(3):
                    low[axis] = min(low[axis], accessor["min"][axis] + offset[axis])
                    high[axis] = max(high[axis], accessor["max"][axis] + offset[axis])
        for child in node.get("children", []):
            bounds(child, offset)

    scene = doc["scenes"][doc.get("scene", 0)]
    for node in scene["nodes"]:
        bounds(node, [0, 0, 0])
    height = high[1] - low[1]
    center = [(low[0] + high[0]) / 2, low[1], (low[2] + high[2]) / 2]
    doc["nodes"].append(
        {
            "name": "FootCenteredUnitHeight",
            "children": scene["nodes"],
            "scale": [1 / height] * 3,
            "translation": [-c / height for c in center],
        }
    )
    scene["nodes"] = [len(doc["nodes"]) - 1]
    data.extend(b"\0" * (-len(data) % 4))
    doc["buffers"][0]["byteLength"] = len(data)
    encoded = json.dumps(doc, separators=(",", ":")).encode()
    encoded += b" " * (-len(encoded) % 4)
    glb = (
        struct.pack("<4sII", b"glTF", 2, 28 + len(encoded) + len(data))
        + struct.pack("<I4s", len(encoded), b"JSON")
        + encoded
        + struct.pack("<I4s", len(data), b"BIN\0")
        + data
    )
    (ROOT / "assets/models/environment_catapult.glb").write_bytes(glb)
    licenses = ROOT / "assets/licenses"
    (licenses / "KayKit-Medieval-Hexagon-CC0.txt").write_bytes(fetch("LICENSE.txt"))
    (licenses / "catapult-model.json").write_text(
        json.dumps(
            {
                "source": BASE + SOURCE + "building_tower_catapult_blue.gltf",
                "commit": COMMIT,
                "source_sha256": hashlib.sha256(raw).hexdigest(),
                "output_sha256": hashlib.sha256(glb).hexdigest(),
                "normalization": "Y-up unit-height; horizontal center and lowest Y at origin",
                "arm_node": 0,
                "turret_node": 1,
                "original_height": height,
            },
            indent=2,
        )
        + "\n"
    )
    print(
        f"environment_catapult.glb: {len(glb)} bytes; arm node 0; turret node 1; original height {height:.4f}"
    )


if __name__ == "__main__":
    main()
