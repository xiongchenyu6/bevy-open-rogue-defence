import pathlib,subprocess
p=pathlib.Path('target/verification/storm-matrix-order-r2-2026-11-06')
with (p/'before-tests.log').open('w') as log:r=subprocess.run(['nix','develop','--command','cargo','test','--locked','--release','--lib','combat::tests::storm_matrix_','-j2','--','--nocapture'],stdout=log,stderr=subprocess.STDOUT)
(p/'before-exit.txt').write_text(str(r.returncode)+'\n');print('BEFORE terminal',r.returncode,flush=True)
assert r.returncode==101
assert '1 passed; 1 failed' in (p/'before-tests.log').read_text()
