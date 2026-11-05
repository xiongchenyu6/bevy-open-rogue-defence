from pathlib import Path
import json
p=Path(__file__).resolve().parent
assert '681 passed; 0 failed; 1 ignored' in (p/'full-tests.log').read_text()
build=json.loads((p/'build-process.json').read_text());assert len(build)==3 and all(x['exit']==0 for x in build)
cases=json.loads((p/'native-process.json').read_text());assert len(cases)==5 and sum(x['count']for x in cases)==910
for case in cases:
 assert case['passed'] and case['exit']==0
 text=(p/case['name']/'capture.log').read_text()
 assert 'panicked at' not in text and f'wrote {case["count"]} simulation frames' in text
 marker='PASS equipment:' if case['name']=='owned-regression' else 'EQUIPMENT_EMPTY_PASS'
 assert marker in text
missing=(p/'missing-recovery/capture.log').read_text()
assert 'EQUIPMENT_EMPTY_MISSING_PASS actual image load failed' in missing
assert 'frame=125 actual image ready and visible' in missing
assert 'PASS original empty armory:' in (p/'bitmap-check.log').read_text()
print('PASS archived empty armory R2: 681 tests, strict Clippy/build, 5 native cases/910 frames, actual missing-image fallback and reload; controlled fixture only')
