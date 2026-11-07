"""Portable archive checks. Uses Python standard library and archived bytes only."""
from pathlib import Path
import gzip
import hashlib
import importlib.util
import json
import re
import sys

sys.dont_write_bytecode = True
ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path(__file__).resolve().parent

def sha(data):
    return hashlib.sha256(data).hexdigest()

def load(name):
    return json.loads((ROOT / name).read_text())

def log(name):
    return gzip.decompress((ROOT / ('raw/' + name + '.gz')).read_bytes()).decode()

def receipt(name, code=0):
    value = load('raw/' + name + '-result.json')
    assert value['returncode'] == code, name
    return value

def native(name, frames, scenario, size):
    value = receipt(name)
    command = value['command']
    assert command[-3:] == [scenario, '0xC0126', size], name
    frame_arg = command.index('frames')
    assert command[frame_arg + 2] == str(frames), name
    assert command[frame_arg - 1].endswith('/r9/capture'), name
    text = log(name + '.log')
    assert f'wrote {frames} simulation frames' in text, name
    assert 'panicked' not in text and 'ERROR' not in text, name
    return text

manifest = load('SHA256.json')
actual = {str(path.relative_to(ROOT)) for path in ROOT.rglob('*')
          if path.is_file() and '__pycache__' not in path.parts}
assert actual == set(manifest) | {'SHA256.json'}, (actual - set(manifest), set(manifest) - actual)
for name, expected in manifest.items():
    assert sha((ROOT / name).read_bytes()) == expected, name
for name, value in load('raw-log-manifest.json').items():
    packed = (ROOT / value['path']).read_bytes()
    raw = gzip.decompress(packed)
    assert sha(packed) == value['gzip_sha256'], name
    assert sha(raw) == value['raw_sha256'] and len(raw) == value['raw_bytes'], name

# Every copied per-attempt source joins its own original pre-run input map.
for stage in ['','r1-parameter-count/','r2/','r3/','r4/','r5/','r6/','r7/','r8/','r9/']:
    values = load('raw/' + stage + 'inputs.json')
    source = ROOT / 'raw' / stage / 'source'
    snapshots = list(source.rglob('*.rs'))
    assert len(snapshots) == 7, stage
    for path in snapshots:
        name = str(path.relative_to(source))
        assert sha(path.read_bytes()) == values[name], (stage, name)
final_inputs = load('raw/r9/inputs.json')
# Main R9 stopped at invalid CLI arguments before its normal post-map write.
assert final_inputs == load('raw/r9/final-build-controls/inputs-before.json') == load('raw/r9/final-build-controls/inputs-after.json')
assert len(final_inputs) == 397
assert load('raw/r7/inputs.json') == load('raw/r7/inputs-after.json')

origin = load('baseline-origin/inputs.json')
pointer = load('baseline-origin/pointer-inputs.json')
assert origin == load('baseline-origin/inputs-after.json')
assert pointer == load('baseline-origin/pointer-inputs-after.json')
assert {name for name in origin.keys() | pointer.keys() if origin.get(name) != pointer.get(name)} == {'src/bin/capture/immediate_choice.rs'}
for name in ['src/game/progression_ui.rs','src/game/progression_ui/layout.rs']:
    assert sha((ROOT / 'baseline-origin/source' / name).read_bytes()) == origin[name]
assert (ROOT / 'raw/before-binary.sha256').read_text().strip() == (ROOT / 'baseline-origin/binary.sha256').read_text().strip()
for size in ['320x667','360x800']:
    receipt('before-' + size)
    assert 'wrote 240 simulation frames' in log('before-' + size + '.log')

for name in ['touch-tests','layout-tests','library-tests','clippy','build','capture-clippy']:
    receipt('r9/' + name)
assert '3 passed; 0 failed' in log('r9/touch-tests.log')
assert '1 passed; 0 failed' in log('r9/layout-tests.log')
assert '773 passed; 0 failed; 1 ignored' in log('r9/library-tests.log')
for name in ['clippy','capture-clippy']:
    assert 'Finished' in log('r9/' + name + '.log')
    assert 'error:' not in log('r9/' + name + '.log')

spec = importlib.util.spec_from_file_location('phone_geometry', ROOT / 'verify_geometry.py')
geometry = importlib.util.module_from_spec(spec)
spec.loader.exec_module(geometry)
geometry_summary = {}
expected_touch = [(120,'Started'),(122,'Moved'),(124,'Moved'),(126,'Moved'),(128,'Moved'),(130,'Ended'),(140,'Started'),(142,'Moved'),(144,'Ended'),(200,'Started'),(204,'Ended')]
for size in ['320x667','360x800']:
    for language in ['en','zh']:
        for count in [3,6]:
            case = f'after-{size}-{language}-{count}'
            text = native('r9/' + case, 240, f'phone-draft-{count}', size)
            touches = [(int(frame), phase) for frame,phase in re.findall(r'PHONE_TOUCH frame=(\d+) phase=(\w+)', text)]
            assert touches == expected_touch, case
            geometry_summary[case] = geometry.check(ROOT / f'raw/r9/{case}.log.gz', count, language == 'en', require_hint=True)
