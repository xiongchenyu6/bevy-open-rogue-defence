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
assert json.loads(raw['natural-r5/forge-process.json'])['returncode']==1
old=json.loads(raw['natural-r5/forge/result.json']);assert old['acquired_materials']==['Arc Lens']
assert len(old['purchases'])==1
p=old['purchases'][0];assert (p['name'],p['price'],p['wallet_before'],p['wallet_after'])==('Arc Lens',180,291,111)
assert b"[(100, '7'), (200, ''), (600, '17:15')]" in raw['natural-r5/forge.log']
prov=json.loads((D/'provenance.json').read_text());assert len(prov['sources'])==2 and prov['base_commit']=='d0a10ea79bdfc6a2452cffe8e1ff88d6e88ee447'
for n,h in prov['sources'].items():assert sha((D/'source'/n).read_bytes())==h
assert sha((D/'failure-market-open.png').read_bytes())==prov['image_sha256']
assert gzip.decompress((D/'source.patch.gz').read_bytes()).startswith(b'diff --git ')
print('PASS exact archive/raw bytes,122 tests,Arc Lens exact debit/ownership receipt and later strict-clock route interruption.')
