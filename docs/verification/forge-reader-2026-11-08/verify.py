import gzip,hashlib,json,re,sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for x in json.loads((D/'collection-plan.json').read_text())['files']:
 b=gzip.decompress((D/x['archive']).read_bytes());assert len(b)==x['bytes'] and sha(b)==x['sha256'];raw[x['source']]=b
assert json.loads(raw['tests-result.json'])['returncode']==0
assert re.search(rb'Ran 62 tests',raw['tests.log']) and re.search(rb'\nOK\s*$',raw['tests.log'])
assert json.loads(raw['capture-result.json'])['returncode']==0
prov=json.loads((D/'provenance.json').read_text());assert prov['native_inputs']==402 and prov['native_matches_frozen_r5'] is True
assert len(json.loads((D/'native-source-inputs.json').read_text()))==402
assert len(prov['python_files'])==3
for n,h in prov['python_files'].items():assert sha((D/'source'/n).read_bytes())==h
for n,x in prov['fixtures'].items():assert sha((D/'frames'/n).read_bytes())==x['sha256']
assert re.findall(rb'actual recipe (\w+):',raw['capture.log'])==[b'AstralPrism',b'Suncoil',b'WorldEngine']
assert b'actual recipe AstralPrism:' in raw['capture.log'] and b'wrote 300 simulation frames' in raw['capture.log']
print('PASS exact archive/raw bytes, actual62 Python tests,300-update native capture,3 consumer snapshots and2 actual PNG fixtures.')