assert sum(value['title_observations'] for value in geometry_summary.values()) == 72
assert sum(value['hint_observations'] for value in geometry_summary.values()) == 8
assert geometry_summary == load('geometry-summary.json')
for name,size,scenario,frames in [('supp-desktop','1280x720','crown-detail',240),('supp-compact','844x390','crown-detail',240),('supp-continuous-desktop','1280x720','immediate-skill',320),('supp-build-skill','1280x720','build-skill',180),('supp-build-bond','1280x720','build-bond',180)]:
    path = 'r9/final-build-controls/' + scenario if scenario.startswith('build-') else 'r9/' + name
    text = native(path, frames, scenario, size)
    if scenario == 'crown-detail':
        assert 'PASS draft-layout frame=100' in text and 'PASS draft-layout frame=180' in text
        assert 'PASS crown-detail:' in text
    elif scenario == 'immediate-skill':
        assert 'PASS immediate-skill: three real mouse choices2/1/0, one settlement/payment per hand, held mouse no duplicate' in text
    else:
        assert 'build preview paused; all cards/actions fit viewport' in text

receipt('r8/library-tests', 101)
assert '772 passed; 1 failed; 1 ignored' in log('r8/library-tests.log')
assert '滑 (U+6ED1)' in log('r8/library-tests.log')

# Preserve actual earlier failures and R6's technically passing visual negative.
receipt('r1-parameter-count/touch-tests', 101)
assert '18systemparameters' in load('raw/r1-parameter-count/reason.json')['result']
receipt('r2/touch-tests', 101)
assert 'ambiguous numeric type' in log('r2/touch-tests.log')
receipt('r3/clippy', 101)
receipt('r4/after-320x667-en-3', 101)
assert 'assertion' in log('r4/after-320x667-en-3.log')
receipt('r5/capture-clippy', 101)
assert 'unused import' in log('r5/capture-clippy.log')
for size in ['320x667','360x800']:
    for language in ['en','zh']:
        for count in [3,6]:
            receipt(f'r6/after-{size}-{language}-{count}')
            assert 'PASS phone-draft:' in log(f'r6/after-{size}-{language}-{count}.log')
negative = next(line for line in log('r6/after-320x667-en-6.log').splitlines() if 'text="Sharpshooter"' in line)
assert geometry.vector(negative, 'glyph_size')[1] > float(re.search(r'font=Px\(([-\d.eE]+)\)',negative)[1]) * 1.3

images = load('root-viewed-images.json')
assert len(images) == 26
assert sum(item['category'] == 'final-phone' for item in images) == 16
for item in images:
    assert sha((ROOT / item['archive']).read_bytes()) == item['sha256']
    assert item['reviewed_by'] == 'root'
assert load('units.json')['r9']['terminal_observed'] == {'ActiveState':'failed','Result':'exit-code','ExecMainStatus':1}
assert load('units.json')['r9-final-controls']['terminal_observed'] == {'ActiveState':'inactive','Result':'success','ExecMainStatus':0}
receipt('r9/supp-build-skill', 101)
assert 'frame 180 out of range' in log('r9/supp-build-skill.log')
assert final_inputs == load('raw/r9/final-build-controls/inputs-before.json') == load('raw/r9/final-build-controls/inputs-after.json')
assert (ROOT / 'raw/r9/binary.sha256').read_text().strip() == (ROOT / 'raw/r9/final-build-controls/binary.sha256').read_text().strip()
assert (ROOT / 'raw/r9/binary.sha256').read_text().strip() == load('local-bulk-manifest.json')['r9/capture']['sha256']
touch_inputs = load('raw/touch-continuation/inputs.json')
assert touch_inputs == load('raw/touch-continuation/inputs-after.json')
assert len(touch_inputs) == 397
assert {k for k in final_inputs.keys() | touch_inputs.keys() if final_inputs.get(k) != touch_inputs.get(k)} == {'src/bin/capture/immediate_choice.rs'}
for path in (ROOT / 'raw/touch-continuation/source').rglob('*.rs'):
    assert sha(path.read_bytes()) == touch_inputs[str(path.relative_to(ROOT / 'raw/touch-continuation/source'))]
for name in ['release-clippy','capture-clippy','build']:
    receipt('touch-continuation/' + name)
    assert 'Finished' in log('touch-continuation/' + name + '.log')
for case in ['touch-immediate-skill','touch-immediate-bond','touch-immediate-skill-paid']:
    name = 'touch-continuation/' + case
    value = receipt(name)
    assert value['command'][-3:] == [case.removeprefix('touch-'), '0xC0126', '360x800']
    assert value['command'][value['command'].index('frames') + 2] == '320'
    assert value['command'][value['command'].index('frames') - 1].endswith('/touch-continuation/capture')
    text = log(name + '.log')
    assert 'wrote 320 simulation frames' in text and 'PASS immediate-' in text
    assert 'panicked' not in text and 'ERROR' not in text
    stages = [36,106,176] + ([246] if case.endswith('-paid') else [])
    assert [int(f) for f in re.findall(r'TOUCH_HELD_BEFORE_RELEASE frame=(\d+)',text)] == stages
    for frame in stages:
        assert f'actual touch Ended frame={frame + 3}' in text
        assert f'TOUCH_AUDIT actual_frame={frame + 4} stage_frame={frame}' in text
    if case.endswith('-paid'):
        assert 'exact285→0, next145 unaffordable' in text
    else:
        assert 'three real touch choices2/1/0' in text
assert load('units.json')['touch-continuation']['terminal_observed'] == {'ActiveState':'inactive','Result':'success','ExecMainStatus':0}
assert (ROOT / 'raw/touch-continuation/binary.sha256').read_text().strip() == load('local-bulk-manifest.json')['touch-continuation/capture']['sha256']
print('PASS exact archive, lossless logs, source maps, 773 tests, strict checks, 16 final native cases/4040 updates, logged geometry, real touch settlement and 26 selected originals')
