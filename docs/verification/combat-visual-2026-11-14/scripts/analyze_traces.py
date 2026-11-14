"""Contact-frame alignment and before/after damage-row comparison from native capture logs.
usage: analyze_traces.py VERIFICATION_DIR  -> writes trace-analysis.json there."""
import json, re, sys, pathlib
root = pathlib.Path(sys.argv[1])
ANSI = re.compile(r'\x1b\[[0-9;]*m')
VEC = r'Vec3\(([-\d.e]+), ([-\d.e]+), ([-\d.e]+)\)'
SOCKET = {'ember': (0.951, 0.890, 0.346), 'storm': (1.575, 0.906, 0.305)}  # forward, up, right

def lines(path):
    return [ANSI.sub('', l) for l in path.read_text(errors='replace').splitlines()]

def hits(log):
    rows = []
    for l in log:
        if 'PROJECTILE_HIT' not in l:
            continue
        row = dict(re.findall(r'(\w+)=([^\s]+)', l.split('PROJECTILE_HIT', 1)[1]))
        rows.append({k: row[k] for k in ('base_damage', 'final_damage', 'critical_roll_bits', 'crit_multiplier',
                                         'hp_before_bits', 'hp_after_bits', 'rng_before_crit', 'rng_after_crit',
                                         'rng_after_resolution')})
    return rows

def frames(log, tag, extra=None):
    out = []
    for l in log:
        if l.startswith(tag + ' '):
            f = int(re.search(r'frame=(\d+)', l).group(1))
            if extra is None or extra(l):
                out.append((f, l))
    return out

result = {}
for kind in ('ember', 'storm', 'sword'):
    after = lines(root / 'after' / f'attack-{kind}.log')
    before = lines(root / 'before' / f'attack-{kind}.log')
    entry = {}
    b, a = hits(before), hits(after)
    shared = min(len(b), len(a))
    entry['damage_rows'] = {'before': len(b), 'after': len(a), 'shared_prefix': shared,
                            'identical_shared_rows': sum(x == y for x, y in zip(b, a))}
    contact = [f for f, _ in frames(after, 'ATTACK_TIMING', lambda l: 'damage=0.000' not in l)]
    lights = sorted({f for f, _ in frames(after, 'CONTACT_LIGHT')})
    hit_audio = [f for f, _ in frames(after, 'AUDIO_START', lambda l: 'audio/hit' in l or 'audio/crit' in l)]
    shot_audio = [f for f, _ in frames(after, 'AUDIO_START', lambda l: 'audio/shot' in l)]
    other_audio = sorted({re.search(r'source=Some\("([^"]+)"', l).group(1) for _, l in frames(after, 'AUDIO_START')})
    releases = frames(after, 'PROJECTILE_RELEASE')
    entry['contact_frames_first80'] = contact
    entry['contacts_with_light_same_frame'] = sum(f in lights for f in contact)
    entry['light_start_frames_without_contact'] = [f for f in lights if f - 1 not in lights and f not in contact and f <= 80]
    entry['hit_audio_frames'] = hit_audio
    entry['hit_audio_on_contact_frame'] = sum(f in contact for f in hit_audio if f <= 80)
    entry['hit_audio_first80'] = sum(f <= 80 for f in hit_audio)
    entry['audio_sources_seen'] = other_audio
    rel_frames = [f for f, _ in releases]
    entry['release_frames'] = rel_frames
    entry['shot_audio_frames'] = shot_audio
    entry['shot_audio_on_release_frame'] = sum(f in rel_frames for f in shot_audio)
    if kind in SOCKET:
        errs, phases, rig = [], [], []
        for f, l in releases:
            pos = [float(x) for x in re.search(r'position=' + VEC, l).groups()]
            ren = [float(x) for x in re.search(r'rendered=' + VEC, l).groups()]
            hero = [float(x) for x in re.search(r'hero_pose=Some\(' + VEC, l).groups()]
            node = [float(x) for x in re.search(r'weapon_node=Some\(' + VEC, l).groups()]
            off = [ren[i] - pos[i] for i in range(3)]  # fixture hero faces +X: forward=x, right=z
            want = (SOCKET[kind][0], SOCKET[kind][1] - 0.25, SOCKET[kind][2])
            errs.append(max(abs(off[i] - want[i]) for i in range(3)))
            clip = re.search(r'clips=\[\((\d+), ([\d.]+), ([\d.]+)\)', l)
            phases.append(float(clip.group(2)))
            rig.append([round(node[i] - hero[i], 4) for i in range(3)])
        entry['release_rendered_minus_logic_max_error_vs_socket'] = max(errs)
        entry['release_clip_seconds'] = sorted(set(round(p, 4) for p in phases))
        entry['live_weapon_node_minus_hero_first'] = rig[0]
        entry['logic_lane_height_all_0.25'] = all('position=Vec3(' in l and re.search(r'position=' + VEC, l).group(2) == '0.25' for _, l in releases)
    result[kind] = entry
(root / 'trace-analysis.json').write_text(json.dumps(result, indent=1))
print(json.dumps(result, indent=1))
