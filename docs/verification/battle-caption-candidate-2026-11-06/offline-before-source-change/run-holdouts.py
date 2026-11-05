from pathlib import Path
import subprocess,json,time
root=Path.cwd();out=root/'target/verification/overlap-earned-forge-2026-11-06/audit';forge=out.parent/'forge'
args=['docker','run','--rm','--network','none','--mount',f'type=bind,src={root},dst=/repo,readonly','--mount',f'type=bind,src={forge},dst=/proof,readonly','--mount',f'type=bind,src={forge}/harness-source,dst=/harness,readonly','--mount',f'type=bind,src={out},dst=/audit,readonly','sha256:0d4cccee982b31492e0f7b01a50afc1a0ec710d3166dab22e02322ce64832715','python3','/audit/holdouts.py'];started=time.monotonic()
with (out/'candidate-holdouts.log').open('w')as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT,timeout=300)
(out/'candidate-holdouts-process.json').write_text(json.dumps({'args':args,'exit':p.returncode,'elapsed':time.monotonic()-started},indent=2)+'\n')
raise SystemExit(p.returncode)
