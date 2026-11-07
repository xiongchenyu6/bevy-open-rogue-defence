"""Portable numerical-integration evidence verifier."""
import gzip,hashlib,json,re,sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def j(name):return json.loads((D/name).read_text())
def log(name):return gzip.decompress((D/'raw'/(name+'.log.gz')).read_bytes()).decode()
def digest(data):return hashlib.sha256(data).hexdigest()
m=j('SHA256.json');assert set(m)=={str(p.relative_to(D)) for p in D.rglob('*') if p.is_file() and p.name!='SHA256.json'}
for name,value in m.items():assert digest((D/name).read_bytes())==value,name
for name,row in j('raw-logs.json').items():
 data=gzip.decompress((D/name).read_bytes());assert len(data)==row['bytes'] and digest(data)==row['sha256']
assert j('raw/red-result.json')['returncode']==101
red=log('red');assert '0 passed; 1 failed;' in red
rows=re.findall(r'ARROW_NATIVE kind=EmberRanger card=(false|true) loss=([0-9.]+) profiles=(.*?) rng=(\d+)',red)
assert len(rows)==2 and {r[0] for r in rows}=={'false','true'}
assert all(float(r[1])==52.796875 and 'arrow: false' in r[2] for r in rows) and rows[0][3]==rows[1][3]
for name in ('focused','library','clippy'):assert j('raw/r3-'+name+'-result.json')['returncode']==0
assert j('raw/r3-inputs.json')==j('raw/r3-inputs-after.json')
inputs=j('raw/r3-inputs.json')
for path in (D/'raw').glob('r3-src-*.rs'):
 name=path.name.removeprefix('r3-').removesuffix('.rs').replace('-','/')+'.rs'
 # Module filenames contain underscores, which remain unchanged.
 assert digest(path.read_bytes())==inputs[name],name
focused=log('r3-focused')
for prefix in ('ARROW_NATIVE','ARROW_FUSION'):
 for kind,bonus in [('EmberRanger',1.15),('GroveArtificer',1.15),('StormSavant',1.0)]:
  pairs=re.findall(prefix+r' kind='+kind+r' card=(false|true) loss=([0-9.]+) (.*?) rng=(\d+)',focused)
  assert len(pairs)==2 and {r[0] for r in pairs}=={'false','true'},(prefix,kind)
  values={r[0]:float(r[1]) for r in pairs}
  assert abs(values['true']-values['false']*bonus)<.03,(prefix,kind,values)
  assert all(('arrow: true' in r[2])==(bonus>1) for r in pairs)
  assert pairs[0][3]==pairs[1][3],(prefix,kind,'RNG')
pairs=re.findall(r'ARROW_PIERCING card=(false|true) loss=([0-9.]+) (.*?) critical=(.*?) rng=(\d+) preview=([0-9.]+)',focused)
assert len(pairs)==2
values={r[0]:float(r[1]) for r in pairs};assert abs(values['true']-values['false']*1.15)<.04
assert pairs[0][3:5]==pairs[1][3:5]
assert all('arrow: true' in r[2] and 'skill_index: Some(7)' in r[2] for r in pairs)
assert re.search(r'test result: ok\. \d+ passed; 0 failed;',log('r3-library'))
print('PASS red reproduction and bounded native/Fusion/skill7 numerical integration')

ansi=re.compile(r'\x1b\[[0-9;]*m')
results={}
for label in ('desktop','phone'):
 for card in (0,1):
  name=f'r3-{label}-card{card}'
  assert j('raw/'+name+'-result.json')['returncode']==0
  text=log(name);assert 'wrote 360 simulation frames' in text and 'PASS skill-identity-7' in text
  contacts=[]
  for line in text.splitlines():
   if 'PROJECTILE_HIT ' not in line:continue
   body=ansi.sub('',line).split('PROJECTILE_HIT ',1)[1];fields=list(re.finditer(r'(?<!\S)([a-z_]+)=',body))
   row={m[1]:body[m.end():fields[i+1].start() if i+1<len(fields) else len(body)].strip() for i,m in enumerate(fields)}
   if 'skill_index: Some(7)' not in row['profile'] or 'card_proc: false' not in row['profile']:continue
   assert 'arrow: true' in row['profile']
   base=float(row['base_damage']);scaled=float(row['scaled_base_damage']);assert base>0
   assert abs(scaled/base-(1.265 if card else 1.1))<1e-5
   contacts.append(row)
  assert len(contacts)==70 and any(float(row['crit_multiplier'])==1 for row in contacts)
  observed=re.search(r'ARROW_CARD_OBSERVATION frame=295 card=(false|true) attacks=(\d+) base_attack=(\S+) learned_rank=(\d+) caster_every=4',text)
  assert observed and observed[1]==str(bool(card)).lower() and int(observed[2])==24 and observed[3]=='262.11884' and observed[4]=='1'
  # Same-card viewport comparisons retain every numeric field and profile,
  # excluding only entity identities and outer iteration ordinal.
  results[label,card]=[{k:v for k,v in row.items() if k not in ('projectile','projectile_bits','enemy','enemy_bits','outer_ordinal')} for row in contacts]
for card in (0,1):assert results['desktop',card]==results['phone',card]
assert j('raw/r3-library-inputs.json')=={k:v for k,v in inputs.items() if not k.startswith('src/bin/')}
assert j('raw/r3-capture-inputs.json')==inputs
assert j('raw/r3-capture-build-result.json')['returncode']==0
state=(D/'unit-state.txt').read_text();assert 'Result=success' in state and 'ExecMainStatus=0' in state
provenance=j('source-provenance.json')
assert set(provenance['production_diffs'])=={'src/game/bonds.rs','src/game/combat/ranged_release.rs','src/game/hero_fusion.rs'}
for name,row in (provenance['production_diffs'] | provenance['test_support_diffs']).items():
 assert digest((D/'source-before'/name).read_bytes())==row['before']
 assert inputs[name]==row['after']
assert j('raw/after-r1-clippy-helper-placement/clippy-result.json')['returncode']!=0
assert j('raw/after-r2-capture-counter-access/r2-clippy-result.json')['returncode']!=0
print('PASS four native typed-contact scales and same-card viewport numeric comparisons')
