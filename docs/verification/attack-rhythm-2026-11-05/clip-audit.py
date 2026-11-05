#!/usr/bin/env python3
"""Read-only GLB attack bone audit. Run from the repository root; no dependencies.

Positions include each exported node's transforms, including the imported model
scale/forward rotation. They are authored model-space measurements, not collision
points, weapon tips, or a claim that an authored contact event exists. In-between
quaternion keys use shortest-arc normalized linear interpolation, matching the
initial exploratory audit; channels are sampled at 30Hz and the approximation is
explicit rather than presented as an exact Bevy animation evaluation.
"""
import bisect
import hashlib
import json
import math
import re
from pathlib import Path
import struct

ROOT = Path.cwd()
OUTPUT = Path(__file__).resolve().with_name('clip-audit.json')
FRACTIONS = [0.0, 0.25, 0.42, 0.5, 0.6, 0.7, 0.8, 1.0]
MODELS = ['stone_warden', 'solar_vanguard', 'iron_berserker', 'ember_ranger', 'storm_savant']
MANIFEST = {entry['file']: entry for entry in json.loads((ROOT / 'assets/models/cast-manifest.json').read_text())}


def multiply(a, b):
    return [[sum(a[r][k] * b[k][c] for k in range(4)) for c in range(4)] for r in range(4)]


def trs(translation, rotation, scale):
    x, y, z, w = rotation
    return [
        [(1-2*y*y-2*z*z)*scale[0], (2*x*y-2*z*w)*scale[1], (2*x*z+2*y*w)*scale[2], translation[0]],
        [(2*x*y+2*z*w)*scale[0], (1-2*x*x-2*z*z)*scale[1], (2*y*z-2*x*w)*scale[2], translation[1]],
        [(2*x*z-2*y*w)*scale[0], (2*y*z+2*x*w)*scale[1], (1-2*x*x-2*y*y)*scale[2], translation[2]],
        [0, 0, 0, 1],
    ]


def audit(model):
    path = ROOT / f'assets/models/hero_{model}.glb'
    raw = path.read_bytes()
    magic, version, total = struct.unpack_from('<4sII', raw)
    assert magic == b'glTF' and version == 2 and total == len(raw)
    chunks = {}
    offset = 12
    while offset < len(raw):
        length, kind = struct.unpack_from('<I4s', raw, offset)
        chunks[kind] = raw[offset + 8:offset + 8 + length]
        offset += 8 + length
    document = json.loads(chunks[b'JSON'])
    binary = chunks[b'BIN\x00']

    def accessor(index):
        value = document['accessors'][index]
        assert value['componentType'] == 5126 and 'sparse' not in value
        view = document['bufferViews'][value['bufferView']]
        width = {'SCALAR': 1, 'VEC3': 3, 'VEC4': 4}[value['type']]
        offset = view.get('byteOffset', 0) + value.get('byteOffset', 0)
        stride = view.get('byteStride', width * 4)
        return [struct.unpack_from('<' + 'f' * width, binary, offset + n * stride) for n in range(value['count'])]

    nodes = document['nodes']
    parents = {child: parent for parent, node in enumerate(nodes) for child in node.get('children', [])}
    animation = next(value for value in document['animations'] if value['name'] == 'Attack')
    channels = []
    duration = 0.0
    for channel in animation['channels']:
        sampler = animation['samplers'][channel['sampler']]
        assert sampler.get('interpolation', 'LINEAR') == 'LINEAR'
        times = [value[0] for value in accessor(sampler['input'])]
        duration = max(duration, times[-1])
        channels.append((channel['target']['node'], channel['target']['path'], times, accessor(sampler['output'])))
    slot = next(index for index, node in enumerate(nodes) if node.get('name') == 'handslot.r')

    def sample(fraction):
        time = duration * fraction
        pose = [{'translation': node.get('translation', [0, 0, 0]),
                 'rotation': node.get('rotation', [0, 0, 0, 1]),
                 'scale': node.get('scale', [1, 1, 1])} for node in nodes]
        for index, path_name, times, values in channels:
            hi = min(bisect.bisect_right(times, time), len(times) - 1)
            lo = max(0, hi - 1)
            factor = 0.0 if times[hi] == times[lo] else (time - times[lo]) / (times[hi] - times[lo])
            left, right = values[lo], values[hi]
            if path_name == 'rotation' and sum(a*b for a, b in zip(left, right)) < 0:
                right = tuple(-value for value in right)
            value = [a + (b-a) * factor for a, b in zip(left, right)]
            if path_name == 'rotation':
                norm = math.sqrt(sum(component * component for component in value))
                value = [component / norm for component in value]
            pose[index][path_name] = value
        cache = {}

        def global_matrix(index):
            if index in cache:
                return cache[index]
            node, value = nodes[index], pose[index]
            local = trs(value['translation'], value['rotation'], value['scale'])
            if 'matrix' in node:
                local = [[node['matrix'][column * 4 + row] for column in range(4)] for row in range(4)]
            result = multiply(global_matrix(parents[index]), local) if index in parents else local
            cache[index] = result
            return result

        matrix = global_matrix(slot)
        return {'fraction': fraction, 'source_seconds': round(time, 9),
                'slot_position': [round(matrix[row][3], 9) for row in range(3)],
                'slot_local_z_axis_in_model_space': [round(matrix[row][2], 9) for row in range(3)]}

    # Resolve channel targets by bone name, so this compares animation data rather
    # than model-specific node indices, mesh tint, or exported GLB byte layout.
    signature_data = sorted((nodes[index].get('name'), path_name, times, values)
                            for index, path_name, times, values in channels)
    signature = hashlib.sha256(json.dumps(signature_data, separators=(',', ':')).encode()).hexdigest()
    return {'file': str(path.relative_to(ROOT)), 'sha256': hashlib.sha256(raw).hexdigest(),
            'source_model': MANIFEST[path.name]['source'],
            'source_clip': MANIFEST[path.name]['clips']['Attack'],
            'attack_channel_sha256': signature, 'duration_seconds': duration,
            'bone': 'handslot.r',
            'attached_children': [nodes[index].get('name') for index in nodes[slot].get('children', [])],
            'samples': [sample(fraction) for fraction in FRACTIONS]}


