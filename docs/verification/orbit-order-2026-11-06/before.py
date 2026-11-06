from pathlib import Path
import subprocess,json,hashlib
p=Path('target/verification/orbit-order-2026-11-06')
def inputs():
 return {str(f):hashlib.sha256(f.read_bytes()).hexdigest()for f in sorted([Path('Cargo.toml'),Path('Cargo.lock'),*Path('src').rglob('*.rs'),*Path('assets').rglob('*')])if f.is_file()}
a=inputs();(p/'before-inputs.json').write_text(json.dumps(a,indent=2)+'\n')
for f in ['src/game/skills.rs','src/game/combat.rs','src/game/combat/orbit_order_tests.rs']:(p/('before-'+Path(f).name)).write_bytes(Path(f).read_bytes())
with (p/'before-tests.log').open('w')as log:r=subprocess.run(['nix','develop','--command','cargo','test','--locked','--release','--lib','combat::tests::orbit_blade_','-j2','--','--nocapture'],stdout=log,stderr=subprocess.STDOUT)
(p/'before-exit.txt').write_text(str(r.returncode)+'\n');print('BEFORE actual exit',r.returncode,flush=True)
assert inputs()==a,'source/assets drift'
(p/'before-inputs-after.json').write_text(json.dumps(inputs(),indent=2)+'\n')
s=(p/'before-tests.log').read_text()
assert r.returncode==101 and s.count('ORBIT_BLADE_ORDER reverse=')==4 and 'test result: FAILED.'in s,'require actual four-world behavior negative, not compile/fixture failure'
print('BEFORE four-world valid behavior failure preserved',flush=True)
