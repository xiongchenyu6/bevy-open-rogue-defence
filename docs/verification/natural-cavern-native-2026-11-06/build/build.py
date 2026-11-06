import hashlib,json,pathlib,shutil,subprocess,time
root=pathlib.Path.cwd();out=root/'target/verification/den-wide-camera-2026-11-06'
def inputs():
 paths=[root/'Cargo.toml',root/'Cargo.lock',*root.joinpath('src').rglob('*.rs'),*root.joinpath('assets').rglob('*')]
 return {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths) if p.is_file()}
before=inputs();(out/'build-inputs-before.json').write_text(json.dumps(before,indent=2)+'\n')
commands=[['nix','develop','--command','cargo','clippy','--locked','--release','--bin','capture','-j2','--','-D','warnings'],['nix','develop','--command','cargo','build','--locked','--bin','capture','-j2','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0']]
results=[]
for name,cmd in zip(['clippy','build'],commands):
 assert inputs()==before
 print('STAGE',name,'start',flush=True);start=time.monotonic()
 with (out/(name+'.log')).open('w') as log:r=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,timeout=900)
 results.append(dict(stage=name,command=cmd,exit=r.returncode,seconds=time.monotonic()-start))
 (out/'commands.json').write_text(json.dumps(results,indent=2)+'\n');print('STAGE',name,'exit',r.returncode,flush=True)
 assert r.returncode==0 and inputs()==before
binary=out/'capture';shutil.copy2(root/'target/market-visual/debug/capture',binary)
(out/'binary.json').write_text(json.dumps(dict(sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),bytes=binary.stat().st_size),indent=2)+'\n')
(out/'build-inputs-after.json').write_text(json.dumps(inputs(),indent=2)+'\n')
print('BUILD_READY source and assets frozen; camera is diagnostic-only',flush=True)
