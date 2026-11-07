from pathlib import Path
import subprocess,json,time
r=Path.cwd();p=r/'target/verification/forge-package-2026-11-08';out=p/'preview';artifact=json.loads((p/'artifact.json').read_text());image=json.loads((p/'runtime-image.json').read_text())['id'];name='rogue-forge-preview-nov8'
cmd=['docker','run','--rm','--name',name,'--network','none','--mount',f'type=bind,src={r/artifact["package_directory"]},dst=/game,readonly','--mount',f'type=bind,src={out},dst=/verify','--mount',f'type=bind,src={out/"harness-source"},dst=/harness,readonly','--env','PACKAGE_COMMIT='+artifact['source_commit'],'--workdir','/tmp',image,'python3','/harness/forge_preview_runtime.py']
start=time.monotonic()
try:
 with (out/'process.log').open('w') as log:res=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,timeout=600)
 receipt={'command':cmd,'returncode':res.returncode,'elapsed':time.monotonic()-start}
except subprocess.TimeoutExpired:receipt={'command':cmd,'timeout':True,'elapsed':time.monotonic()-start}
finally:subprocess.run(['docker','stop','-t','15',name],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
(out/'process-result.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt),flush=True);assert receipt.get('returncode')==0
