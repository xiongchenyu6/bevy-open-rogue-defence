"""Independent stdlib GLB/payload/cavity checks. No Blender, Rust or game launch."""
from pathlib import Path
import argparse, hashlib, json, math, struct, subprocess
ROOT = Path.cwd()
parser = argparse.ArgumentParser()
parser.add_argument('--asset', required=True)
parser.add_argument('--blockout', action='store_true')
parser.add_argument('--vertex-painted', action='store_true')
parser.add_argument('--path-envelope')
args = parser.parse_args()
PATH = ROOT / args.asset
raw = PATH.read_bytes()
magic, version, length = struct.unpack_from('<III', raw)
assert magic == 1179937895 and version == 2 and (length == len(raw))
size, kind = struct.unpack_from('<II', raw, 12)
assert kind == 1313821514
doc = json.loads(raw[20:20 + size])
bs, kind = struct.unpack_from('<II', raw, 20 + size)
assert kind == 5130562
binary = raw[28 + size:]
assert len(binary) == bs and len(doc['buffers']) == 1 and ('uri' not in doc['buffers'][0])
assert not doc.get('cameras') and (not doc.get('animations')) and (not doc.get('skins'))
assert not any(('KHR_lights_punctual' in node.get('extensions', {}) for node in doc['nodes']))
assert args.blockout or args.vertex_painted or doc.get('images')
assert all(('uri' not in image and 'bufferView' in image for image in doc.get('images', [])))
assert len(raw) < 4000000
TYPES = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4}
FMT = {5121: 'B', 5123: 'H', 5125: 'I', 5126: 'f'}

def accessor(i):
    a = doc['accessors'][i]
    view = doc['bufferViews'][a['bufferView']]
    fmt = '<' + FMT[a['componentType']] * TYPES[a['type']]
    width = struct.calcsize(fmt)
    stride = view.get('byteStride', width)
    start = view.get('byteOffset', 0) + a.get('byteOffset', 0)
    return [struct.unpack_from(fmt, binary, start + k * stride) for k in range(a['count'])]
triangles = []
vertices = []
for mesh in doc['meshes']:
    for primitive in mesh['primitives']:
        assert {'POSITION', 'NORMAL'} <= primitive['attributes'].keys()
        assert args.blockout or args.vertex_painted or 'TEXCOORD_0' in primitive['attributes']
        if args.vertex_painted and (not doc['materials'][primitive['material']].get('emissiveFactor')):
            assert 'COLOR_0' in primitive['attributes']
            colors = accessor(primitive['attributes']['COLOR_0'])
            assert len(colors) > 0 and all((math.isfinite(c) for row in colors for c in row))
        points = accessor(primitive['attributes']['POSITION'])
        vertices += points
        assert all((math.isfinite(v) for p in points for v in p))
        indices = [x[0] for x in accessor(primitive['indices'])]
        assert len(indices) % 3 == 0
        triangles += [tuple((points[x] for x in indices[k:k + 3])) for k in range(0, len(indices), 3)]
for node in doc['nodes']:
    assert node.get('translation', [0, 0, 0]) == [0, 0, 0]
    assert node.get('scale', [1, 1, 1]) == [1, 1, 1]
    assert node.get('rotation', [0, 0, 0, 1]) == [0, 0, 0, 1]
lo = [min((v[k] for v in vertices)) for k in range(3)]
hi = [max((v[k] for v in vertices)) for k in range(3)]
assert all((math.isfinite(value) and abs(value) < 256 for value in lo + hi)), (lo, hi)
assert all((0 < hi[k] - lo[k] < 256 for k in range(3))), (lo, hi)
assert (50 if args.blockout else 300) < len(triangles) < 30000
for material in doc['materials']:
    pbr = material['pbrMetallicRoughness']
    assert pbr.get('metallicFactor', 1) == 0 and pbr['roughnessFactor'] >= 0.9
    assert material.get('alphaMode', 'OPAQUE') == 'OPAQUE' and 'KHR_materials_unlit' not in material.get('extensions', {})

def ray_hits(y, z):
    hits = []
    for a, b, c in triangles:
        det = (b[1] - a[1]) * (c[2] - a[2]) - (c[1] - a[1]) * (b[2] - a[2])
        if abs(det) < 1e-08:
            continue
        u = ((y - a[1]) * (c[2] - a[2]) - (c[1] - a[1]) * (z - a[2])) / det
        v = ((b[1] - a[1]) * (z - a[2]) - (y - a[1]) * (b[2] - a[2])) / det
        if u >= -1e-06 and v >= -1e-06 and (u + v <= 1 + 1e-06):
            hits.append(a[0] + u * (b[0] - a[0]) + v * (c[0] - a[0]))
    return sorted(set((round(h, 4) for h in hits)))
clearance = {}
for lane in [-4.3, -2, 0, 2, 4.3]:
    hits = ray_hits(0.7, lane)
    clearance[str(lane)] = hits[:4]
