import pathlib,json,hashlib,subprocess,shutil,time,os
r=pathlib.Path.cwd();o=pathlib.Path(__file__).resolve().parent;f=o/'frozen';f.mkdir()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def inputs():
 p=[r/n for n in ('Cargo.toml','Cargo.lock','flake.nix','flake.lock','build.rs','rust-toolchain.toml')if(r/n).is_file()]
 for d in ('src','assets','vendor','.cargo'):p.extend(x for x in(r/d).rglob('*')if x.is_file())
 p.extend(r/n for n in('tools/run_balance_matrix.py','tools/compare_balance_replays.py'))
 return {str(x.relative_to(r)):sha(x)for x in sorted(p)}
head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip();assert not subprocess.check_output(['git','diff','--name-only'],text=True).strip()
before=inputs();(o/'inputs-before.json').write_text(json.dumps(before,indent=2)+'\n');(o/'source-commit.txt').write_text(head+'\n')
shutil.copytree(r/'assets',f/'assets')
for n in ('tools/run_balance_matrix.py','tools/compare_balance_replays.py','src/game/campaign_progress.rs'):
 p=f/n;p.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(r/n,p)
pinned={str(x.relative_to(f)):sha(x)for x in sorted(f.rglob('*'))if x.is_file()}
for n in pinned:(f/n).chmod(0o444)
(o/'frozen-files.json').write_text(json.dumps(pinned,indent=2)+'\n');print('FROZEN',head,len(list((f/'assets').rglob('*'))),'asset paths',flush=True)
commands=[('bin-tests',['nix','develop','--command','cargo','test','--locked','--release','--bin','balance_sim','-j2']),('bin-build',['nix','develop','--command','cargo','build','--locked','--release','--bin','balance_sim','-j2'])];results=[]
for name,args in commands:
 assert inputs()==before;print('STAGE',name,'begin',flush=True);t=time.monotonic()
 with(o/(name+'.log')).open('w')as log:q=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT,timeout=1500)
 results.append({'stage':name,'args':args,'exit':q.returncode,'seconds':time.monotonic()-t});(o/'processes.json').write_text(json.dumps(results,indent=2)+'\n');assert inputs()==before;assert q.returncode==0,name
 print('STAGE',name,'terminal0',flush=True)
shutil.copy2(r/'target/release/balance_sim',f/'balance-sim-snapshot');(f/'balance-sim-snapshot').chmod(0o555)
(o/'binary.json').write_text(json.dumps({'sha256':sha(f/'balance-sim-snapshot'),'bytes':(f/'balance-sim-snapshot').stat().st_size,'source_commit':head},indent=2)+'\n');(o/'inputs-after.json').write_text(json.dumps(inputs(),indent=2)+'\n')
assert all(sha(f/n)==h for n,h in pinned.items());assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==head
print('POST_ORDER_SIMULATOR_BUILD_PASS; pair has not run yet',flush=True)
