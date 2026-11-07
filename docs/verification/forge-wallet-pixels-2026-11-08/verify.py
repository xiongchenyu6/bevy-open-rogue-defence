import gzip,hashlib,json,re,sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
assert json.loads(raw['tests-and-calibration/tests-result.json'])['returncode']==0
assert re.search(rb'Ran 121 tests',raw['tests-and-calibration/tests.log']) and re.search(rb'\nOK\s*$',raw['tests-and-calibration/tests.log'])
assert json.loads(raw['natural-r4/forge-process.json'])['returncode']==1
old=json.loads(raw['natural-r4/forge/result.json']);assert old['purchases']==[] and old['pending_purchase']['price']==180 and old['pending_purchase']['wallet_before']==297
prov=json.loads((D/'provenance.json').read_text());assert len(prov['sources'])==3
for n,h in prov['sources'].items():assert sha((D/'source'/n).read_bytes())==h
assert sha((D/'purchase-117.png').read_bytes())==prov['fixture_sha256']
cal=[json.loads(x) for x in raw['tests-and-calibration/calibration.jsonl'].decode().splitlines()];assert any(x['case']=='new117' and x['bounds']==[163,180] and x['reads']==['17','7'] for x in cal)
thresholds=[json.loads(x) for x in raw['tests-and-calibration/thresholds.jsonl'].decode().splitlines()];assert any(x['case']=='new117' and x['threshold'] is None and x['reads']==['117','117'] for x in thresholds)
print('PASS exact archive/raw bytes, failed natural pending purchase,121 tests,3 source snapshots and actual117 fixture.')
