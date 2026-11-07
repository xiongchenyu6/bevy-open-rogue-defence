import argparse,gzip,hashlib,json,re,struct
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('archive',type=Path);p.add_argument('--local-root',type=Path);args=p.parse_args();D=args.archive.resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
extract={}
for n,x in json.loads((D/'extraction-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['uncompressed_sha256'] and len(b)==x['uncompressed_bytes'];assert len(b.splitlines())==x['selected_rows'];extract[n]=b
ANSI=re.compile(r'\x1b\[[0-9;]*m');IDS={'projectile','projectile_bits','enemy','enemy_bits'}
def contacts(label):
 out=[]
 for line in extract[f'extracted/{label}-contacts.gz'].decode().splitlines():
  body=ANSI.sub('',line).split('PROJECTILE_HIT ',1)[1];fields=list(re.finditer(r'(?<!\S)([a-z_]+)=',body));row={x[1]:body[x.end():fields[i+1].start() if i+1<len(fields) else len(body)].strip() for i,x in enumerate(fields)}
  out.append({k:v for k,v in row.items() if k not in IDS})
 return out
a,b=contacts('a'),contacts('b');assert len(a)==len(b)==10218 and a==b
assert all('outer_ordinal' in x and 'candidate_ordinal' in x and 'rng_after_resolution' in x for x in a)
def no_root(x):
 if isinstance(x,dict):return {k:no_root(v) for k,v in x.items() if k!='root'}
 if isinstance(x,list):return [no_root(v) for v in x]
 return x
ordinary=[[json.loads(l) for l in extract[f'extracted/{label}-ordinary.gz'].decode().splitlines()] for label in 'ab'];assert list(map(len,ordinary))==[202,202] and no_root(ordinary[0])==no_root(ordinary[1])
assert [sum(x['type']=='checkpoint' for x in rows) for rows in ordinary]==[54,54]
def projection(row):
 out={k:v for k,v in row.items() if k!='enemy_order'}
 for field in ['enemies','projectiles','hero_positions']:out[field]=sorted(actor[1:] for actor in row[field])
 return out
report=json.loads((D/'comparison/comparison.json').read_text());assert report['full_recorded_match'] is True and report['recorded_numeric_equal'] is True
assert report['actor_counts']==[93642,93642] and report['contact_counts']==[10218,10218] and report['ordinary_counts']==[202,202]
assert report['first_numeric'] is None and report['first_contact_nonidentity'] is None and report['first_ordinary_numeric'] is None
assert report['first_identity'] is not None and report['first_ordinary_identity'] is not None
assert json.loads((D/'comparison/first-numeric-context.json').read_text())is None
for label in 'ab':
 case=json.loads((D/f'summary-{label}.json').read_text())['cases'][0];assert case['returncode']==0 and case['valid'] and not case['censored'] and not case['timed_out'] and case['outcome']=='win';assert case['last']['frame']==93642 and case['last']['elapsed']==1508.875
assert all(x['returncode']==0 for x in json.loads((D/'pair-processes.json').read_text()))
mp=json.loads((D/'compiled-inputs.json').read_text());assert len(mp)==211
assert sha((D/'class_mechanics.rs').read_bytes())==mp['src/game/basic_attacks/class_mechanics.rs'];assert sha((D/'collision_order_tests.rs').read_bytes())==mp['src/game/combat/collision_order_tests.rs']
state=(D/'unit-state.txt').read_text();assert 'Result=success' in state and 'ExecMainStatus=0' in state
if args.local_root:
 for name,x in json.loads((D/'local-bulk-manifest.json').read_text()).items():
  f=args.local_root/name;h=hashlib.sha256()
  with f.open('rb') as src:
   for chunk in iter(lambda:src.read(1024*1024),b''):h.update(chunk)
  assert f.stat().st_size==x['bytes'] and h.hexdigest()==x['sha256'],name
 print('PASS read-only local bulk hashes; no simulator launched')
print('PASS portable bytes,202 ordinary/10218 contacts per case,retained identity differences and recorded full-match true for this pair.')
