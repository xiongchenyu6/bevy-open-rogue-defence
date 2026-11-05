import pathlib,subprocess,hashlib,json
root=pathlib.Path.cwd();out=root/'target/verification/dagger-rhythm-2026-11-06'
def receipt():
 files=[root/'Cargo.toml',root/'Cargo.lock']+list((root/'src').rglob('*.rs'))+list((root/'assets').rglob('*'))
 return {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(files) if p.is_file()}
a=receipt();(out/'release-inputs-before.json').write_text(json.dumps(a,indent=2)+'\n');assert a==json.loads((out/'after-inputs-after.json').read_text()),'Source changed since actual native artifact'
for name,args in [('full-tests.log',['nix','develop','--command','cargo','test','--locked','--release','--lib','-j2']),('clippy.log',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings'])]:
 print('START',name,flush=True)
 with (out/name).open('w') as log:r=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
 assert r.returncode==0,name
 print('PASS',name,flush=True)
b=receipt();(out/'release-inputs-after.json').write_text(json.dumps(b,indent=2)+'\n');assert a==b,'Build inputs changed'
print('TERMINAL RELEASE PASS',flush=True)
