from pathlib import Path
import json,subprocess,time
root=Path.cwd();out=root/'target/verification/inventory-close-r3-2026-11-06';name='rogue-inventory-close-r3-nov6'
args=['docker','run','--rm','--name',name,'--network','none','--mount',f'type=bind,src={root}/target/desktop-packages-inventory-nov5/rogue-commander-linux,dst=/game,readonly','--mount',f'type=bind,src={out},dst=/verify','--mount',f'type=bind,src={out}/harness-source,dst=/harness,readonly','--env','PYTHONPATH=/harness','--workdir','/tmp','rogue-commander-linux-smoke:ubuntu24-oct8','python3','/verify/probe.py']
(out/'command.json').write_text(json.dumps(args,indent=2)+'\n');started=time.monotonic()
try:
 with (out/'runtime.log').open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT,timeout=300)
 result={'exit':p.returncode,'elapsed':time.monotonic()-started}
except subprocess.TimeoutExpired:
 subprocess.run(['docker','stop','-t','15',name]);result={'timeout':True,'elapsed':time.monotonic()-started}
(out/'process.json').write_text(json.dumps(result,indent=2)+'\n')
raise SystemExit(0 if result.get('exit')==0 else 1)
