import gzip,hashlib,json,re,sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
assert json.loads(raw['tests/tests-result.json'])['returncode']==0
assert re.search(rb'Ran 122 tests',raw['tests/tests.log']) and re.search(rb'\nOK\s*$',raw['tests/tests.log'])
assert json.loads(raw['replay/ready-replay-result.json'])['returncode']==0
replay=json.loads(raw['replay/ready-replay.log']);assert replay['retrospective_pixel_verification']=='PASS'
assert [x['name'] for x in replay['materials']]==['Arc Lens','Echo Core'] and all(x['status']=='¥ Owned' for x in replay['materials'])
assert replay['action']=='Forge equipment'
assert json.loads(raw['natural-r6/forge-process.json'])['returncode']==1
old=json.loads(raw['natural-r6/forge/result.json']);assert old['acquired_materials']==['Arc Lens','Echo Core']
assert [(x['price'],x['wallet_before'],x['wallet_after']) for x in old['purchases']]==[(180,304,124),(300,356,56)]
prov=json.loads((D/'provenance.json').read_text());assert len(prov['sources'])==2
for n,h in prov['sources'].items():assert sha((D/'source'/n).read_bytes())==h
for n,h in prov['images'].items():assert sha((D/n).read_bytes())==h
print('PASS exact archive/raw bytes, two actual owned purchases, failed readiness, retrospective real-pixel replay and final tests.')
