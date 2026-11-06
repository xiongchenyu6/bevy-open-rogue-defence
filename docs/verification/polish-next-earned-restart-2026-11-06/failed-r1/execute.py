import json,pathlib,subprocess,time,hashlib,sys
root=pathlib.Path.cwd();out=root/'target/verification/polish-next-earned-restart-2026-11-06';state=root/'target/verification/polish-next-earned-profile-2026-11-06';sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();expected='ffab0a0d890e9ddd012de5516a7ef6de6a444a7b48108a1ae0fcecd43433cfbb'
authorization=json.loads((root/'target/verification/polish-next-package-2026-11-06/authorization.json').read_text());assert authorization['earned_restart_authorized'] is True
assert [p.name for p in state.iterdir()]==['campaign.save'] and sha(state/'campaign.save')==expected
for f,h in json.loads((out/'harness-provenance.json').read_text())['files'].items():assert sha(out/'harness-source'/f)==h
args=json.loads((out/'command.json').read_text());(out/'executed-command.json').write_text(json.dumps(args,indent=2)+'\n');started=time.monotonic()
try:
 with (out/'runtime.log').open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT,timeout=300)
 result=dict(exit=p.returncode,elapsed=time.monotonic()-started)
except subprocess.TimeoutExpired:
 result=dict(timeout=True,elapsed=time.monotonic()-started)
finally:
 subprocess.run(['docker','stop','-t','15','rogue-polish-next-earned-restart-nov6'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
(out/'process.json').write_text(json.dumps(result,indent=2)+'\n');print(result,flush=True)
(out/'profile-after.json').write_text(json.dumps(dict(source_profile=str(state),files={str(p.relative_to(state)):sha(p) for p in state.rglob('*') if p.is_file()}),indent=2)+'\n')
assert result.get('exit')==0,'Actual short earned restart incomplete; preserve failure'
r=json.loads((out/'result.json').read_text());assert r['completed'] is True and r['result']=='PASS' and r['quit_exit']==0
assert r['attributes']['strength']==34 and r['equipment_count']==0 and len(r['six_cleared_public_ocr'])==6
assert r['resume_countdown']['after']<r['resume_countdown']['before']
assert r['save_after_sha256']==expected==sha(state/'campaign.save')
print('GENUINE_FFAB_RESTART_PASS',flush=True)