path_audit = None
if not args.blockout:
    assert args.path_envelope, '--path-envelope required: local rays are not a production clearance gate'
    envelope = json.loads((ROOT / args.path_envelope).read_text())
    assert envelope['spawn_world_x'] == [15.3, 16.4] and envelope['spawn_world_z'] == [-4.25, 4.25]
    assert envelope['den_translation'] == [15.65, 0, 0] and envelope['den_yaw'] == -0.55
    expected = {'rift_sapper', 'gnawling', 'shell_guard', 'void_caster', 'core_rusher', 'chanting_acolyte', 'iron_colossus', 'field_mender', 'split_spore', 'null_eye', 'siege_lobber'}
    assert {u['name'] for u in envelope['source_units']} == expected
    for unit in envelope['source_units']:
        assert unit['sha256'] == hashlib.sha256((ROOT / f"assets/models/enemy_{unit['name']}.glb").read_bytes()).hexdigest()
    c = math.cos(0.55)
    q = math.sin(0.55)
    world_tri = [tuple(((15.65 + c * p[0] - q * p[2], p[1], q * p[0] + c * p[2]) for p in t)) for t in triangles]
    exit_x = min((p[0] for t in world_tri for p in t)) - max((b['max_x'] for b in envelope['bands'])) - 0.2

    def sub(a, b):
        return tuple((a[k] - b[k] for k in range(3)))

    def cross(a, b):
        return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])

    def dot(a, b):
        return sum((a[k] * b[k] for k in range(3)))

    def overlap(tri, low, high):
        if any((max((v[k] for v in tri)) < low[k] or min((v[k] for v in tri)) > high[k] for k in range(3))):
            return False
        center = [(low[k] + high[k]) / 2 for k in range(3)]
        half = [(high[k] - low[k]) / 2 for k in range(3)]
        v = [sub(p, center) for p in tri]
        edges = [sub(v[1], v[0]), sub(v[2], v[1]), sub(v[0], v[2])]
        axes = [(1, 0, 0), (0, 1, 0), (0, 0, 1), cross(edges[0], edges[1])] + [cross(e, a) for e in edges for a in [(1, 0, 0), (0, 1, 0), (0, 0, 1)]]
        for axis in axes:
            if dot(axis, axis) < 1e-18:
                continue
            projections = [dot(p, axis) for p in v]
            radius = sum((half[k] * abs(axis[k]) for k in range(3)))
            if min(projections) > radius + 1e-07 or max(projections) < -radius - 1e-07:
                return False
        return True
    collisions = []
    for i, band in enumerate(envelope['bands']):
        low = [exit_x + band['min_x'], max(0.035, band['y0']), -4.25 + band['min_z']]
        high = [16.4 + band['max_x'], band['y1'], 4.25 + band['max_z']]
        if low[1] >= high[1]:
            continue
        hits = [j for j, t in enumerate(world_tri) if overlap(t, low, high)]
        if hits:
            collisions.append({'band': i, 'world_box': [low, high], 'triangles': hits[:8]})
    path_audit = {'envelope_sha256': hashlib.sha256((ROOT / args.path_envelope).read_bytes()).hexdigest(), 'finite_exit_world_x': exit_x, 'height_bands': len(envelope['bands']), 'continuous_spawn_z': [-4.25, 4.25], 'collisions': collisions}
    details = []
    for group in collisions:
        for index in group['triangles']:
            tri = world_tri[index]
            centroid = [sum((p[k] for p in tri)) / 3 for k in range(3)]
            details.append({'height_band': group['band'], 'body_band_box_world': group['world_box'], 'triangle_index': index, 'triangle_world': tri, 'centroid_world': centroid})
    diagnostic = {'asset_sha256': hashlib.sha256(raw).hexdigest(), 'path_audit': path_audit, 'reported_colliding_triangle_details': details, 'body_sweep_not_weakened': True}
    (ROOT / 'target/verification/intact-cave-C-lateral95-export-2026-11-06/exported-strong-SAT-diagnostic.json').write_text(json.dumps(diagnostic, indent=2))
    assert not collisions, json.dumps({'PRODUCTION_PATH_FAIL': path_audit})
tracked = subprocess.check_output(['git', 'ls-files', 'assets/models/*.glb'], cwd=ROOT, text=True).splitlines()
old = {}
for name in tracked:
    if name == str(PATH.relative_to(ROOT)):
        continue
    data = (ROOT / name).read_bytes()
    assert data == subprocess.check_output(['git', 'show', 'HEAD:' + name], cwd=ROOT), name
    old[name] = hashlib.sha256(data).hexdigest()
report = {'asset': str(PATH.relative_to(ROOT)), 'sha256': hashlib.sha256(raw).hexdigest(), 'bytes': len(raw), 'triangles': len(triangles), 'mesh_count': len(doc['meshes']), 'material_count': len(doc['materials']), 'embedded_images': len(doc.get('images', [])), 'bevy_bounds': [lo, hi], 'front_lane_clearance_y_0_7': clearance, 'existing_models_unchanged_count': len(old), 'existing_model_sha256': old, 'runtime_integrated': False, 'blockout': args.blockout, 'production_path_audit': path_audit}
print(json.dumps(report, indent=2))
print('DEMON_CAVERN_BLOCKOUT_GEOMETRY_PASS not finished art or runtime' if args.blockout else 'DEMON_CAVERN_ASSET_PASS independent self-contained GLB, matte materials, real open mouth/depth, original models unchanged')
