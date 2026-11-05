import pathlib,subprocess,hashlib,json
root=pathlib.Path.cwd();out=root/'target/verification/native-aim-2026-11-06'
def receipt():
 files=[root/'Cargo.toml',root/'Cargo.lock']+list((root/'src').rglob('*.rs'))+list((root/'vendor/bevy_audio').rglob('*.rs'))
 return dict(head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),source_sha256={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(files)},diff_sha256=hashlib.sha256(subprocess.check_output(['git','diff','--no-ext-diff','--binary','HEAD'])).hexdigest())
before=receipt();(out/'after-source-before.json').write_text(json.dumps(before,indent=2)+'\n')
commands=[('after-full-tests.log',['nix','develop','--command','cargo','test','--locked','--release','--lib','-j2']),('after-clippy.log',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings']),('after-build.log',['nix','develop','--command','cargo','build','--locked','--release','--bin','balance_sim','-j2'])]
for name,args in commands:
 print('START',name,flush=True)
 with (out/name).open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
 assert p.returncode==0,(name,p.returncode)
 print('PASS',name,flush=True)
after=receipt();(out/'after-source-after.json').write_text(json.dumps(after,indent=2)+'\n');assert before['source_sha256']==after['source_sha256'],'Rust source changed during build'
import shutil
snapshot=out/'balance-sim-snapshot';shutil.copy2(root/'target/release/balance_sim',snapshot)
(out/'binary.json').write_text(json.dumps(dict(path=str(snapshot),sha256=hashlib.sha256(snapshot.read_bytes()).hexdigest(),source_head=after['head'],source_diff_sha256=after['diff_sha256']),indent=2)+'\n')
print('TERMINAL PASS; native aim design verification; no numerical matrix',flush=True)
