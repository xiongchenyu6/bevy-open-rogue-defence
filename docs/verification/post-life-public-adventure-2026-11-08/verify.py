import gzip,hashlib,json,re,sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
host='060649ac32b34ce2cda231a9cf1ebcdfd8e8c12f0b0b3f94e527714f6993d911'
for i in range(1,8):
 label='r'+str(i);p=json.loads(raw[label+'/process.json']);v=json.loads(raw[label+'/provenance.json']);assert p['returncode']==(137 if i in [3,5,7] else 1)
 assert p['host_profile_after']==v['host_profile_before']==host and p['host_profile_unchanged'] and p['harness_unchanged']
 assert v['version']['git_commit']=='df51a6cddc908e5358f2cac9fa07068f46a6a5cc' and v['version']['binary_sha256']=='8d69ed5dec8fb555e6d605eb553ce1e12f0c7c38439a8d305a69564cff05e0d9'
 for n,h in v['harness_files'].items():assert sha(raw[label+'/chapter6/harness-source/'+n])==h
 if i in [3,5,7]:assert json.loads(raw[label+'/controlled-stop.json'])['reason']
 else:assert b'AssertionError: Camp choice did not open' in raw[label+'/chapter6.log']
stop=json.loads(raw['r7/controlled-stop.json']);assert stop['evidence']=='chapter6/run-094.png'
assert sha((D/'frames/r7/run-094.png').read_bytes())==stop['evidence_sha256']
assert 'r7/chapter6/result.json' not in raw and 'r7/chapter6/latest-report.json' not in raw
assert json.loads(raw['r7/clock-tests-process.json'])['returncode']==0
assert b'Ran 11 tests' in raw['r7/clock-tests.log'] and b'OK (skipped=1)' in raw['r7/clock-tests.log']
pending=json.loads((D/'pending-helper/provenance.json').read_text());assert len(pending['sources'])==2
for n,h in pending['sources'].items():assert sha((D/'pending-helper/source'/n).read_bytes())==h
images=json.loads((D/'original-image-index.json').read_text());assert len(images)==609
for f in (D/'frames').rglob('*.png'):
 relative=f.relative_to(D/'frames');key=str(Path(relative.parts[0])/'chapter6'/Path(*relative.parts[1:]));assert sha(f.read_bytes())==images[key]['sha256']
review=json.loads((D/'review-mapping.json').read_text());assert len(review)==9
for row in review:assert (D/row['original']).exists() and (D/row['jpeg']).exists()
assert not any(n.endswith('.save') for n in m)
print('PASS exact archive/raw bytes,seven partial-route receipts,controlled relic-image origin,helper pins and11-run clock tests.')
