import hashlib,json,pathlib,shutil,subprocess,time
root=pathlib.Path.cwd();out=root/'target/verification/cave-adoption-r6-2026-11-06'
def inputs():
 paths=[root/name for name in ('Cargo.toml','Cargo.lock','flake.nix','flake.lock','rust-toolchain.toml') if (root/name).is_file()]
 for folder in ('src','assets','vendor','.cargo'):
  paths.extend(p for p in (root/folder).rglob('*') if p.is_file())
 return {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest()for p in sorted(paths)}
before=inputs();(out/'inputs-before.json').write_text(json.dumps(before,indent=2)+'\n')
(out/'head-before.txt').write_text(subprocess.check_output(['git','rev-parse','HEAD'],text=True))
commands = [('clippy', ['nix', 'develop', '--command', 'cargo', 'clippy', '--locked', '--release', '--all-targets', '-j2', '--', '-D', 'warnings']), ('full-tests', ['nix', 'develop', '--command', 'cargo', 'test', '--locked', '--release', '--lib', '-j2', '--', '--nocapture']), ('capture-build', ['nix', 'develop', '--command', 'cargo', 'build', '--locked', '--bin', 'capture', '-j2', '--target-dir', 'target/market-visual', '--config', 'profile.dev.opt-level=0', '--config', 'profile.dev.package."*".opt-level=0', '--config', 'profile.dev.debug=0'])]
results=[]
for name,args in commands:
 assert inputs()==before
 print('STAGE',name,'begin',flush=True);start=time.monotonic()
 with (out/(name+'.log')).open('w')as log:
  result=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT,timeout=1500)
 results.append({'stage':name,'args':args,'exit':result.returncode,'seconds':time.monotonic()-start})
 (out/'processes.json').write_text(json.dumps(results,indent=2)+'\n')
 print('STAGE',name,'terminal',result.returncode,flush=True)
 assert inputs()==before,'source/assets changed during verification'
 assert result.returncode==0,name+' failed; preserve actual failure before changing source'
(out/'inputs-after.json').write_text(json.dumps(inputs(),indent=2)+'\n')
(out/'head-after.txt').write_text(subprocess.check_output(['git','rev-parse','HEAD'],text=True))
shutil.copy2(root/'target/market-visual/debug/capture',out/'capture')
(out/'binary.json').write_text(json.dumps({'sha256':hashlib.sha256((out/'capture').read_bytes()).hexdigest(),'bytes':(out/'capture').stat().st_size},indent=2)+'\n')
print('COMBINED_SOURCE_CHECKS_PASS; native production/input/layout acceptance still required',flush=True)
