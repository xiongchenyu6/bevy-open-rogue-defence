import subprocess,time,pathlib,sys
out=pathlib.Path('target/verification/melee-whoosh-implementation-2026-11-06')
for _ in range(600):
 r=subprocess.run(['systemctl','--user','show','rogue-whoosh-final-suite-nov6.service','-p','ActiveState','-p','ExecMainStatus'],capture_output=True,text=True,check=True)
 if 'ActiveState=active' not in r.stdout and 'ActiveState=activating' not in r.stdout:break
 time.sleep(2)
else:raise TimeoutError('final suite gate timeout')
assert 'ExecMainStatus=0' in r.stdout,r.stdout
assert '0 failed; 1 ignored' in (out/'final-full-tests.log').read_text()
assert 'Finished `release`' in (out/'final-clippy.log').read_text()
print('FINAL_TEST_CLIPPY_GATE_PASS',flush=True)
sys.exit(subprocess.run([sys.executable,str(out/'native.py')]).returncode)
