"""Rerun of the 24-case pressure matrix (same dimensions as post-life-pressure-matrix-2026-11-08)."""
from pathlib import Path
import subprocess,json,hashlib,time,os,shutil,sys
r=Path.cwd();p=r/'target/verification/combat-visual-2026-11-13/matrix';f=p/'frozen'
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
f.mkdir(parents=True);(f/'tools').mkdir();(f/'src/game').mkdir(parents=True)
for n in ['tools/run_balance_matrix.py','tools/analyze_balance.py','src/game/campaign_progress.rs']:shutil.copy2(r/n,f/n)
shutil.copytree(r/'assets',f/'assets');shutil.copy2(r/'target/verification/combat-visual-2026-11-13/balance-sim-head',f/'balance-sim');(f/'balance-sim').chmod(0o555)
pins={str(q.relative_to(f)):sha(q) for q in f.rglob('*') if q.is_file()}
head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
dirty=subprocess.check_output(['git','status','--porcelain','--untracked-files=no'],text=True)
(p/'source-join.json').write_text(json.dumps(dict(head=head,tracked_dirty=dirty,frozen_pins=pins),indent=2))
profile=Path.home()/'.config/rogue-commander/campaign-v1.save';before=sha(profile) if profile.exists() else None
cmd=['nix','develop','--command','python3',str(f/'tools/run_balance_matrix.py'),'--binary',str(f/'balance-sim'),'--batch',sys.argv[1],'--frames','180000','--jobs','4','--case-timeout','900','--chapters','0,5','--training','0','--seeds','1,BEEF','--policies','passive,active','--starters','0,1,2','--positions','behind','--require-telemetry']
env={k:v for k,v in os.environ.items() if not k.startswith('ROGUE_') and k!='RUST_LOG'};env['ROGUE_SIM_ASSET_ROOT']=str(f/'assets');start=time.monotonic()
with (p/'matrix.log').open('w') as log:result=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT)
after=sha(profile) if profile.exists() else None
(p/'process.json').write_text(json.dumps(dict(command=cmd,returncode=result.returncode,elapsed=time.monotonic()-start,profile_unchanged=before==after),indent=2))
assert all(sha(f/n)==h for n,h in pins.items())
raise SystemExit(result.returncode)
