import argparse,gzip,hashlib,itertools,json,re
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('archive',type=Path);p.add_argument('--local-root',type=Path);args=p.parse_args();D=args.archive.resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
prov=json.loads(raw['provenance.json']);result=json.loads(raw['result.json']);assert len(prov['inputs'])==211 and result['inputs_unchanged'] and result['assets_unchanged']
assert prov['host_before']==result['host_after']=='060649ac32b34ce2cda231a9cf1ebcdfd8e8c12f0b0b3f94e527714f6993d911'
expected={f'ur{i}-{branch}' for i,branch in itertools.product(range(26,30),['surge','control','sustain'])};assert {x['case'] for x in result['cases']}==expected
images=json.loads((D/'image-index.json').read_text());assert len(images)==60
witness={}
pat=r'PASS UR (\d+) (Surge|Control|Sustain): real 3-way selection exclusive; actions=(\d+) damage=(true|false) pull=(true|false) summons_max=(\d+) wood=(\d+) core=([\d.]+) pause frozen'
for row in result['cases']:
 case=row['case'];assert row['returncode']==0 and json.loads(raw[case+'/process.json'])==row
 assert row['command'][-4:]==['400',case,'0xA11CE','1280x720']
 text=raw[case+'/native.log'].decode();records=re.findall(pat,text);assert len(records)==1
 skill,branch,actions,damage,pull,summons,wood,core=records[0];assert case==f'ur{skill}-{branch.lower()}' and int(actions)>0 and damage=='true'
 assert 'wrote 400 simulation frames' in text and 'all 5 screenshot(s) saved' in text
 assert {n for n in images if n.startswith(case+'/')}=={f'{case}/native/frame{x:05}.png' for x in [30,60,180,300,399]}
 witness[case]={'pull':pull,'summons':int(summons),'wood':int(wood),'core':float(core)}
for n,x in images.items():f=D/'frames'/n;assert f.stat().st_size==x['bytes'] and sha(f.read_bytes())==x['sha256']
assert witness['ur26-sustain']['summons']==2 and witness['ur29-surge']['summons']==4
assert witness['ur27-control']['pull']=='true' and witness['ur27-sustain']['core']==560
assert witness['ur29-sustain']['wood']==68
review=json.loads(raw['review/mapping.json']);assert len(review)==24
for row in review:assert sha((D/'frames'/row['source']).read_bytes())==row['sha256'] and (D/'review'/row['review']).exists()
join=json.loads((D/'source-join.json').read_text());assert join['capture_sha256']==prov['binary']
for n,h in join['source_files'].items():assert sha((D/'source'/n).read_bytes())==prov['inputs'][n]==h
if args.local_root:
 assert sha((args.local_root/join['capture_path']).read_bytes())==prov['binary']
 for n,h in {**prov['inputs'],**prov['assets']}.items():assert sha((args.local_root/n).read_bytes())==h,n
 print('PASS read-only actual binary,211 inputs and asset hashes; no native run')
print('PASS exact archive/raw bytes,12 branches/4800 updates,60 originals,24 reviewed frames and varied runtime/pause witnesses.')