models = [audit(model) for model in MODELS]
assert models[0]['source_clip'] == models[1]['source_clip'] == '1H_Melee_Attack_Chop'
assert models[0]['attack_channel_sha256'] == models[1]['attack_channel_sha256']
assert models[0]['samples'] == models[1]['samples']
result = {'method': 'Read-only GLB TRS hierarchy; LINEAR vector and shortest-arc NLERP quaternion sampling',
          'limitations': 'Bone trajectory, not an authored release event, weapon-tip collision, native animation trace, or Bevy exact SLERP evaluation',
          'theoretical_unmodified_build_only': {
              'attacks_per_second': 1.954, 'interval_seconds': 1 / 1.954,
              'action_window_seconds': 0.45, 'old_contact_seconds': 0.45 * 0.42,
              'proposed_sword_contact_seconds': 0.45 * 0.6,
              'warning': 'Not the observed hero runtime: hero-derived attack speed changes the window'},
          'stone_solar_identical_attack_channels_and_samples': True, 'models': models}
trace = OUTPUT.parent / 'before-sword.log'
if trace.exists():
    observations = []
    for match in re.finditer(r'ATTACK_TIMING frame=(\d+) health=[\d.]+ damage=([\d.]+) clips=\[\((\d+), ([\d.]+), ([\d.]+)\)\]', trace.read_text()):
        frame, damage, graph_node, seek, speed = match.groups()
        if float(damage) > 0:
            observations.append({'frame': int(frame), 'damage': float(damage),
                                 'animation_graph_node': int(graph_node),
                                 'observed_source_seek_seconds': float(seek),
                                 'observed_animation_speed': float(speed),
                                 'observed_source_phase': float(seek) / models[0]['duration_seconds'],
                                 'inferred_action_window_seconds': models[0]['duration_seconds'] / float(speed)})
    assert observations, 'Available native baseline must contain actual positive-damage trace'
    result['native_baseline'] = {'file': str(trace.relative_to(ROOT)),
                                 'sha256': hashlib.sha256(trace.read_bytes()).hexdigest(),
                                 'note': 'Observed trace takes precedence over theoretical unmodified-build timing',
                                 'positive_damage_observations': observations}
OUTPUT.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'output': str(OUTPUT.relative_to(ROOT)), 'stone_solar_identical': True,
                  'models': [{'file': m['file'], 'sha256': m['sha256'], 'clip': m['source_clip']} for m in models]}, indent=2))
