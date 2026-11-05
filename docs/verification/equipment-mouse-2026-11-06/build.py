from pathlib import Path
import subprocess,json,time
out=Path('target/verification/equipment-mouse-r3-2026-11-06')
commands=[('clippy.log',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings']),('capture-build.log',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j2','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])]
results=[]
with (out/'build-sequence.log').open('w') as sequence:
 for name,args in commands:
  started=time.monotonic()
  with (out/name).open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
  results.append({'log':name,'args':args,'exit':p.returncode,'elapsed':time.monotonic()-started});(out/'build-process.json').write_text(json.dumps(results,indent=2)+'\n')
  print(name,p.returncode,file=sequence,flush=True)
  if p.returncode:raise SystemExit(p.returncode)
