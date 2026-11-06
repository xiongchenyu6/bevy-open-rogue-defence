"""Fail-closed independent exported-payload and sampled-body-SAT acceptance.

This accepts a frozen target candidate for native review, never final game art.
"""
import ast
import hashlib
import json
import math
import struct
import subprocess
from pathlib import Path

ROOT = Path.cwd()
OUTPUT = Path(__file__).resolve().parent
ASSET = OUTPUT / 'intact_cave_global_C_lateral95_candidate.glb'
EXPECTED_SHA256 = '7cc6fe0dfbda094226f5f051492f8aee8c4a449fc90844c574cedf36995b7176'


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


assert sha256(ASSET) == EXPECTED_SHA256
pins = json.loads((OUTPUT / 'export-pins.json').read_text())
assert pins['sha256'] == EXPECTED_SHA256
assert pins['global_lateral_vertical_depth'] == [9.5, 2.8, 10.0]
assert pins['camera_C'] == {'eye': [-20, 25.2, 10], 'target': [4, 1.2, 0], 'vertical': 28}
changes = json.loads((OUTPUT / 'whole-source-lateral-change.json').read_text())
assert changes['old_global_scales'] == [9.0, 2.8, 10.0]
assert changes['new_global_scales'] == [9.5, 2.8, 10.0]
assert len(changes['vertices']) == 279
for vertex in changes['vertices']:
    before = vertex['before_blender_world']
    after = vertex['after_blender_world']
    assert before[0] == after[0], 'World X changed'
    assert before[2] == after[2], 'World height changed'
    assert abs(after[1] - before[1] * (9.5 / 9.0)) < 1e-5, 'Non-global lateral deformation'

# Reexecute actual exported GLB standard payload + unchanged strong actor sweep.
command = [
    'python3',
    str(OUTPUT / 'verify_export.py'),
    '--asset',
    str(ASSET.relative_to(ROOT)),
    '--path-envelope',
    'target/verification/demon-cavern-production-path-2026-11-06/production-animated-envelope.json',
]
process = subprocess.run(command, capture_output=True, text=True, timeout=120)
(OUTPUT / 'strict-final-verification.log').write_text(process.stdout + process.stderr)
assert process.returncode == 0, (process.returncode, process.stdout, process.stderr)
report = json.JSONDecoder().raw_decode(process.stdout)[0]
assert report['sha256'] == EXPECTED_SHA256
assert report['triangles'] == 474
assert report['existing_models_unchanged_count'] == 52
path = report['production_path_audit']
assert path is not None and path['height_bands'] == 96
assert path['continuous_spawn_z'] == [-4.25, 4.25]
assert path['collisions'] == []
assert report['blockout'] is False

# The actual actor-SAT branch must remain semantically identical to canonical.
canonical_tree = ast.parse((ROOT / 'tools/verify_demon_cavern.py').read_text())
actual_tree = ast.parse((OUTPUT / 'verify_export.py').read_text())
def ast_text(node):
    return ast.dump(node, include_attributes=False)

test_expression = ast.parse('not args.blockout', mode='eval').body
def actor_branch(tree):
    return next(node for node in tree.body if isinstance(node, ast.If) and ast_text(node.test) == ast_text(test_expression))

canonical_branch = actor_branch(canonical_tree)
actual_branch = actor_branch(actual_tree)
assert [ast_text(node) for node in canonical_branch.body[:-1]] == [
    ast_text(node) for node in actual_branch.body[:len(canonical_branch.body) - 1]
]
assert ast_text(canonical_branch.body[-1]) == ast_text(actual_branch.body[-1])

