"""Validate shipped GLBs against runtime paths and the import manifest (stdlib only)."""

import hashlib
import json
import re
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODELS = ROOT / "assets/models"


def validate(path, animated):
    raw = path.read_bytes()
    magic, version, length = struct.unpack_from("<III", raw)
    assert magic == 0x46546C67 and version == 2 and length == len(raw), path
    size, kind = struct.unpack_from("<II", raw, 12)
    assert kind == 0x4E4F534A
    doc = json.loads(raw[20 : 20 + size])
    binary_size, kind = struct.unpack_from("<II", raw, 20 + size)
    assert kind == 0x004E4942 and 28 + size + binary_size == length
    assert len(doc["buffers"]) == 1 and "uri" not in doc["buffers"][0]
    assert binary_size >= doc["buffers"][0]["byteLength"]
    for view in doc["bufferViews"]:
        assert view["buffer"] == 0
        assert view.get("byteOffset", 0) + view["byteLength"] <= binary_size
    assert doc["meshes"] and doc.get("materials"), (
        f"{path}: missing authored mesh/materials"
    )
    assert doc.get("images") or path.name in {
        "enemy_burrow_stalker.glb",
        "summon_spirit_hound.glb",
    }, f"{path}: missing texture"
    # Quaternius Wolf intentionally uses four colored PBR materials without UVs.
    for image in doc.get("images", []):
        assert "uri" not in image and image["bufferView"] < len(doc["bufferViews"])
        assert doc["bufferViews"][image["bufferView"]]["byteLength"] > 64
    for mesh in doc["meshes"]:
        for primitive in mesh["primitives"]:
            assert {"POSITION", "NORMAL"} <= primitive["attributes"].keys()
            if doc.get("images"):
                assert "TEXCOORD_0" in primitive["attributes"]
            for index in primitive["attributes"].values():
                assert 0 <= index < len(doc["accessors"])
    if animated:
        assert [a["name"] for a in doc["animations"]] == [
            "Idle",
            "Walk",
            "Attack",
            "Hit",
        ]
        assert doc.get("skins") or path.name == "summon_bolt_turret.glb"
        for animation in doc["animations"]:
            assert animation["channels"] and animation["samplers"]
            for channel in animation["channels"]:
                assert channel["target"]["node"] < len(doc["nodes"])
            for sampler in animation["samplers"]:
                time = doc["accessors"][sampler["input"]]
                assert time["count"] >= 2 and time["max"][0] > 0
                assert doc["accessors"][sampler["output"]]["count"] >= time["count"]
    return {
        "file": path.name,
        "bytes": length,
        "mesh_count": len(doc["meshes"]),
        "skin_count": len(doc.get("skins", [])),
        "animations": [a["name"] for a in doc.get("animations", [])],
        "sha256": hashlib.sha256(raw).hexdigest(),
    }


def main():
    manifest = json.loads((MODELS / "cast-manifest.json").read_text())
    cast = {m["file"] for m in manifest}
    arena = (ROOT / "src/game/arena.rs").read_text()
    runtime_cast = set(
        re.findall(r'models/((?:hero|enemy|boss|summon)_[^"#]+\.glb)', arena)
    )
    assert cast == runtime_cast, (
        f"Cast manifest/runtime mismatch: {cast ^ runtime_cast}"
    )
    runtime_models = set(re.findall(
        r'"models/([^"#{}]+\.glb)',
        "\n".join(p.read_text() for p in (ROOT / "src").rglob("*.rs")),
    ))
    environment = (ROOT / "src/game/environment.rs").read_text()
    gate_models = re.search(
        r'for \(name, model, z, width, height\) in \[(.*?)\]',
        environment,
        re.S,
    )
    assert gate_models, "Missing gate model table"
    runtime_models.update(
        f"environment_{model}.glb"
        for model in re.findall(r'\("[^"]+", "([^"]+)"', gate_models.group(1))
    )
    for filename in runtime_models:
        assert (MODELS / filename).is_file(), f"Missing runtime model: {filename}"
    reports = [
        validate(path, path.name in cast) for path in sorted(MODELS.glob("*.glb"))
    ]
    report = {
        "models": len(reports),
        "animated_cast": len(cast),
        "bytes": sum(m["bytes"] for m in reports),
        "results": reports,
    }
    target = ROOT / "screenshots/model-verification.json"
    target.parent.mkdir(exist_ok=True)
    target.write_text(json.dumps(report, indent=2) + "\n")
    print(
        f"PASS: {len(reports)} self-contained authored GLBs; {len(cast)} cast models with Idle/Walk/Attack/Hit; {report['bytes']:,} bytes"
    )
    print(f"Report: {target}")


if __name__ == "__main__":
    main()
