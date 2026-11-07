from pathlib import Path
import subprocess,json,hashlib,shutil,time,os
r=Path.cwd();p=r/'target/verification/full-slot-variety-2026-11-07/baseline-build';frozen=r/'target/verification/full-slot-variety-2026-11-07/matched-inputs'
def inputs():return {str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()}
a=inputs();(p/'inputs.json').write_text(json.dumps(a,indent=2));(p/'HEAD.txt').write_text(subprocess.check_output(['git','rev-parse','HEAD'],text=True))
for n in ['src/game/progression.rs','src/game/progression/full_slot_tests.rs','src/bin/balance_sim.rs']:
 d=p/'source'/n;d.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(r/n,d)
def run(name,cmd):
 t=time.monotonic()
 with (p/(name+'.log')).open('w') as out:x=subprocess.run(cmd,stdout=out,stderr=subprocess.STDOUT)
 (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode,'elapsed':time.monotonic()-t}));assert inputs()==a,'source changed while building';print(name,'TERMINAL',x.returncode,flush=True);assert x.returncode==0
run('bin-tests',['nix','develop','--command','cargo','test','--locked','--release','--bin','balance_sim','-j2'])
run('build',['nix','develop','--command','cargo','build','--locked','--release','--bin','balance_sim','-j2'])
exe=frozen/'balance-sim-before';shutil.copyfile(r/'target/release/balance_sim',exe);exe.chmod(0o755);(p/'binary.sha256').write_text(hashlib.sha256(exe.read_bytes()).hexdigest());(p/'inputs-after.json').write_text(json.dumps(inputs(),indent=2))
assert all(hashlib.sha256((frozen/k).read_bytes()).hexdigest()==v for k,v in a.items() if k.startswith('assets/'))
print('BASELINE BUILD FREEZE PASS',flush=True)
env={k:v for k,v in os.environ.items() if not k.startswith('ROGUE_') and k!='RUST_LOG'};env['ROGUE_SIM_ASSET_ROOT']=str(frozen/'assets')
cmd=['nix','develop','--command','python3',str(frozen/'tools/run_balance_matrix.py'),'--binary',str(exe),'--batch','full-slot-baseline-c1-2026-11-07','--frames','180000','--jobs','2','--case-timeout','900','--chapters','0','--training','0','--seeds','1,2','--policies','active,passive','--starters','0,1,2','--positions','behind','--require-telemetry'];t=time.monotonic()
with (p/'matrix.log').open('w') as out:x=subprocess.run(cmd,stdout=out,stderr=subprocess.STDOUT,env=env,timeout=5700)
(p/'matrix-result.json').write_text(json.dumps({'command':cmd,'returncode':x.returncode,'elapsed':time.monotonic()-t}));assert inputs()==a;print('BASELINE MATRIX TERMINAL',x.returncode,flush=True);assert x.returncode==0
