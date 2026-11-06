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
        "adventure_embermaw.glb",
        "adventure_tidehorn.glb",
        "adventure_plaguejaw.glb",
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
            if path.name.startswith("adventure_"):
                assert {"JOINTS_0", "WEIGHTS_0"} <= primitive["attributes"].keys()
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
                if (path.name.startswith("adventure_")
                        and doc["nodes"][channel["target"]["node"]].get("name", "").startswith("leg")
                        and channel["target"]["path"] == "rotation"):
                    sampler = animation["samplers"][channel["sampler"]]
                    assert doc["accessors"][sampler["input"]]["count"] >= 10
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


def _delimiter_end(code, start, opening, closing):
    depth = 1
    index = start + 1
    while index < len(code):
        depth += (code[index] == opening) - (code[index] == closing)
        index += 1
        if depth == 0:
            return index
    raise ValueError(f"Unclosed Rust {opening} at character {start}")


def production_rust_source(source):
    """Keep production literals; exclude comments and inline cfg(test) modules.

    A same-length lexical mask locates attributes and balanced braces without
    interpreting braces in comments, characters, escaped or raw string literals.
    """
    code = list(source)
    strings = []
    index = 0
    raw_start = re.compile(r'(?:br|cr|r)(#*)"')
    char_literal = re.compile(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'")

    def blank(start, end):
        code[start:end] = ["\n" if c == "\n" else " " for c in source[start:end]]

    while index < len(source):
        start = index
        if source.startswith("//", index):
            newline = source.find("\n", index + 2)
            index = len(source) if newline < 0 else newline
        elif source.startswith("/*", index):
            depth = 1
            index += 2
            while index < len(source) and depth:
                if source.startswith("/*", index):
                    depth += 1
                    index += 2
                elif source.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            if depth:
                raise ValueError("Unclosed Rust block comment")
        elif (raw := raw_start.match(source, index)) and (
            index == 0 or not (source[index - 1].isalnum() or source[index - 1] == "_")
        ):
            terminator = '"' + raw.group(1)
            end = source.find(terminator, raw.end())
            if end < 0:
                raise ValueError("Unclosed Rust raw string")
            index = end + len(terminator)
            strings.append((start, index))
        elif source[index] == '"':
            index += 1
            while index < len(source):
                if source[index] == "\\":
                    index += 2
                elif source[index] == '"':
                    index += 1
                    break
                else:
                    index += 1
            else:
                raise ValueError("Unclosed Rust string")
            strings.append((start, index))
        elif char := char_literal.match(source, index):
            index = char.end()
        else:
            index += 1
            continue
        blank(start, index)

    masked = "".join(code)
    test_modules = []
    for attribute in re.finditer(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", masked):
        cursor = attribute.end()
        while True:
            cursor += len(masked[cursor:]) - len(masked[cursor:].lstrip())
            extra = re.match(r"#\s*\[", masked[cursor:])
            if not extra:
                break
            cursor = _delimiter_end(masked, cursor + extra.end() - 1, "[", "]")
        module = re.match(
            r"(?:pub\b(?:\s*\([^)]*\))?\s*)?mod\s+\w+\s*\{",
            masked[cursor:],
        )
        if module:
            brace = cursor + module.end() - 1
            test_modules.append((attribute.start(), _delimiter_end(masked, brace, "{", "}")))
    for start, end in strings:
        if not any(left <= start < right for left, right in test_modules):
            code[start:end] = source[start:end]
    for start, end in test_modules:
        blank(start, end)
    return "".join(code)


def runtime_model_names(source):
    return set(re.findall(r'"models/([^"#{}]+\.glb)', production_rust_source(source)))


def check_runtime_model_files(sources, models, extra=()):
    names = set(extra)
    for source in sources:
        names.update(runtime_model_names(source.read_text()))
    for filename in sorted(names):
        assert (models / filename).is_file(), f"Missing runtime model: {filename}"
    return names


def main():
    manifest = json.loads((MODELS / "cast-manifest.json").read_text())
    cast = {m["file"] for m in manifest}
    adventures = {
        m["file"]
        for m in json.loads((MODELS / "adventure-manifest.json").read_text())["assets"]
    }
    arena = production_rust_source((ROOT / "src/game/arena.rs").read_text())
    for asset in json.loads((MODELS / "adventure-manifest.json").read_text())["assets"]:
        assert hashlib.sha256((MODELS / asset["file"]).read_bytes()).hexdigest() == asset["sha256"]
    runtime_cast = set(
        re.findall(r'models/((?:hero|enemy|boss|summon)_[^"#]+\.glb)', arena)
    )
    assert cast == runtime_cast, (
        f"Cast manifest/runtime mismatch: {cast ^ runtime_cast}"
    )
    runtime_adventures = set(re.findall(r'models/(adventure_[^"#]+\.glb)', arena))
    assert adventures == runtime_adventures
    # src/bin holds capture/QA binaries; they probe deliberately missing models.
    runtime_sources = [
        p for p in (ROOT / "src").rglob("*.rs") if ROOT / "src/bin" not in p.parents
    ]
    environment = production_rust_source((ROOT / "src/game/environment.rs").read_text())
    gate_models = re.search(
        r'for \(name, model, z, width, height\) in \[(.*?)\]',
        environment,
        re.S,
    )
    assert gate_models, "Missing gate model table"
    check_runtime_model_files(
        runtime_sources,
        MODELS,
        (f"environment_{model}.glb"
         for model in re.findall(r'\("[^"]+", "([^"]+)"', gate_models.group(1))),
    )
    reports = [
        validate(path, path.name in cast | adventures) for path in sorted(MODELS.glob("*.glb"))
    ]
    report = {
        "models": len(reports),
        "animated_cast": len(cast),
        "animated_adventures": len(adventures),
        "bytes": sum(m["bytes"] for m in reports),
        "results": reports,
    }
    target = ROOT / "screenshots/model-verification.json"
    target.parent.mkdir(exist_ok=True)
    target.write_text(json.dumps(report, indent=2) + "\n")
    print(
        f"PASS: {len(reports)} self-contained authored GLBs; {len(cast)} cast + {len(adventures)} original adventure models with Idle/Walk/Attack/Hit; {report['bytes']:,} bytes"
    )
    print(f"Report: {target}")


if __name__ == "__main__":
    main()
