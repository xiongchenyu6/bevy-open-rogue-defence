import pathlib,subprocess
out=pathlib.Path('target/verification/designated-strike-2026-11-06')
with (out/'before-tests.log').open('w') as log:p=subprocess.run(['nix','develop','--command','cargo','test','--locked','--release','--lib','nonarea_typed_strike_cannot_be_intercepted_by_overlapping_elite','-j2','--','--nocapture'],stdout=log,stderr=subprocess.STDOUT)
(out/'before-exit.txt').write_text(str(p.returncode)+'\n');print('BEFORE terminal exit',p.returncode,flush=True)
assert p.returncode!=0
assert 'test result: FAILED. 0 passed; 1 failed' in (out/'before-tests.log').read_text(),'must fail actual regression, not compilation'
