#!/usr/bin/env python3
"""Measure where a hero's weapon is during its authored Attack clip.

Evaluates the glTF node hierarchy at sampled clip phases and prints the weapon
node origin and the weapon mesh's most forward bounds face centre ("muzzle") in the
model's root space (Y up, -Z forward, already including CastScale). The release
constants in src/game/combat/ranged_release.rs are taken from this output.

Usage: measure_weapon_socket.py assets/models/hero_ember_ranger.glb 2H_Crossbow
"""
import json
import math
import struct
import sys

COMPONENTS = {"SCALAR": 1, "VEC3": 3, "VEC4": 4}


def load(path):
    data = open(path, "rb").read()
    length, _ = struct.unpack_from("<II", data, 12)
    document = json.loads(data[20 : 20 + length])
    binary = data[20 + length + 8 :]
    return document, binary


def accessor(document, binary, index):
    item = document["accessors"][index]
    view = document["bufferViews"][item["bufferView"]]
    assert item["componentType"] == 5126, "only float accessors are animated"
    width = COMPONENTS[item["type"]]
    start = view.get("byteOffset", 0) + item.get("byteOffset", 0)
    stride = view.get("byteStride", 4 * width)
    return [
        struct.unpack_from("<" + "f" * width, binary, start + row * stride)
        for row in range(item["count"])
    ]


def quat_matrix(q):
    x, y, z, w = q
    return [
        [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
        [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
        [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
    ]


def compose(translation, rotation, scale):
    r = quat_matrix(rotation)
    return [
        [r[i][0] * scale[0], r[i][1] * scale[1], r[i][2] * scale[2], translation[i]]
        for i in range(3)
    ] + [[0.0, 0.0, 0.0, 1.0]]


def multiply(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def point(matrix, p):
    return tuple(sum(matrix[i][k] * (*p, 1.0)[k] for k in range(4)) for i in range(3))


def lerp(a, b, t):
    return tuple(x + (y - x) * t for x, y in zip(a, b))


def slerp(a, b, t):
    dot = sum(x * y for x, y in zip(a, b))
    if dot < 0:
        b, dot = tuple(-y for y in b), -dot
    if dot > 0.9995:
        q = lerp(a, b, t)
    else:
        angle = math.acos(dot)
        q = tuple(
            (math.sin((1 - t) * angle) * x + math.sin(t * angle) * y) / math.sin(angle)
            for x, y in zip(a, b)
        )
    norm = math.sqrt(sum(x * x for x in q))
    return tuple(x / norm for x in q)


def sample(times, values, t, rotation):
    if t <= times[0][0]:
        return values[0]
    if t >= times[-1][0]:
        return values[-1]
    for i in range(len(times) - 1):
        t0, t1 = times[i][0], times[i + 1][0]
        if t0 <= t <= t1:
            f = (t - t0) / (t1 - t0) if t1 > t0 else 0.0
            return (slerp if rotation else lerp)(values[i], values[i + 1], f)
    raise AssertionError("unreachable")


def main():
    path, weapon = sys.argv[1], sys.argv[2]
    clip_name = sys.argv[3] if len(sys.argv) > 3 else "Attack"
    steps = int(sys.argv[4]) if len(sys.argv) > 4 else 20
    document, binary = load(path)
    nodes = document["nodes"]
    parent = {child: i for i, node in enumerate(nodes) for child in node.get("children", [])}
    target = next(i for i, node in enumerate(nodes) if node.get("name") == weapon)
    clip = next(a for a in document["animations"] if a.get("name") == clip_name)
    tracks = {}
    duration = 0.0
    for channel in clip["channels"]:
        sampler = clip["samplers"][channel["sampler"]]
        assert sampler.get("interpolation", "LINEAR") in ("LINEAR", "STEP")
        times = accessor(document, binary, sampler["input"])
        values = accessor(document, binary, sampler["output"])
        duration = max(duration, times[-1][0])
        tracks[(channel["target"]["node"], channel["target"]["path"])] = (times, values)

    def local(index, t):
        node = nodes[index]
        trs = {
            "translation": tuple(node.get("translation", (0, 0, 0))),
            "rotation": tuple(node.get("rotation", (0, 0, 0, 1))),
            "scale": tuple(node.get("scale", (1, 1, 1))),
        }
        for key in trs:
            if (index, key) in tracks:
                times, values = tracks[(index, key)]
                trs[key] = sample(times, values, t, key == "rotation")
        return compose(trs["translation"], trs["rotation"], trs["scale"])

    def world(index, t):
        matrix = local(index, t)
        while index in parent:
            index = parent[index]
            matrix = multiply(local(index, t), matrix)
        return matrix

    mesh = nodes[target].get("mesh")
    corners = []
    if mesh is not None:
        bounds = document["accessors"][
            document["meshes"][mesh]["primitives"][0]["attributes"]["POSITION"]
        ]
        low, high = bounds["min"], bounds["max"]
        middle = [(a + b) / 2 for a, b in zip(low, high)]
        # Face centres of the weapon's local bounds: one of them is its business end.
        for axis in range(3):
            for end in (low, high):
                face = list(middle)
                face[axis] = end[axis]
                corners.append(tuple(face))
    print(f"{path} node={weapon} clip={clip_name} duration={duration:.4f}s")
    print("phase  time    origin(x,y,z)              muzzle(x,y,z) [most forward (-Z) bounds face centre]")
    for step in range(steps + 1):
        phase = step / steps
        matrix = world(target, phase * duration)
        origin = point(matrix, (0, 0, 0))
        muzzle = min((point(matrix, c) for c in corners), key=lambda p: p[2]) if corners else origin
        print(
            f"{phase:5.2f}  {phase * duration:6.3f}  "
            f"{origin[0]:7.3f} {origin[1]:7.3f} {origin[2]:7.3f}   "
            f"{muzzle[0]:7.3f} {muzzle[1]:7.3f} {muzzle[2]:7.3f}"
        )


if __name__ == "__main__":
    main()
