from pathlib import Path
import subprocess,json,hashlib
r=Path.cwd();p=r/'target/verification/arrow-classification-2026-11-07'
def inputs():return {str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()}
a=inputs();(p/'after-inputs.json').write_text(json.dumps(a,indent=2))
for f in ['src/game/bonds.rs','src/game/combat.rs','src/game/combat/ranged_release.rs','src/game/combat/ranged_release_tests.rs','src/game/hero_fusion.rs','src/game/progression.rs','src/game/skills.rs','src/game/combat/arrow_classification_tests.rs']:(p/('after-'+f.replace('/','-'))).write_bytes((r/f).read_bytes())
for name,cmd in [
 ('focused',['nix','develop','--command','cargo','test','--locked','--release','--lib','arrow_classification_','-j2','--','--nocapture']),
 ('library',['nix','develop','--command','cargo','test','--locked','--release','--lib','-j2']),
 ('clippy',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings'])]:
 with (p/(name+'.log')).open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT)
 (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode}))
 assert inputs()==a,'input freeze changed'
 assert x.returncode==0,(name,x.returncode)
 print(name,'ACTUAL TERMINAL PASS',flush=True)
(p/'after-inputs-after.json').write_text(json.dumps(inputs(),indent=2));print('ARROW CLASSIFICATION CHECKS PASS',flush=True)
