#!/usr/bin/env python3
"""Portable bounded native counters/inference; not global gameplay equivalence."""
import ast
from collections import Counter
from decimal import Decimal
import gzip,hashlib,json,math,re,struct
from pathlib import Path
D=Path(__file__).resolve().parent
load=lambda n:json.loads((D/n).read_text())
sha=lambda b:hashlib.sha256(b).hexdigest()
manifest=load('SHA256.json');assert set(manifest)=={str(p.relative_to(D))for p in D.rglob('*')if p.is_file()and p.name!='SHA256.json'}
assert '.gitattributes'in manifest
for n,h in manifest.items():assert sha((D/n).read_bytes())==h,n
logs={}
for row in load('raw-log-manifest.json'):
 data=gzip.decompress((D/row['archive']).read_bytes());assert len(data)==row['raw_bytes']and sha(data)==row['raw_sha256']
 logs[row['archive'].removeprefix('logs/').removesuffix('.gz')]=re.sub(r'\x1b\[[0-9;]*m','',data.decode())
a,b=load('before-inputs.json'),load('after-inputs.json');assert len(a)==391 and len(b)==392
assert {k for k in set(a)|set(b)if a.get(k)!=b.get(k)}=={'src/game/combat.rs','src/game/combat/ranged_release.rs','src/game/combat/ranged_release_tests.rs','src/game/combat/native_aim_tests.rs'}
assert b==load('after-inputs-after.json')
for stage,inputs,names in [('before',a,['combat.rs','combat/ranged_release_tests.rs']),('after',b,['combat.rs','combat/ranged_release.rs','combat/ranged_release_tests.rs'])]:
 for name in names:assert sha((D/(stage+'-'+name.replace('/','-'))).read_bytes())==inputs['src/game/'+name]
assert sha((D/'source/after-native_aim_tests.rs').read_bytes())==b['src/game/combat/native_aim_tests.rs']
assert load('before-result.json')['returncode']==101 and '0 passed; 1 failed;'in logs['before-test.log']
assert 'real native projectile emitted before candidate phase'in logs['before-test.log']
for n in ('focused-tests','library-tests','build','clippy'):assert load(n+'-result.json')['returncode']==0
assert '14 passed; 0 failed;'in logs['focused-tests.log'] and '756 passed; 0 failed; 1 ignored;'in logs['library-tests.log']
for n in ('before-unit-state.txt','before-trace-unit-state.txt','after-unit-state.txt','supplement-unit-state.txt'):
 text=(D/n).read_text();assert 'Result=success'in text and 'ExecMainStatus=0'in text
for folder in ('r1-immediate-fixture-failures','r2-elite-radius-fixture-failure','r3-visual-age-candidate','r4-large-frame-metadata-fixture-failures'):assert load(folder+'/reason.json')
assert '741 passed; 11 failed;'in logs['r1-immediate-fixture-failures/library-tests.log']
assert load('r2-elite-radius-fixture-failure/focused-tests-result.json')['returncode']!=0
assert '755 passed; 0 failed; 1 ignored;'in logs['r3-visual-age-candidate/library-tests.log']
assert '750 passed; 6 failed;'in logs['r4-large-frame-metadata-fixture-failures/library-tests.log']
N=r'[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?'
def read(name,updates=210):
 text=logs[name+'.log'];receipt=load(name+'-result.json');assert receipt['returncode']==0
 assert str(updates)in receipt['command'] and f'wrote {updates} simulation frames to 'in text
 assert 'PASS: real basic attack damage'in text and not re.search(r'ERROR|panicked|Validation Error',text)
 footer=re.findall(r'all (\d+) screenshot\(s\) saved after (\d+) wait frame\(s\)',text);assert len(footer)==1
 phases=[];timing=[];shots=[];hits=[];pending=[]
 for line in text.splitlines():
  if 'PROJECTILE_HIT'in line:
   fields=dict(re.findall(rf'(?<!\w)(\w+)=({N})(?=\s|$)',line));profile=re.search(r'profile=(.*?) base_damage=',line);assert profile
   hit=dict(frame=None,fields=fields,profile=profile[1]);hits.append(hit);pending.append(hit)
  h=re.match(r'^(?:RELEASE_PHASE|ATTACK_TIMING|HIT_PUNCH_FIRST) frame=(\d+)\b',line)
  if h:
   for hit in pending:hit['frame']=int(h[1])
   pending=[]
  m=re.fullmatch(r'RELEASE_PHASE frame=(\d+) hero=(\w+) clips=(.*)',line)
  if m:phases.append((int(m[1]),m[2],ast.literal_eval(m[3]),line))
  if line.startswith('ATTACK_TIMING '):timing.append(line)
  m=re.fullmatch(r'PROJECTILE_RELEASE frame=(\d+) entity=\S+ added_tick=\d+ hero=(\w+) position=Vec3\((.*?)\) velocity=Vec3\((.*?)\) clips=(.*)',line)
  if m:shots.append(dict(frame=int(m[1]),kind=m[2],position=[float(x)for x in m[3].split(',')],velocity=[float(x)for x in m[4].split(',')],clips=ast.literal_eval(m[5])))
 assert [x[0]for x in phases]==list(range(1,211)) and len(timing)==80
 return dict(phases=phases,timing=timing,shots=shots,hits=hits,unassigned_tail=pending,wait=int(footer[0][1]))
