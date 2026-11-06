"""Read-only self-contained archive verification; no mutable game assets or HEAD.

No Blender, CUDA, game launch, target folder, network or generated-image service.
"""
import ast
import copy
import gzip
import hashlib
import json
import struct
import subprocess
import zipfile
from pathlib import Path

BASE = Path(__file__).resolve().parent
ROOT = BASE
FINAL_SHA = '7cc6fe0dfbda094226f5f051492f8aee8c4a449fc90844c574cedf36995b7176'
NEGATIVE_SHA = '02e8fda05f8e35399260f742a9ec4877b694c21032e54f76d4558649e2f948ff'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check(command, expected_returncode):
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=120)
    assert result.returncode == expected_returncode, (command, result.returncode, result.stdout, result.stderr)
    return result


manifest = json.loads((BASE / 'SHA256.json').read_text())
files = {
    str(path.relative_to(BASE))
    for path in BASE.rglob('*')
    if path.is_file() and path.name != 'SHA256.json'
}
assert files == set(manifest), ('Archive file inventory changed', files ^ set(manifest))
for name, expected in manifest.items():
    assert sha(BASE / name) == expected, name

asset = BASE / 'candidate/intact-cave-7cc6.glb'
negative = BASE / 'negative/lateral9-02e8.glb'
assert sha(asset) == FINAL_SHA
assert sha(negative) == NEGATIVE_SHA
final = json.loads((BASE / 'candidate/actual-final-candidate.json').read_text())
assert final['sha256'] == FINAL_SHA
assert final['actual_checker_returncode'] == 0
assert final['actual_exported_strong_SAT']['collisions'] == []
assert final['global_scales'] == [9.5, 2.8, 10.0]
changes = json.loads((BASE / 'candidate/whole-source-lateral-change.json').read_text())
assert len(changes['vertices']) == 279
for vertex in changes['vertices']:
    before = vertex['before_blender_world']
    after = vertex['after_blender_world']
    assert before[0] == after[0] and before[2] == after[2]
    assert abs(after[1] - before[1] * (9.5 / 9.0)) < 1e-5

# Historical source hashes come from the archived actual positive log, never HEAD.
baseline = json.loads((BASE / 'source/original-52-model-hashes.json').read_text())
historical = json.JSONDecoder().raw_decode((BASE / 'logs/actual-strict-export-check.log').read_text())[0]
assert len(baseline) == 52 and baseline == historical['existing_model_sha256']
envelope_data = json.loads((BASE / 'source/production-animated-envelope.json').read_text())
assert len(envelope_data['source_units']) == 11
assert sha(BASE / 'source/production-animated-envelope.json') == 'feb5795a8887f46fe27e1bb92a0f88fc1abf3f406728f8bfcc03034c14e89f23'
for unit in envelope_data['source_units']:
    assert unit['sha256'] == baseline[f"assets/models/enemy_{unit['name']}.glb"]

# Pin exact geometry math, finite path, triangle/AABB SAT and final collision assert.
# Only historical source-hash resolution is archive-local rather than mutable assets.
contract = json.loads((BASE / 'body-code-contract.json').read_text())
tree = ast.parse((BASE / 'check_payload_and_path.py').read_text())
test = ast.dump(ast.parse('not args.blockout', mode='eval').body, include_attributes=False)
branch = next(node for node in tree.body if isinstance(node, ast.If) and ast.dump(node.test, include_attributes=False) == test)
start = contract['geometry_statement_start']
count = contract['geometry_prefix_statement_count']
geometry = branch.body[start:start + count] + [branch.body[-1]]
digest = hashlib.sha256(''.join(ast.dump(node, include_attributes=False) for node in geometry).encode()).hexdigest()
assert digest == contract['geometry_statements_ast_sha256'], 'Geometry/body SAT changed'

