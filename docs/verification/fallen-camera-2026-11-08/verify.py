import argparse,gzip,hashlib,json,re,shutil,subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('archive',type=Path);p.add_argument('--pixels',action='store_true');args=p.parse_args();D=args.archive.resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
assert json.loads(raw['before/result.json'])['returncode']==101 and b'0 passed; 1 failed' in raw['before/tests.log']
for n in ['result.json','library-result.json','clippy-result.json','sim-build-result.json','capture-build-result.json','native-result.json']:assert json.loads(raw['after/'+n])['returncode']==0
assert b'1 passed; 0 failed' in raw['after/tests.log'] and b'792 passed; 0 failed; 1 ignored' in raw['after/library.log']
pat=r'FALLEN_CAMERA before=Vec3\(([^)]+)\) fallen=Vec3\(([^)]+)\)'
b=re.search(pat,raw['before/tests.log'].decode());a=re.search(pat,raw['after/tests.log'].decode());assert b and a
assert tuple(map(float,b[1].split(',')))==(-21.899813,25.2,10.791589)
assert tuple(map(float,b[2].split(',')))==(-1009.1186,25.2,422.13278)
assert a[1]==a[2]==b[1]
prov=json.loads((D/'provenance.json').read_text());mp=json.loads(raw['after/inputs.json']);assert sha((D/'source'/prov['source']).read_bytes())==prov['sha256']==mp[prov['source']]
assert sha(raw['before/camera.rs'])==json.loads(raw['before/inputs.json'])[prov['source']]
assert raw['after/host-profile-before.sha256'].strip()==raw['after/host-profile-after.sha256'].strip()==b'060649ac32b34ce2cda231a9cf1ebcdfd8e8c12f0b0b3f94e527714f6993d911'
assert len(json.loads(raw['after/isolated-environment.json']))==4
assert b'wrote 400 simulation frames' in raw['after/native.log'] and b'third death settles defeat' in raw['after/native.log']
review=json.loads(raw['after/root-reviewed.json']);assert len(review['frames'])==7
for n in review['frames']:assert (D/'frames/after/native'/n).exists()
pixels=json.loads(raw['after/pixel-comparison.json']);assert len(pixels)==4
for row in pixels:
 f=D/'visual-before'/row['frame'] if row['label']=='before' else D/'frames/after/native'/row['frame'];assert sha(f.read_bytes())==row['sha256']
 if args.pixels:
  tool=shutil.which('magick') or shutil.which('convert');assert tool
  value=float(subprocess.check_output([tool,str(f),'-crop',row['roi'],'+repage','-colorspace','Gray','-format','%[fx:standard_deviation]','info:'],text=True));assert abs(value-row['gray_standard_deviation'])<=.000001
state=(D/'unit-state.txt').read_text();assert 'Result=success' in state and 'ExecMainStatus=0' in state
if args.pixels:print('PASS independent before/after ROI standard deviations')
print('PASS exact archive/raw bytes,actual camera negative,final1/792+1/Clippy/native,source and isolated host hashes.')
