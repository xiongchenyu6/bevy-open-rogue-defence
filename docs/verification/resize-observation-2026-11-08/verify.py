import argparse,gzip,hashlib,json,shutil,subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('archive',type=Path);p.add_argument('--pixels',action='store_true');args=p.parse_args();D=args.archive.resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
assert json.loads(raw['process.json'])['returncode']==0
result=json.loads(raw['runtime/result.json'])
for key in ['package_hashes_verified','no_source','no_rust','no_nix','chapter_recruit_battle','fullscreen_restored','language_saved_and_reloaded','normal_quit_twice','real_guide_after_readiness','tutorial_dismissal_saved','restart_skips_automatic_guide_and_runs_battle','public_portrait_skill_draw','portrait_skill_click_resumed_battle']:assert result[key] is True,key
rows=json.loads(raw['runtime/resize-observations.json']);assert len(rows)==24
images=json.loads((D/'local-image-manifest.json').read_text())
for cycle in range(4):
 group=[x for x in rows if x['cycle']==cycle];assert [x['frame'] for x in group]==list(range(6));assert group[0]['right_region_mean']==1.0
 assert all(0<x['right_region_mean']<1 for x in group[1:])
 for i,row in enumerate(group):
  assert row['elapsed_since_geometry']>=0 and row['capture_completed']>row['elapsed_since_geometry']
  if i:assert row['elapsed_since_geometry']>group[i-1]['capture_completed']
  f=D/'frames'/row['screenshot'];assert sha(f.read_bytes())==images['runtime/'+row['screenshot']]['sha256']
  if args.pixels:
   tool=shutil.which('magick') or shutil.which('convert');assert tool,'ImageMagick required for --pixels'
   measured=float(subprocess.check_output([tool,str(f),'-crop','900x720+380+0','+repage','-format','%[fx:mean]','info:'],text=True));assert abs(measured-row['right_region_mean'])<=.000001,(f,measured,row['right_region_mean'])
join=json.loads((D/'harness-join.json').read_text());assert join['changed_files']==['smoke_runtime.py'] and len(join['actual_files'])==6
for n,h in join['actual_files'].items():
 assert sha(raw['harness/'+n])==h
 if n!='smoke_runtime.py':assert raw['harness/'+n]==(D/'baseline'/n).read_bytes()
review=json.loads(raw['root-reviewed.json']);assert len(review['originals'])==8
for n in review['originals']:assert (D/'frames'/n).exists()
assert len(list((D/'frames').glob('*.png')))==24
state=(D/'unit-state.txt').read_text();assert 'Result=success' in state and 'ExecMainStatus=0' in state
if args.pixels:print('PASS actual ImageMagick recheck of all24 right-region means')
print('PASS exact archive/raw bytes,24 bounded observations,four white-to-nonwhite transitions and actual13-check smoke.')
