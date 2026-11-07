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
assert re.search(rb'Ran 119 tests',raw['tests-and-calibration/tests.log']) and re.search(rb'\nOK\s*$',raw['tests-and-calibration/tests.log'])
assert json.loads(raw['natural-r3/forge-process.json'])['returncode']==1
old=json.loads(raw['natural-r3/forge/result.json']);assert old['purchases']==[] and len(old['metric_observations'])==16
assert all(x['state']=='unknown' for x in old['metric_observations'])
prov=json.loads((D/'provenance.json').read_text());assert len(prov['sources'])==2
for n,h in prov['sources'].items():assert sha((D/'source'/n).read_bytes())==h
assert sha((D/'unknown-hud.png').read_bytes())==prov['fixtures']['existing_r2_png']
assert sha((D/'r3-last.png').read_bytes())==prov['fixtures']['r3_last_png']
assert prov['fixtures']['existing_r2_png']!=prov['fixtures']['r3_last_png']
cal=[json.loads(x) for x in raw['tests-and-calibration/calibration.jsonl'].decode().splitlines()];assert len(cal)==48
chosen=[x for x in cal if x['crop']=='90x20+20+82' and x['scale'] in [300,400] and x['method']=='gray'];assert len(chosen)==4
assert all(re.fullmatch(r'LIVES\s*3',x['text']) for x in chosen)
full=json.loads(raw['tests-and-calibration/full-classification.json']);assert len(full)==2 and all(x['state']=='battle' and x['rows'] for x in full)
print('PASS exact archive/raw bytes,16 unknown observations,48 calibrations,119 tests and2 distinct real HUD fixtures.')
