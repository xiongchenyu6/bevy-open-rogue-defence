import pathlib,subprocess
out=pathlib.Path('target/verification/ray-order-2026-11-06')
with (out/'before-tests.log').open('w') as log:p=subprocess.run(['nix','develop','--command','cargo','test','--locked','--release','--lib','ray_pulse_damage_','-j2','--','--nocapture'],stdout=log,stderr=subprocess.STDOUT)
(out/'before-exit.txt').write_text(str(p.returncode)+'\n');print('BEFORE terminal',p.returncode,flush=True)
assert p.returncode==101
assert '0 passed; 2 failed' in (out/'before-tests.log').read_text()