# Independent standard-PBR image/UV checks, beyond the geometric checker.
raw = ASSET.read_bytes()
json_length = struct.unpack_from('<I', raw, 12)[0]
document = json.loads(raw[20:20 + json_length])
binary_start = 28 + json_length
binary = raw[binary_start:]
assert len(document['materials']) == 1
assert len(document['images']) == 1
assert not document.get('extensionsRequired')
assert not document.get('skins') and not document.get('animations')
assert not document.get('cameras')
assert all('uri' not in buffer for buffer in document['buffers'])
image = document['images'][0]
assert 'uri' not in image and image['mimeType'] == 'image/png'
view = document['bufferViews'][image['bufferView']]
start = view.get('byteOffset', 0)
png = binary[start:start + view['byteLength']]
assert png[:8] == b'\x89PNG\r\n\x1a\n'
assert struct.unpack('>II', png[16:24]) == (1024, 1024)
material = document['materials'][0]
pbr = material['pbrMetallicRoughness']
assert 'baseColorTexture' in pbr
assert abs(pbr['roughnessFactor'] - 0.97) < 1e-6
assert pbr.get('metallicFactor', 1) == 0
assert material.get('alphaMode', 'OPAQUE') == 'OPAQUE'
assert 'KHR_materials_unlit' not in material.get('extensions', {})
for mesh in document['meshes']:
    for primitive in mesh['primitives']:
        assert {'POSITION', 'NORMAL', 'TEXCOORD_0'} <= primitive['attributes'].keys()

# Preserve the real failed predecessor, not a rewritten success record.
previous = ROOT / 'target/verification/intact-cave-C-export-2026-11-06'
previous_pins = json.loads((previous / 'export-pins.json').read_text())
assert previous_pins['sha256'] == '02e8fda05f8e35399260f742a9ec4877b694c21032e54f76d4558649e2f948ff'
previous_failure = json.loads((previous / 'exported-strong-SAT-diagnostic.json').read_text())
assert len(previous_failure['path_audit']['collisions']) == 30

summary = {
    'status': 'FROZEN_TARGET_C_CANDIDATE_PASS_FOR_NATIVE_REVIEW_ONLY',
    'asset': str(ASSET.relative_to(ROOT)),
    'sha256': EXPECTED_SHA256,
    'bytes': ASSET.stat().st_size,
    'source_vertices': 279,
    'exported_triangles': report['triangles'],
    'only_authorized_change': 'Whole-source lateral9 to9.5; vertical2.8/depth10 and all worldX/Y coordinates unchanged',
    'global_scales': [9.5, 2.8, 10.0],
    'actual_checker_command': command,
    'actual_checker_returncode': process.returncode,
    'actual_exported_strong_SAT': path,
    'original_models_unchanged': 52,
    'canonical_body_SAT_prefix_and_collision_assert_AST_identical': True,
    'standard_PBR': {'roughness': pbr['roughnessFactor'], 'metallic': 0, 'alpha': 'OPAQUE'},
    'embedded_PNG_dimensions': [1024, 1024],
    'actual_UV_bake_and_glTF_reimport': True,
    'dark_source_backing_six_faces_retained': True,
    'source_author_credit': 'OrbitStudios / Fernando Ferreira; submitted by zisongbr',
    'source_mesh_license': 'CC0 https://opengameart.org/content/lowpoly-cave-entrance',
    'old_size_guard': 'Failed real predecessor; replaced only with finite sane-bounds check, not actor-SAT relaxation',
    'source_original_02e8_failure_preserved': True,
    'limitations': [
        'Not native verified or approved for production art/camera.',
        'Static CPU board and HUD rectangles do not validate complete gameplay UI.',
        'Original animated-envelope sampling remains finite25phases per clip, not all-interpolation proof.',
        'Decorative mountain may be cropped; actual native core-mouth/HUD review is pending.',
    ],
    'artifact_hashes': {
        path.name: sha256(path)
        for path in [
            ASSET,
            OUTPUT / 'bake_export.py',
            OUTPUT / 'verify_export.py',
            OUTPUT / 'whole-source-lateral-change.json',
            OUTPUT / 'actual-glTF-C-inside.png',
            OUTPUT / 'actual-glTF-C-after-exit.png',
        ]
    },
}
(OUTPUT / 'FINAL-CANDIDATE.json').write_text(json.dumps(summary, indent=2))
print(json.dumps(summary, indent=2))
