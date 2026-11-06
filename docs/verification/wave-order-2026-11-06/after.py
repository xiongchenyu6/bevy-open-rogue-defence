from pathlib import Path
import subprocess,json,hashlib
p=Path('target/verification/wave-order-2026-11-06')
def inputs():
 return {str(f):hashlib.sha256(f.read_bytes()).hexdigest()for f in sorted([Path('Cargo.toml'),Path('Cargo.lock'),*Path('src').rglob('*.rs'),*Path('assets').rglob('*')])if f.is_file()}
a=inputs();(p/'after-inputs.json').write_text(json.dumps(a,indent=2)+'\n')
for f in ['src/game/skills.rs','src/game/combat.rs','src/game/combat/wave_order_tests.rs','src/game/skills/tidal_wave.rs']:(p/('after-'+Path(f).name)).write_bytes(Path(f).read_bytes())
for name,cmd in [
 ('after-focused',['cargo','test','--locked','--release','--lib','combat::tests::tidal_wave_','-j2','--','--nocapture']),
 ('after-full',['cargo','test','--locked','--release','--lib','-j2','--','--nocapture']),
 ('clippy',['cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings'])]:
 with (p/(name+'.log')).open('w')as log:r=subprocess.run(['nix','develop','--command',*cmd],stdout=log,stderr=subprocess.STDOUT)
 (p/(name+'-exit.txt')).write_text(str(r.returncode)+'\n');print(name,'terminal',r.returncode,flush=True)
 assert r.returncode==0 and inputs()==a
(p/'after-inputs-after.json').write_text(json.dumps(inputs(),indent=2)+'\n')
print('TERMINAL AFTER PASS: focused/full/strict Clippy and frozen inputs',flush=True)
