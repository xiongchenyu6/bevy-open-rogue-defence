from pathlib import Path
import hashlib,json
p=Path(__file__).resolve().parent;r=json.loads((p/'result.json').read_text())
assert r['completed'] is True and r['quit_exit']==0
assert json.loads((p/'process.json').read_text())['exit']==0
assert r['package_version']['git_commit']=='86cb08498e446a27b31554f8cb6dd3eb641e214b'
assert r['crafted_inventory_ocr']=={'count':1,'names':['astralprism']}
assert [x['names']for x in r['inventory_inspections']]==[['arclens'],['arclens','echocore'],['astralprism']]
assert [(x['name'],x['price'],x['wallet_before'],x['wallet_after'])for x in r['purchases']]==[('Arc Lens',180,320,140),('Echo Core',300,363,63)]
assert r['resume_countdown_after']<r['resume_countdown_before']
for name in ['initial-campaign.save','final-campaign.save']:
 assert hashlib.sha256((p/name).read_bytes()).hexdigest()=='da5bf0a9ca3f4032a265425f9989bda292d8c0c2622848157df6d2c0223ba825'
for name,sha in json.loads((p/'harness-snapshot.json').read_text())['files'].items():
 assert hashlib.sha256((p/'harness-source'/name).read_bytes()).hexdigest()==sha
assert json.loads((p/'restart-precondition-failure/process.json').read_text())['exit']==1
print('PASS archived 86 natural forge only; conditional restart precondition failure remains NOT VERIFIED')
