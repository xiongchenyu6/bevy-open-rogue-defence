from pathlib import Path
import hashlib,json,re
root=Path(__file__).resolve().parent
def read(name):return json.loads((root/name).read_text())
for name,digest in read('evidence-hashes.json').items():assert hashlib.sha256((root/name).read_bytes()).hexdigest()==digest,name
assert read('source-before.json')==read('source-after.json')
for stage in ('format','bin-tests','clippy','build','snapshot-root-smoke'):assert read(stage+'-process.json')['exit']==0,stage
assert '6 passed; 0 failed' in (root/'bin-tests.log').read_text()
assert read('result.json')['passed'] is True
asset=read('asset-manifest.json')['files'];assert len(asset)==185 and sum(r['file'].endswith('.glb') for r in asset)==52
assert len({r['file'] for r in asset})==185
assert 'SIM_ASSET_ROOT_FREEZE_PASS' in (root/'sequence.log').read_text()
text=(root/'snapshot-root-smoke.log').read_text();assert 'panicked at' not in text and 'ERROR' not in text
assert 'BALANCE_ASSET_ROOT '+read('result.json')['asset_root'] in text.splitlines()
rows=[]
for line in text.splitlines():
 try:rows.append(json.loads(line))
 except json.JSONDecodeError:pass
last=[r for r in rows if r.get('type')=='checkpoint'][-1]
assert last['frame']==1200 and last['paused'] is False and last['finished'] is False
assert read('missing-root-rejected-process.json')['exit']==101
assert 'simulation asset root is not a directory' in (root/'missing-root-rejected.log').read_text()
assert read('initial-compile-failure/bin-tests-process.json')['exit']==101
assert 'cannot find module or crate `serde_json`' in (root/'initial-compile-failure/bin-tests.log').read_text()
print('PASS simulator frozen-root proof: 6 tests, strict Clippy/build, 1200-update startup, invalid root rejected; no full-game claim')
