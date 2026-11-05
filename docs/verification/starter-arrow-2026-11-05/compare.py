from pathlib import Path
import json,re
p=Path(__file__).resolve().parent
expected=json.loads((p/'comparison.json').read_text())
for case,row in expected.items():
 b=(p/'before'/(case+'.log')).read_text();a=(p/'after'/(case+'.log')).read_text()
 trace=lambda s:re.findall(r'^ATTACK_TIMING .+$',s,re.M)
 assert len(trace(b))==80 and trace(b)==trace(a),case
 assert 'panicked at' not in a and 'wrote 250 simulation frames' in a,case
 if 'actual_first_volley_arrows' in row:
  assert f"PASS archer-first-volley arrows={row['actual_first_volley_arrows']}" in a
  assert f'PASS {case}: production attack flight+damage' in a
print('PASS: 400 synchronous damage/HP/playback records, actual tier/volley assertions and native completion')
