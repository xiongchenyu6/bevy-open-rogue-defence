import gzip,hashlib,json,re,sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
assert json.loads(raw['before/result.json'])['returncode']==101 and b'0 passed; 1 failed' in raw['before/tests.log']
for n in ['result.json','library-result.json','clippy-result.json','sim-build-result.json']:assert json.loads(raw['after/'+n])['returncode']==0
assert b'4 passed; 0 failed' in raw['after/tests.log'] and b'788 passed; 0 failed; 1 ignored' in raw['after/library.log']
pat=r'RICOCHET_GEOMETRY reverse=(true|false).*?destination=Vec3\(([^)]+)\) velocity=Vec3\(([^)]+)\) hp=\[([^]]+)\] rng=(\d+)'
def values(stage):
 rows=re.findall(pat,raw[stage+'/tests.log'].decode());assert len(rows)==2
 return {rev:(tuple(map(float,pos.split(','))),tuple(map(float,vel.split(','))),tuple(map(float,hp.split(','))),int(rng)) for rev,pos,vel,hp,rng in rows}
b=values('before');a=values('after');rng=1914567331323306758
assert b['false']==((2,0,2),(0,0,20),(900,1000,925),rng)
assert b['true']==((2,0,-2),(0,0,-20),(900,925,1000),rng)
assert a['false']==a['true']==((2,0,-2),(0,0,-20),(900,925,1000),rng)
mp=json.loads(raw['after/inputs.json']);assert mp==json.loads((D/'inputs-observed-after.json').read_text())
prov=json.loads((D/'provenance.json').read_text());assert len(prov['sources'])==2
for n,h in prov['sources'].items():assert sha((D/'source'/n).read_bytes())==mp[n]==h
assert sha(raw['before/collision_order_tests.rs'])==json.loads(raw['before/inputs.json'])['src/game/combat/collision_order_tests.rs']
state=(D/'unit-state.txt').read_text();assert 'Result=success' in state and 'ExecMainStatus=0' in state
print('PASS exact archive/raw bytes,actual pooled ricochet negative,final4/788+1/Clippy/build and geometric HP/RNG variants.')
