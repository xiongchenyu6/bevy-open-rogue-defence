import gzip,hashlib,json,re,sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
manifest=json.loads((D/'SHA256.json').read_text())
assert set(manifest)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for name,h in manifest.items():assert sha((D/name).read_bytes())==h,name
plan=json.loads((D/'collection-plan.json').read_text());data={}
for item in plan['files']:
 b=(D/item['archive']).read_bytes();b=gzip.decompress(b) if item['compressed'] else b
 assert len(b)==item['bytes'] and sha(b)==item['sha256'];data[item['source']]=b
BASE='target/verification/decision-phase-2026-11-07/'
def raw(name):return data[BASE+name].decode()
def obj(name):return json.loads(raw(name))
for stage in ['focused','library-tests','release-clippy','bin-tests','capture-build','capture-clippy','native-1280x720-en','native-360x800-zh','sim-build','replay-a','replay-b']:
 assert obj('after-r3/'+stage+'-result.json')['returncode']==0,stage
assert re.search(r'4 passed; 0 failed',raw('after-r3/focused.log'))
assert re.search(r'786 passed; 0 failed; 1 ignored',raw('after-r3/library-tests.log'))
assert re.search(r'6 passed; 0 failed',raw('after-r3/bin-tests.log'))
assert obj('before-capacity/result.json')['returncode']==101
assert obj('before-r3/result.json')['returncode']==0
assert obj('before/result.json')['returncode']==101 and obj('before-r2/result.json')['returncode']==101
assert obj('after-r1/focused-result.json')['returncode']==101
inputs=obj('after-r3/inputs.json');assert len(inputs)==401 and inputs==obj('after-r3/inputs-after.json')
for key,b in data.items():
 prefix=BASE+'after-r3/source/'
 if key.startswith(prefix):assert sha(b)==inputs[key[len(prefix):]],key
identity={'root','entity','projectile','enemy','projectile_bits','enemy_bits'}
def projection(x):
 if isinstance(x,dict):return {k:projection(v) for k,v in x.items() if k not in identity}
 if isinstance(x,list):return [projection(v) for v in x]
 return x
pairs=[];counts=[]
for label in 'ab':
 name='target/verification/picker-order-2026-11-07/frozen/target/verification/decision-phase-after-r3-'+label+'/active-h1-behind-s1-c0-t0.jsonl'
 rows=[json.loads(l) for l in data[name].decode().splitlines() if l.startswith('{')];pairs.append(rows)
 pending=None;matched=0
 for row in rows:
  if row['type']=='picker_entry':pending=row
  elif row['type']=='picker_exit' and not row['offered']:pending=None
  elif row['type']=='draft_offer' and pending is not None:
   assert pending['capacity']==row['fusion_capacity'];matched+=1;pending=None
 assert matched==20;counts.append(matched)
entries=[[r for r in rows if r['type']=='picker_entry'] for rows in pairs]
assert list(map(len,entries))==[20,20]
assert any(projection(a)!=projection(b) for a,b in zip(entries[0],entries[1])),'Known repeat mismatch must remain visible'
state=raw('after-r3/unit-state.txt');assert 'Result=success' in state and 'ExecMainStatus=0' in state
print('PASS exact archive/raw bytes, actual test receipts, 401 source consensus, 40 capacity joins. Known replay divergence retained.')
