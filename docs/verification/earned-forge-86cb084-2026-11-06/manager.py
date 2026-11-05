"""Prepared only; launch requires reviewed package pins and frozen helper snapshots."""
from pathlib import Path
import json,hashlib,subprocess,time
ROOT=Path.cwd();OUT=ROOT/'target/verification/vortex-earned-forge-2026-11-06'
PACKAGE=ROOT/'target/desktop-packages-vortex-nov6/rogue-commander-linux'
RESTART=ROOT/'target/verification/vortex-forge-restart-2026-11-06'
COMMIT='86cb08498e446a27b31554f8cb6dd3eb641e214b'
SAVE='da5bf0a9ca3f4032a265425f9989bda292d8c0c2622848157df6d2c0223ba825'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
pins=json.loads((OUT/'reviewed-package-pins.json').read_text())
version=json.loads((PACKAGE/'version.json').read_text())
assert version['git_commit']==COMMIT==pins['source_commit']
assert sha(PACKAGE/'rogue_commander')==pins['binary_sha256']==version['binary_sha256']
assert sha(PACKAGE/'package-manifest.json')==pins['manifest_sha256']
manifest=json.loads((PACKAGE/'package-manifest.json').read_text())['files']
assert len(manifest)==pins['files_verified']
for row in manifest:assert sha(PACKAGE/row['file'])==row['sha256'],row['file']
(OUT/'package-check.json').write_text(json.dumps(pins,indent=2)+'\n')
def check_snapshot(folder):
 snapshot=json.loads((folder/'harness-snapshot.json').read_text())
 assert snapshot['package_commit']==COMMIT
 for name,expected in snapshot['files'].items():
  assert sha(folder/'harness-source'/name)==expected,name

def run(folder,name,driver,limit,extra_mounts=()):
 args=['docker','run','--rm','--name',name,'--network','none',
       '--mount',f'type=bind,src={PACKAGE},dst=/game,readonly',
       '--mount',f'type=bind,src={folder},dst=/verify',
       '--mount',f'type=bind,src={folder}/harness-source,dst=/harness,readonly',
       '--env','PACKAGE_COMMIT='+COMMIT,'--env','PYTHONPATH=/harness']
 for mount in extra_mounts:args+=['--mount',mount]
 args+=['--workdir','/tmp','rogue-commander-linux-smoke:ubuntu24-oct8','python3','/harness/'+driver]
 (folder/'command.json').write_text(json.dumps(args,indent=2)+'\n');started=time.monotonic()
 try:
  with (folder/'runtime.log').open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT,timeout=limit)
  status={'exit':p.returncode,'elapsed':time.monotonic()-started}
 except subprocess.TimeoutExpired:
  subprocess.run(['docker','stop','-t','15',name],stdout=subprocess.DEVNULL)
  status={'timeout':True,'elapsed':time.monotonic()-started}
 (folder/'process.json').write_text(json.dumps(status,indent=2)+'\n');print(name,status,flush=True)
 return status
forge=OUT/'forge';check_snapshot(forge)
assert list((forge/'state').iterdir())==[forge/'state/campaign.save']
assert sha(forge/'state/campaign.save')==SAVE
status=run(forge,'rogue-vortex-earned-forge-nov6','forge_runtime.py',4800)
assert status.get('exit')==0,'Earned forge incomplete; no restart'
proof=json.loads((forge/'result.json').read_text())
assert proof['completed'] is True and proof['quit_exit']==0
assert proof['package_version']==version
assert proof['crafted_inventory_ocr']=={'count':1,'names':['astralprism']}
assert proof['initial_save_sha256']==proof['final_save_sha256']==SAVE
assert sha(forge/'state/campaign.save')==SAVE
# Exact same RW profile; no copy, injection or preference preparation.
check_snapshot(RESTART)
status=run(RESTART,'rogue-vortex-forge-restart-nov6','forge_restart_runtime.py',300,[
 f'type=bind,src={forge}/state,dst=/earned-state',
 f'type=bind,src={forge},dst=/previous-proof,readonly'])
assert status.get('exit')==0,'Same-profile restart incomplete'
restart=json.loads((RESTART/'result.json').read_text())
assert restart['completed'] is True and restart['result']=='PASS' and restart['quit_exit']==0
assert restart['equipment_count']==0 and restart['attributes']['strength']==30
assert restart['save_after_sha256']==SAVE
assert restart['resume_countdown']['after']<restart['resume_countdown']['before']
print('EARNED_FORGE_AND_SAME_PROFILE_RESTART_PASS',flush=True)
