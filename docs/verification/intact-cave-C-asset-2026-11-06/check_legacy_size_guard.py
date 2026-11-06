"""Independent stdlib GLB/payload/cavity checks. No Blender, Rust or game launch."""
from pathlib import Path
import argparse, hashlib, json, math, struct, subprocess
ROOT = Path(__file__).resolve().parent
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
assert 5.25 <= hi[1] <= 6.55 and lo[1] > -0.2, (lo, hi)
assert 4.0 <= hi[2] - lo[2] <= 19.0 and hi[0] - lo[0] > 6, (lo, hi)
print('LEGACY_DECORATIVE_SIZE_PASS')
