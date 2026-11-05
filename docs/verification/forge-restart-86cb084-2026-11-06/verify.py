from pathlib import Path
import hashlib,json
p=Path(__file__).resolve().parent;r=json.loads((p/'result.json').read_text())
assert r['completed'] is True and r['result']=='PASS' and r['quit_exit']==0
process=json.loads((p/'process.json').read_text());assert process['exit']==0 and process['elapsed']<180
assert r['package_version']['git_commit']=='86cb08498e446a27b31554f8cb6dd3eb641e214b'
assert r['equipment_count']==0 and r['attributes']['strength']==30 and r['attributes']['level']==1
assert r['resume_countdown']['after']<r['resume_countdown']['before']
assert r['save_after_sha256']=='da5bf0a9ca3f4032a265425f9989bda292d8c0c2622848157df6d2c0223ba825'
before=json.loads((p/'profile-before.json').read_text());after=json.loads((p/'profile-after.json').read_text())
assert before==after and before['copied'] is False
assert set(before['files'])=={'campaign.save','config/rogue-commander/tutorial-dismissed'}
command=json.loads((p/'command.json').read_text())
assert '--network' in command and 'none' in command
state=[x for x in command if 'dst=/earned-state' in x];assert len(state)==1 and '/vortex-earned-forge-2026-11-06/forge/state,dst=/earned-state' in state[0]
assert any('/vortex-earned-forge-2026-11-06/forge,dst=/previous-proof,readonly' in x for x in command)
for name,sha in json.loads((p/'harness-snapshot.json').read_text())['files'].items():
 assert hashlib.sha256((p/'harness-source'/name).read_bytes()).hexdigest()==sha
obs=r['inventory_close_observations'];down=[x for x in obs if x.get('input')=='mouseDown'];up=[x for x in obs if x.get('input')=='mouseUp']
assert len(down)==len(up)==1 and up[0]['held_seconds']<10
assert down[0]['wall']<up[0]['wall']<down[0]['hold_deadline']<=down[0]['operation_deadline']
assert obs[-1]['phase']=='operation_end' and obs[-1]['elapsed']<20
print('PASS archived actual 86 same-profile restart: Strength30, empty gear, countdown advanced, quit0, unchanged profile; bounded one held Close')
