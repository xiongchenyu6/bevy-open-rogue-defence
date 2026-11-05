from pathlib import Path
import subprocess,json,time
root=Path.cwd();out=root/'target/verification/inventory-phase-budgets-2026-11-06'
started=time.time()
args=['docker','run','--rm','--network','none','--mount',f'type=bind,src={root},dst=/repo,readonly','--workdir','/repo/tools/desktop','rogue-commander-linux-smoke:ubuntu24-oct8','python3','-m','unittest','discover','-v']
with (out/'tests.log').open('w') as log:
 result=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT,timeout=600)
(out/'result.json').write_text(json.dumps({'returncode':result.returncode,'elapsed_seconds':time.time()-started,'command':args},indent=2)+'\n')
raise SystemExit(result.returncode)