# Re-run exported-GLB SAT, including the actual failed predecessor, from archive files.
envelope = BASE / 'source/production-animated-envelope.json'
checker = BASE / 'check_payload_and_path.py'
positive_command = [
    'python3', str(checker), '--asset', str(asset.relative_to(ROOT)),
    '--path-envelope', str(envelope.relative_to(ROOT)),
]
positive = check(positive_command, 0)
report = json.JSONDecoder().raw_decode(positive.stdout)[0]
assert report['sha256'] == FINAL_SHA and report['triangles'] == 474
assert report['historical_original_model_baseline_count'] == 52
assert report['production_path_audit']['height_bands'] == 96
assert report['production_path_audit']['continuous_spawn_z'] == [-4.25, 4.25]
assert report['production_path_audit']['collisions'] == []
negative_command = [
    'python3', str(checker), '--asset', str(negative.relative_to(ROOT)),
    '--path-envelope', str(envelope.relative_to(ROOT)),
]
failed = check(negative_command, 1)
failure = json.JSONDecoder().raw_decode(failed.stdout)[0]['PRODUCTION_PATH_FAIL']
assert len(failure['collisions']) == 30
assert [group['band'] for group in failure['collisions']] == list(range(27, 57))
assert {index for group in failure['collisions'] for index in group['triangles']} == {309, 350, 397, 459}
legacy_command = [
    'python3', str(BASE / 'check_legacy_size_guard.py'),
    '--asset', str(negative.relative_to(ROOT)),
    '--path-envelope', str(envelope.relative_to(ROOT)),
]
legacy = check(legacy_command, 1)
assert 'assert 5.25 <= hi[1] <= 6.55' in legacy.stderr
assert 'AssertionError' in legacy.stderr

# Independent standard-material/UV/embedded PNG checks.
raw = asset.read_bytes()
length = struct.unpack_from('<I', raw, 12)[0]
document = json.loads(raw[20:20 + length])
binary = raw[28 + length:]
assert len(document['materials']) == 1 and len(document['images']) == 1
assert not document.get('skins') and not document.get('animations')
assert not document.get('cameras') and not document.get('extensionsRequired')
assert all('uri' not in item for item in document['buffers'])
image = document['images'][0]
assert 'uri' not in image and image['mimeType'] == 'image/png'
view = document['bufferViews'][image['bufferView']]
offset = view.get('byteOffset', 0)
png = binary[offset:offset + view['byteLength']]
assert png[:8] == b'\x89PNG\r\n\x1a\n' and struct.unpack('>II', png[16:24]) == (1024, 1024)
material = document['materials'][0]
pbr = material['pbrMetallicRoughness']
assert 'baseColorTexture' in pbr and abs(pbr['roughnessFactor'] - 0.97) < 1e-6
assert pbr.get('metallicFactor', 1) == 0 and material.get('alphaMode', 'OPAQUE') == 'OPAQUE'
assert all({'POSITION', 'NORMAL', 'TEXCOORD_0'} <= primitive['attributes'].keys() for mesh in document['meshes'] for primitive in mesh['primitives'])

# Check original author's source bytes and the separately generated material provenance.
source_zip = BASE / 'source/Cave_Entrance_lowpoly.zip'
assert sha(source_zip) == '335aadd05fe302545e96b199c0247bdc6ab3b1f29efa3e8bb057ab67dad72892'
source = json.loads((BASE / 'source/original-source-and-global-transform.json').read_text())
with zipfile.ZipFile(source_zip) as archive:
    original_blend = archive.read('Cave_Entrance_lowpoly/cave_entrance.blend')
assert hashlib.sha256(original_blend).hexdigest() == source['source_blend_sha256']
raw_page = gzip.decompress((BASE / 'source/original-author-asset-page.html.gz').read_bytes())
license_page = json.loads((BASE / 'source/license-page-provenance.json').read_text())
assert hashlib.sha256(raw_page).hexdigest() == license_page['original_raw_HTML_sha256']
page = raw_page.decode()
assert 'creativecommons.org/publicdomain/zero/1.0/' in page
assert 'OrbitStudios, Fernando Ferreira' in page
material_source = json.loads((BASE / 'source/seamless-generation.json').read_text())
assert sha(BASE / 'source/original-generated-seamless-slate.png') == material_source['sha256']

print(json.dumps({
    'status': 'ARCHIVE_ASSET_VERIFY_PASS',
    'candidate_sha256': FINAL_SHA,
    'positive_actual_SAT_returncode': positive.returncode,
    'positive_collisions': 0,
    'negative_actual_SAT_returncode': failed.returncode,
    'negative_colliding_height_bands': 30,
    'legacy_size_guard_returncode': legacy.returncode,
    'standard_embedded_UV_PBR': True,
    'source_and_material_hashes_verified': True,
    'historical_model_baseline_count': 52,
    'no_mutable_repository_inputs': True,
    'scope': 'Asset/export record only; native readability is separate, production camera/input/art adoption unverified by this record.',
}, indent=2))
