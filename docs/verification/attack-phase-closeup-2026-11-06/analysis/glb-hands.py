from pathlib import Path
import json
p=Path('target/verification/attack-rhythm-2026-11-05/clip-audit.py');source=p.read_text().split('models = [audit(model)')[0]
out={}
for model,duration in [('ember_ranger',1.0666666666666667),('storm_savant',.9333333333333333)]:
 out[model]={}
 for hand in ['r','l']:
  ns={};exec(source.replace("'handslot.r'",repr('handslot.'+hand)),ns)
  ns['FRACTIONS']=[i/(duration*30) for i in range(round(duration*30)+1)]
  out[model][hand]=ns['audit'](model)
Path('target/verification/attack-phase-closeup-2026-11-06/analysis/glb-hands.json').write_text(json.dumps(out,indent=2))
for model,h in out.items():
 print(model)
 for i,(r,l) in enumerate(zip(h['r']['samples'],h['l']['samples'])):
  if i%2==0:print(i,r['source_seconds'],r['slot_position'],l['slot_position'])
