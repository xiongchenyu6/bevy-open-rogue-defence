import pathlib,subprocess,hashlib,json
p=pathlib.Path('target/verification/storm-matrix-order-r2-2026-11-06')
expected=json.loads((p/'after-source-hashes.json').read_text())
def check():
 assert all(hashlib.sha256(pathlib.Path(f).read_bytes()).hexdigest()==h for f,h in expected.items()),'source drift'
check()
for name,cmd in [
 ('after-tests',['cargo','test','--locked','--release','--lib','-j2','--','--nocapture']),
 ('clippy',['cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings'])]:
 with (p/(name+'.log')).open('w') as log:r=subprocess.run(['nix','develop','--command',*cmd],stdout=log,stderr=subprocess.STDOUT)
 (p/(name+'-exit.txt')).write_text(str(r.returncode)+'\n');print(name,'terminal',r.returncode,flush=True)
 assert r.returncode==0
 check()
(p/'after-pass.txt').write_text('PASS full library + strict all-target Clippy; source hashes unchanged\n')
