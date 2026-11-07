from pathlib import Path
import hashlib,json,shutil,subprocess,os,time
r=Path.cwd();o=r/'target/verification/vortex-allocation-2026-11-08';f=o/'frozen'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
receipt=json.loads((o/'after-r2/sim-build-result.json').read_text());assert receipt['returncode']==0
inputs=json.loads((o/'after-r2/inputs.json').read_text());assert all(sha(r/n)==h for n,h in inputs.items())
f.mkdir();(f/'tools').mkdir();(f/'src/game').mkdir(parents=True)
for n in ['tools/run_balance_matrix.py','tools/compare_balance_replays.py','src/game/campaign_progress.rs']:
 shutil.copy2(r/n,f/n)
shutil.copytree(r/'assets',f/'assets');shutil.copy2(r/'target/release/balance_sim',f/'balance-sim');(f/'balance-sim').chmod(0o555)
pins={str(p.relative_to(f)):sha(p) for p in f.rglob('*') if p.is_file()};(o/'pair-pins.json').write_text(json.dumps(pins,indent=2))
processes=[]
for label in ['a','b']:
 batch='vortex-allocation-pair-'+label+'-nov8'
 env={k:v for k,v in os.environ.items() if not k.startswith('ROGUE_') and k!='RUST_LOG'}
 env.update(ROGUE_SIM_ASSET_ROOT=str(f/'assets'),ROGUE_SIM_TRACE_RANGE='1:180000',ROGUE_SIM_PICKER_TRACE='1',RUST_LOG='rogue_commander::projectile_hit=trace')
 cmd=['nix','develop','--command','python3',str(f/'tools/run_balance_matrix.py'),'--binary',str(f/'balance-sim'),'--batch',batch,'--frames','180000','--jobs','1','--case-timeout','1800','--chapters','0','--training','0','--seeds','1','--policies','active','--starters','1','--positions','behind','--require-telemetry']
 print('START',batch,flush=True);start=time.monotonic()
 with (o/('pair-'+label+'.log')).open('w') as log:
  try:
   result=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=1900)
   row=dict(label=label,command=cmd,returncode=result.returncode,elapsed=time.monotonic()-start)
  except subprocess.TimeoutExpired:
   row=dict(label=label,command=cmd,timeout=True,elapsed=time.monotonic()-start)
 processes.append(row);(o/'pair-processes.json').write_text(json.dumps(processes,indent=2))
 assert row.get('returncode')==0,row
 summary=json.loads((f/'target/verification'/batch/'summary.json').read_text());case=summary['cases'][0]
 assert len(summary['cases'])==1 and case['valid'] and not case['timed_out'] and case['returncode']==0,case
 assert summary['binary_sha256']==pins['balance-sim']
 assert all(sha(f/n)==h for n,h in pins.items())
 assert all(sha(r/n)==h for n,h in inputs.items())
 print('END',batch,case['outcome'],case['last']['elapsed'],flush=True)
print('PAIR TERMINAL; equality pending independent comparison',flush=True)
