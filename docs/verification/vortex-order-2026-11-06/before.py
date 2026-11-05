import pathlib,subprocess
out=pathlib.Path('target/verification/vortex-order-2026-11-06')
args=['nix','develop','--command','cargo','test','--locked','--release','--lib','vortex_pulse_damage_ignores_cosmetic_enemy_archetype_order','-j2','--','--nocapture']
with (out/'before-tests.log').open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
(out/'before-exit.txt').write_text(str(p.returncode)+'\n')
print('BEFORE terminal exit',p.returncode,flush=True)
assert p.returncode!=0,'expected old production order to fail focused invariant'
assert 'test result: FAILED. 0 passed; 1 failed' in (out/'before-tests.log').read_text(),'must fail actual regression, not compilation'
