import pathlib,subprocess
out=pathlib.Path('target/verification/overlap-geometry-2026-11-06')
with (out/'before-paired-tests.log').open('w') as log:
 p=subprocess.run(['nix','develop','--command','cargo','test','--locked','--release','--lib','overlap_geometry_','-j2','--','--nocapture'],stdout=log,stderr=subprocess.STDOUT)
(out/'before-paired-exit.txt').write_text(str(p.returncode)+'\n')
print('BEFORE terminal',p.returncode,flush=True)
assert p.returncode==101
t=(out/'before-paired-tests.log').read_text();assert '1 passed; 2 failed' in t