report={}
groups=dict(damage=['base_damage','scaled_base_damage','raw_damage','final_damage'],critical=['critical_roll_bits','crit_multiplier'],rng=['rng_before_crit','rng_after_crit','rng_after_resolution'],hp=['hp_before_bits','hp_after_bits','hp_damage'])
for kind,emitted,n_before,n_after in [('ember',24,24,23),('storm',13,13,13)]:
 old,new=read('before-trace-'+kind),read('after-trace-'+kind)
 assert len(old['shots'])==len(new['shots'])==emitted and len(old['hits'])==n_before and len(new['hits'])==n_after
 assert not old['unassigned_tail']and not new['unassigned_tail']
 shared=min(n_before,n_after)
 diffs={field:sum(x['fields'].get(field)!=y['fields'].get(field)for x,y in zip(old['hits'],new['hits']))for field in set().union(*(h['fields']for h in old['hits']+new['hits']))}
 for keys in groups.values():assert all(diffs[k]==0 for k in keys)
 assert all(x['profile']==y['profile']for x,y in zip(old['hits'],new['hits']))
 report[kind]=dict(shared_prefix=shared,nonzero_numeric_field_difference_counts={k:v for k,v in diffs.items()if v},profiles_equal_in_shared_prefix=True)
 if kind=='ember':
  last=new['shots'][-1];assert last['frame']==205 and last['position']==[-1.9399614,.25,0.]and last['velocity']==[18.44,0.,0.]
  upper=last['position'][0]+last['velocity'][0]/30*(210+new['wait']-last['frame'])
  assert new['wait']==1 and upper<3-.48-.48 and new['hits'][-1]['frame']==203
  report['ember_horizon']=dict(last_release=205,maximum_x_through211=upper,nearest_surface_x=2.04,first_possible_fullstep=205+math.ceil((2.04-last['position'][0])/(18.44/30)),scope='Source/log upper bound under stationary1x fixture, not directly observed frame210 Transform; no extrapolated impact counted')
for name in ('ember','storm','sword'):read(name)
sword=read('sword');control=logs['control-before-sword.log'];assert load('control-before-sword-result.json')['returncode']==0
for header,rows in [('ATTACK_TIMING',sword['timing']),('RELEASE_PHASE',[p[3]for p in sword['phases']])]:assert Counter(rows)==Counter(l for l in control.splitlines()if l.startswith(header+' '))
assert not sword['shots']
for stage,kind in __import__('itertools').product(('before','after'),('ember','storm')):read(stage+'-portrait-'+kind,250)
# Independently recompute all conditional inferred source times; no authored-event claim.
inference=load('phase-inference.json')
for label,item in inference['runs'].items():
 stage,kind=label.rsplit('-',1);name=('before-trace-'+kind if stage=='old'else 'r3-visual-age-candidate/'+kind if stage=='r3'else kind)
 run=read(name);values=[]
 for shot,stored in zip(run['shots'],item['inference']['rows']):
  speed=math.sqrt(sum(v*v for v in shot['velocity']));distance=math.sqrt(sum((x-y)**2 for x,y in zip(shot['position'],(-2.,.25,0.))))
  attack=[c for c in shot['clips']if c[0]==3];assert len(attack)==1
  value=attack[0][1]-distance/speed*attack[0][2];assert stored['eligible']and abs(value-stored['inferred_source_release_seconds'])<1e-12
  values.append(value)
 assert len(values)==len(item['inference']['rows']) and item['inference']['spread']==dict(min=min(values),max=max(values),range=max(values)-min(values))
expected={'frames/ember/frame00014.png','frames/storm/frame00020.png','frames/before-portrait-ember/frame00014.png','frames/after-portrait-ember/frame00014.png','frames/before-portrait-storm/frame00020.png','frames/after-portrait-storm/frame00020.png','frames/after-portrait-storm/frame00023.png'}
assert expected=={str(p.relative_to(D))for p in(D/'frames').rglob('*.png')}
for n in expected:
 data=(D/n).read_bytes();assert struct.unpack('>II',data[16:24])==((720,1280)if 'portrait'in n else(1280,720))
print(json.dumps(dict(result='PASS',library=[756,0,1],review=report,selected_pngs=7,limits='Boundedfixtureonly; changedframes/progress/elapsed preserved, no global RNG/endpointHP/hardware/artist release event claim'),indent=2))
