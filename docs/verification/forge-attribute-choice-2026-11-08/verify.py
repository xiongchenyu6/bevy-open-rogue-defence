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
assert re.search(rb'Ran 125 tests',raw['tests/tests.log']) and re.search(rb'\nOK\s*$',raw['tests/tests.log'])
assert json.loads(raw['replay/replay-result.json'])['returncode']==0
replay=json.loads(raw['replay/replay.log']);assert replay['recognized_choice'] is True and replay['all_rows']
assert json.loads(raw['natural-r9/forge-process.json'])['returncode']==1
old=json.loads(raw['natural-r9/forge/result.json']);assert old['crafted_inventory_ocr']=={'count':1,'names':['astralprism']}
prov=json.loads((D/'provenance.json').read_text());assert len(prov['sources'])==2
for n,h in prov['sources'].items():assert sha((D/'source'/n).read_bytes())==h
for n,h in prov['images'].items():assert sha((D/n).read_bytes())==h
assert gzip.decompress((D/'source.patch.gz').read_bytes()).startswith(b'diff --git ')
print('PASS exact archive/raw bytes,125 tests,R9 Prism ownership/choice interruption and actual choice-pixel replay.')
