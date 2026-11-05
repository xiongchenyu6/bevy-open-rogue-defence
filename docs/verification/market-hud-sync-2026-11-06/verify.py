from pathlib import Path
import json,hashlib
p=Path(__file__).resolve().parent
for n,v in json.loads((p/'evidence-hashes.json').read_text()).items():assert hashlib.sha256((p/n).read_bytes()).hexdigest()==v,n
assert json.loads((p/'old-test-process.json').read_text())['exit']==101
old=(p/'old-test.log').read_text();assert 'left: "424"' in old and 'right: "280"' in old and '0 passed; 1 failed' in old
assert '688 passed; 0 failed; 1 ignored' in (p/'full-tests.log').read_text()
assert all(x['exit']==0 for x in json.loads((p/'build-process.json').read_text()))
cases=json.loads((p/'native-process.json').read_text());assert len(cases)==2 and all(x['exit']==0 and x['passed'] for x in cases)
for case in cases:
 text=(p/case['name']/'capture.log').read_text();assert 'PASS first purchase frame session=99700 HUD=99700' in text and 'PASS accessible close toggle' in text and 'wrote 200 simulation frames' in text
rows=json.loads((p/'native-pixels.json').read_text());assert [(r['hud'],r['header']) for r in rows]==[(100000,100000),(99700,99700),(99700,99700)]
print('PASS: archived hashes, actual old-order fail, fixed production tests, two nativefirstframe cases and ENpixels; no earned/fullchapter claim')
