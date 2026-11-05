from pathlib import Path
import json,hashlib
p=Path(__file__).resolve().parent
assert json.loads((p/'process.json').read_text())['exit']==1
assert json.loads((p/'manager-process.json').read_text())['exit']==1
r=json.loads((p/'result.json').read_text());assert not r.get('completed') and r['purchases']==[] and r['acquired_materials']==[]
assert r['panel_open_attempts'][-1]['state']=='unknown' and r['panel_open_attempts'][-1]['requests']==0 and 'input'not in r['panel_open_attempts'][-1]
assert r['package_version']['git_commit']=='f5ef6ff8f37ecab467b5ba0baadb6decd8d49aaf'
assert hashlib.sha256((p/'actual-campaign.save').read_bytes()).hexdigest()=='da5bf0a9ca3f4032a265425f9989bda292d8c0c2622848157df6d2c0223ba825'
for name,sha in json.loads((p/'harness-snapshot.json').read_text())['files'].items():assert hashlib.sha256((p/'harness-source'/name).read_bytes()).hexdigest()==sha
ocr=json.loads((p/'audit/ocr.json').read_text());assert ocr['original_classification']==ocr['fresh_classification']=='unknown' and ocr['fresh_strict_numeric_level_pixels']=='LEVEL76/168XP'
restart=json.loads((p/'restart-not-started.json').read_text());assert not any(restart[k]for k in ['command_exists','runtime_exists','process_exists'])
print('PASS archived strict failure only: forge/manager exit1, zero purchases, unchanged save, actual battle misclassified unknown, restart NOT STARTED')
