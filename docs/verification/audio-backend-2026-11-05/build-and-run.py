import pathlib,subprocess
out=pathlib.Path('target/verification/audio-backend-2026-11-05')
for name,args in [('capture-clippy.log',['nix','develop','--command','cargo','clippy','--locked','--bin','capture','-j2','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0','--','-D','warnings']),('capture-build.log',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j2','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0']),('native-sequence.log',['python3',str(out/'run-native.py'),'--timeout','240'])]:
 print('START',name,flush=True)
 with (out/name).open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
 assert p.returncode==0,(name,p.returncode)
 print('PASS',name,flush=True)
