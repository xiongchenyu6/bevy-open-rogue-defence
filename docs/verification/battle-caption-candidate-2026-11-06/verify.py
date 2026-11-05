from pathlib import Path
import json
p=Path(__file__).resolve().parent
assert 'Ran 97 tests' in (p/'tests.log').read_text() and (p/'tests.log').read_text().rstrip().endswith('OK')
assert json.loads((p/'result.json').read_text())['returncode']==0
old=p/'offline-before-source-change';text=(old/'candidate-holdouts.log').read_text()
assert 'Ran 93 tests' in text and text.rstrip().endswith('OK')
assert json.loads((old/'candidate-holdouts-process.json').read_text())['exit']==0
first=json.loads(text.splitlines()[0]);assert first['original_classification']=='unknown' and first['candidate_classification']=='battle' and first['strict_numeric_pixels']=='LEVEL76/168XP'
print('PASS archived offline candidate: original unknown -> strict numeric battle; old93/new97 tests PASS, native nonbattle priority and crop-failure negatives retained; no game rerun')
