import pathlib,subprocess,json,time,hashlib
root=pathlib.Path.cwd();out=root/'target/verification/inventory-chapter6-2026-11-06';folder=out/'chapter6'
package=root/'target/desktop-packages-inventory-nov5/rogue-commander-linux';proof=json.loads((out/'harness-provenance.json').read_text())
for name,digest in proof['harness_files'].items():
    assert hashlib.sha256((folder/'harness-source'/name).read_bytes()).hexdigest()==digest,name
assert hashlib.sha256((folder/'state/campaign.save').read_bytes()).hexdigest()==proof['incoming_save_sha256']
assert sorted(p.name for p in (folder/'state').iterdir())==['campaign.save']
version=json.loads((package/'version.json').read_text());assert version['git_commit']==proof['package_commit']
manifest=json.loads((package/'package-manifest.json').read_text())['files']
for row in manifest:assert hashlib.sha256((package/row['file']).read_bytes()).hexdigest()==row['sha256'],row['file']
binary_hash=hashlib.sha256((package/'rogue_commander').read_bytes()).hexdigest()
assert binary_hash==version['binary_sha256']==proof['package_binary_sha256']
image=subprocess.check_output(['docker','image','inspect','rogue-commander-linux-smoke:ubuntu24-oct8','--format','{{.Id}}'],text=True).strip()
(out/'package-check.json').write_text(json.dumps(dict(source_commit=version['git_commit'],files_verified=len(manifest),binary_sha256=binary_hash,runtime_image_id=image),indent=2)+'\n')
name='rogue-inventory-c6-nov6'
args=['docker','run','--rm','--name',name,'--network','none','--mount',f'type=bind,src={package},dst=/game,readonly','--mount',f'type=bind,src={folder},dst=/verify','--mount',f'type=bind,src={folder}/harness-source,dst=/harness,readonly','--env','PACKAGE_COMMIT='+version['git_commit']]
for setting in ('CAMPAIGN_STAGE=5','CAMPAIGN_CONTINUE=1','CAMPAIGN_SPEED=1','CAMPAIGN_SEED=0xA11CE'):args+=['--env',setting]
args+=['--workdir','/tmp',image,'python3','/harness/campaign_equipment_runtime.py']
(out/'launch-command.json').write_text(json.dumps(args,indent=2)+'\n')
started=time.time();print(json.dumps(dict(stage='launch',harness_commit=proof['harness_commit'],package_commit=version['git_commit'],container=name,outer_budget=5400)),flush=True)
try:
    with (out/'chapter6.log').open('w') as log:process=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT,timeout=5400)
    result=dict(exit=process.returncode,elapsed=time.time()-started)
except subprocess.TimeoutExpired:
    subprocess.run(['docker','stop','-t','15',name],stdout=subprocess.DEVNULL)
    result=dict(timeout=True,elapsed=time.time()-started)
(out/'chapter6-process.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(dict(stage='terminal',**result)),flush=True)
