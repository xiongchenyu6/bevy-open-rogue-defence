from pathlib import Path
import subprocess,json,hashlib,time
r=Path.cwd();p=r/'target/verification/full-slot-variety-2026-11-07/before'
def inputs():return {str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()}
a=inputs();(p/'inputs.json').write_text(json.dumps(a,indent=2))
for n in ['src/game/progression.rs','src/game/progression/full_slot_tests.rs']:
 f=p/'source'/n;f.parent.mkdir(parents=True,exist_ok=True);f.write_bytes((r/n).read_bytes())
cmd=['nix','develop','--command','cargo','test','--locked','--release','--lib','full_slot_normal_g','-j2','--','--nocapture'];t=time.monotonic()
with (p/'tests.log').open('w') as f:x=subprocess.run(cmd,stdout=f,stderr=subprocess.STDOUT)
(p/'result.json').write_text(json.dumps({'command':cmd,'returncode':x.returncode,'elapsed':time.monotonic()-t}));assert inputs()==a
print('FULL_SLOT_BEFORE TERMINAL',x.returncode,flush=True)
assert x.returncode==101
text=(p/'tests.log').read_text();assert '0 passed; 2 failed' in text and 'FULL_SLOT_NORMAL_G seeds=128 replacements=0' in text
(p/'inputs-after.json').write_text(json.dumps(inputs(),indent=2));print('VALID BEHAVIOR NEGATIVE',flush=True)
