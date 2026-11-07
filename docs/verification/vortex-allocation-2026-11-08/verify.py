import gzip,hashlib,json,re,sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
assert json.loads(raw['before/result.json'])['returncode']==101
assert b'1 passed; 1 failed' in raw['before/tests.log']
for n in ['result.json','library-result.json','clippy-result.json','sim-build-result.json']:assert json.loads(raw['after-r2/'+n])['returncode']==0
assert b'2 passed; 0 failed' in raw['after-r2/tests.log'] and b'787 passed; 0 failed; 1 ignored' in raw['after-r2/library.log']
pattern=r'VORTEX_ORDER reverse=(true|false) reallocate=(true|false).*?health=\[Some\(([-\d.]+)\), Some\(([-\d.]+)\)\] strikes=(\d+) pulse=([\d.]+) rng=(\d+)'
def variants(stage):
 rows=re.findall(pattern,raw[stage+'/tests.log'].decode());assert len(rows)==4
 return {(r,a):(float(x),float(y),int(s),float(p),int(g)) for r,a,x,y,s,p,g in rows}
b=variants('before');a=variants('after-r2');rng=16263334312487546840
assert b[('false','true')]==(825,-50,2,.5,rng) and b[('true','true')]==(900,-125,2,.5,rng)
assert b[('false','false')]==b[('true','false')]==(825,-50,2,.5,rng)
assert all(x==(900,-125,2,.5,rng) for x in a.values())
mp=json.loads(raw['after-r2/inputs.json']);assert len(mp)==211 and mp==json.loads((D/'inputs-observed-after.json').read_text())
prov=json.loads((D/'provenance.json').read_text());assert len(prov['source_snapshots'])==2
for n,x in prov['source_snapshots'].items():assert sha((D/'source'/n).read_bytes())==mp[n]==x['sha256']
assert sha(raw['before/collision_order_tests.rs'])==json.loads(raw['before/inputs.json'])['src/game/combat/collision_order_tests.rs']
diff=json.loads((D/'input-differences.json').read_text());assert len(diff['early_to_final_changed'])==4
state=(D/'unit-state.txt').read_text();assert 'Result=success' in state and 'ExecMainStatus=0' in state
print('PASS exact archive/raw bytes,actual allocation negative,final2/787+1/Clippy/build,HP/RNG variants and211 input consensus.')
