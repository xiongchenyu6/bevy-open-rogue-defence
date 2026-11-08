import argparse,gzip,hashlib,json,re,shutil,subprocess,tarfile,tomllib
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('archive',type=Path);p.add_argument('--pixels',action='store_true');p.add_argument('--local-root',type=Path);args=p.parse_args();D=args.archive.resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
for n in ['library-process.json','clippy-process.json','native-build-process.json']:assert json.loads(raw['validation/'+n])['returncode']==0
assert b'792 passed; 0 failed; 1 ignored' in raw['validation/library.log']
host=json.loads(raw['validation/host-profile.json']);assert host['before']==host['after']=='060649ac32b34ce2cda231a9cf1ebcdfd8e8c12f0b0b3f94e527714f6993d911'
keys=['package_hashes_verified','no_source','no_rust','no_nix','chapter_recruit_battle','fullscreen_restored','language_saved_and_reloaded','normal_quit_twice','real_guide_after_readiness','tutorial_dismissal_saved','restart_skips_automatic_guide_and_runs_battle','public_portrait_skill_draw','portrait_skill_click_resumed_battle']
for label in ['diagnostic','production']:
 assert json.loads(raw[label+'/process.json'])['returncode']==0
 result=json.loads(raw[label+'/runtime/result.json']);assert all(result[x] is True for x in keys)
 rows=json.loads(raw[label+'/runtime/resize-observations.json']);assert len(rows)==24
 for cycle in range(4):
  group=[x for x in rows if x['cycle']==cycle];assert [x['frame'] for x in group]==list(range(6))
  assert group[0]['right_region_mean']==(.154248 if label=='diagnostic' else .027451)
  if label=='production':assert group[1]['right_region_mean']==.027451 and .258<=group[2]['right_region_mean']<=.261
  for i,row in enumerate(group):
   assert 0<=row['elapsed_since_geometry']<row['capture_completed'] and row['right_region_mean']<1
   if i:assert row['elapsed_since_geometry']>group[i-1]['capture_completed']
   if args.pixels:
    tool=shutil.which('magick') or shutil.which('convert');assert tool
    measured=float(subprocess.check_output([tool,str(D/'frames'/label/row['screenshot']),'-crop','900x720+380+0','+repage','-format','%[fx:mean]','info:'],text=True));assert abs(measured-row['right_region_mean'])<=.000001
assert b'XSetWindowBackground' in raw['diagnostic/harness/smoke_runtime.py'] and b'XSetWindowBackground' not in raw['production/harness/smoke_runtime.py']
old=json.loads(gzip.decompress((D/'old-white-observations.json.gz').read_bytes()));assert len(old)==24 and all(x['right_region_mean']==1 for x in old if x['frame']==0)
join=json.loads((D/'source-join.json').read_text());audit=json.loads(raw['package/input-audit-before.json']);assert audit==json.loads(raw['package/input-audit-after.json']) or audit['files']==json.loads(raw['package/input-audit-after.json'])['files']
assert len(audit['files'])==553 and audit['source_commit']==join['source_commit']
for n,h in join['sources'].items():assert sha((D/'source'/n).read_bytes())==h==audit['files'][n]
lock=tomllib.loads((D/'source/Cargo.lock').read_text());assert any(x['name']=='x11rb' and x['version']=='0.13.2' for x in lock['package']);assert any(x['name']=='raw-window-handle' and x['version']=='0.6.2' for x in lock['package'])
for n in ['build-process.json','smoke-process.json']:assert json.loads(raw['package/'+n])['exit']==0
assert all(json.loads(raw['package/smoke/result.json'])[x] is True for x in keys)
artifact=json.loads(raw['package/artifact.json']);check=json.loads(raw['package/root-artifact-check.json']);assert check['result']=='PASS' and check['source_inputs']==553 and check['manifest_files']==892 and check['tar_files']==893
assert check['binary_sha256']==artifact['binary_sha256'] and check['tar_sha256']==artifact['tar_sha256']
assert b'/nix/store' not in raw['package/root-readelf.txt']
review=json.loads(raw['production/review/mapping.json']);assert len(review)==8
for row in review:assert sha((D/'frames/production'/Path(row['source']).name).read_bytes())==row['sha256']
assert len(list((D/'frames').rglob('*.png')))==48
if args.local_root:
 package=args.local_root/artifact['package_directory'];archive=args.local_root/artifact['tar'];assert sha((package/'rogue_commander').read_bytes())==artifact['binary_sha256'] and sha(archive.read_bytes())==artifact['tar_sha256']
 manifest=json.loads((package/'package-manifest.json').read_text())['files'];assert len(manifest)==892
 for row in manifest:assert sha((package/row['file']).read_bytes())==row['sha256']
 with tarfile.open(archive) as tar:
  files={x.name.removeprefix('rogue-commander-linux/'):x for x in tar.getmembers() if x.isfile()};assert set(files)=={x['file'] for x in manifest}|{'package-manifest.json'}
  for n,x in files.items():assert x.uid==x.gid==x.mtime==0 and tar.extractfile(x).read()==(package/n).read_bytes()
 print('PASS actual local package binary/manifest/tar bytes and normalized metadata')
if args.pixels:print('PASS actual ImageMagick48-observation ROI recheck')
print('PASS exact archive/raw bytes,792+1 tests,intervention/production resize bounds,new package13-check smoke and source pins.')
