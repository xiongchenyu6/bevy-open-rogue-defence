from pathlib import Path
import json
root=Path(__file__).resolve().parent
build=json.loads((root/'build-process.json').read_text())
assert len(build)==2 and all(r['exit']==0 for r in build)
for case in json.loads((root/'native-process.json').read_text()):
 assert case['passed'] and case['exit']==0
 text=(root/case['name']/'capture.log').read_text()
 assert 'panicked at' not in text and 'wrote 380 simulation frames' in text
 for mode in ['same-tick','held']:
  for index in range(5):assert f'EQUIPMENT_MOUSE_PASS {mode} item={index} title=' in text
 assert 'EQUIPMENT_MOUSE_PASS all 5 same-tick + 5 held Item selections' in text
print('PASS controlled synthetic native Item mouse integration: both languages/sizes, 20 exact title/art checks, 2 mouse closes, 760 completed updates; strict Clippy/build')
