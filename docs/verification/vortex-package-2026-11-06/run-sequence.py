import pathlib,subprocess,json,time
root=pathlib.Path.cwd();out=root/'target/verification/vortex-package-2026-11-06';package=root/'target/desktop-packages-vortex-nov6/rogue-commander-linux'
while subprocess.run(['systemctl','--user','is-active','--quiet','rogue-vortex-sdk-package-nov6']).returncode==0:time.sleep(3)
commit=json.loads((package/'version.json').read_text())['git_commit']
assert commit == (out/'source-commit.txt').read_text().strip()
import hashlib
manifest=json.loads((package/'package-manifest.json').read_text())['files']
for row in manifest:assert hashlib.sha256((package/row['file']).read_bytes()).hexdigest()==row['sha256'],row['file']
(out/'package-check.json').write_text(json.dumps({'source_commit':commit,'files_verified':len(manifest),'binary_sha256':hashlib.sha256((package/'rogue_commander').read_bytes()).hexdigest()},indent=2)+'\n')
for case,driver,limit,extra in [('smoke','smoke_runtime.py',600,[])]:
    folder=out/case;name='rogue-vortex-full-'+case+'-nov5'
    args=['docker','run','--rm','--name',name,'--network','none','--mount',f'type=bind,src={package},dst=/game,readonly','--mount',f'type=bind,src={folder},dst=/verify','--mount',f'type=bind,src={folder}/harness-source,dst=/harness,readonly','--env','PACKAGE_COMMIT='+commit]
    for setting in extra:args+=['--env',setting]
    args+=['--workdir','/tmp','rogue-commander-linux-smoke:ubuntu24-oct8','python3','/harness/'+driver]
    started=time.time()
    try:
        with (out/(case+'.log')).open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT,timeout=limit)
        status={'exit':p.returncode,'elapsed':time.time()-started}
    except subprocess.TimeoutExpired:
        subprocess.run(['docker','stop','-t','15',name],stdout=subprocess.DEVNULL);status={'timeout':True,'elapsed':time.time()-started}
    (out/(case+'-process.json')).write_text(json.dumps(status,indent=2)+'\n');print(case,status,flush=True)
    if case=='smoke':
        assert status.get('exit')==0 and 'DESKTOP_RUNTIME_PASS' in (out/'smoke.log').read_text(),'Smoke must pass before gameplay acceptance'
    # The forge route is independent of chapter outcome; preserve any harness
    # interruption as incomplete rather than falsely treating it as a game loss.
