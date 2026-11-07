import pathlib,subprocess,hashlib,json,shutil,runpy
r=pathlib.Path.cwd();p=r/'target/verification/draft-header-2026-11-07'
def inputs():return {str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()}
a=inputs();(p/'pointer-inputs.json').write_text(json.dumps(a,indent=2));shutil.copy2(r/'src/bin/capture/immediate_choice.rs',p/'pointer-immediate_choice.rs')
def run(name,cmd):
 with(p/(name+'.log')).open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT)
 (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode}));assert x.returncode==0;assert inputs()==a
 print(name,'TERMINAL PASS',flush=True)
run('pointer-library-tests',['nix','develop','--command','cargo','test','--locked','--release','--lib','-j2'])
run('pointer-build',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])
shutil.copy2(r/'target/market-visual/debug/capture',p/'pointer-capture');(p/'pointer-binary.sha256').write_text(hashlib.sha256((p/'pointer-capture').read_bytes()).hexdigest())
runpy.run_path(str(p/'input.py'),run_name='__main__')
run('pointer-clippy',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings'])
(p/'pointer-inputs-after.json').write_text(json.dumps(inputs(),indent=2));print('POINTER FINAL CHAIN TERMINAL PASS',flush=True)
