import pathlib,json,hashlib,subprocess,shutil
r=pathlib.Path.cwd();o=pathlib.Path(__file__).resolve().parent;v=r/'target/verification/glacier-order-2026-11-06'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
state=dict(line.split('=',1)for line in subprocess.check_output(['systemctl','--user','show','rogue-glacier-order-validation-nov6','-p','ActiveState','-p','Result','-p','ExecMainStatus'],text=True).splitlines())
assert state['ActiveState']=='inactive' and state['Result']=='success' and state['ExecMainStatus']=='0',state
processes=json.loads((v/'validation-processes.json').read_text());assert len(processes)==4 and all(p['exit']==0 for p in processes)
before=json.loads((v/'validation-inputs-before.json').read_text());assert before==json.loads((v/'validation-inputs-after.json').read_text())
assert all(sha(r/n)==h for n,h in before.items())
assert not subprocess.check_output(['git','diff','--name-only'],text=True).strip(),'commit validated source before freezing'
head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
# The existing binary was built by the actual completed validation job. Commit
# changes no input bytes; reuse that exact build rather than compile it again.
inputs=dict(before)
for n in ['tools/run_balance_matrix.py','tools/compare_balance_replays.py']:inputs[n]=sha(r/n)
f=o/'frozen';f.mkdir();shutil.copytree(r/'assets',f/'assets')
for n in ['tools/run_balance_matrix.py','tools/compare_balance_replays.py','src/game/campaign_progress.rs']:
 p=f/n;p.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(r/n,p)
pin={str(p.relative_to(f)):sha(p)for p in sorted(f.rglob('*'))if p.is_file()}
for n in pin:(f/n).chmod(0o444)
shutil.copy2(r/'target/release/balance_sim',f/'balance-sim-snapshot');(f/'balance-sim-snapshot').chmod(0o555)
for name,value in [('inputs-before',inputs),('inputs-after',inputs),('frozen-files',pin),('binary',{'sha256':sha(f/'balance-sim-snapshot'),'bytes':(f/'balance-sim-snapshot').stat().st_size,'source_commit':head,'build_receipt':str(v/'validation-processes.json'),'build_input_manifest_sha256':sha(v/'validation-inputs-after.json'),'reused_completed_validated_build':True})]:
 (o/(name+'.json')).write_text(json.dumps(value,indent=2)+'\n')
(o/'source-commit.txt').write_text(head+'\n');(o/'validation-unit-terminal.json').write_text(json.dumps(state,indent=2)+'\n')
assert all(sha(r/n)==h for n,h in inputs.items())and all(sha(f/n)==h for n,h in pin.items())
print('EXACT_VALIDATED_BUILD_FROZEN',head,'asset_files',len([p for p in(f/'assets').rglob('*')if p.is_file()]),flush=True)
