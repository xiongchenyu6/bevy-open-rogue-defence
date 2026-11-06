from pathlib import Path
import importlib.util,json,struct
r=Path(__file__).parent
spec=importlib.util.spec_from_file_location('cmp',r/'analysis/compare-recorded.py'); m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
f=lambda n:struct.unpack('<f',struct.pack('<I',n))[0]
o={}
for side in 'ab':
 p=r/f'frozen/target/verification/post-glacier-full-{side}-nov6/active-h2-behind-sbeef-c5-t0'
 events=list(m.contacts(p.with_suffix('.stderr')))
 rows={v['frame']:v for v in m.traces(p.with_suffix('.jsonl')) if 94470<=v['frame']<=94475}
 selected=[]
 for i,e in enumerate(events):
  if 93205<=i<=93243: selected.append({'index':i,**e})
 o[side]={'contacts':selected,'frames':rows}
 for e in selected:
  if 'Frost' in e['profile']:
   print(side,e['index'],e['elapsed_bits'],e['projectile'],e['enemy'],e['base_damage'],e['profile'],e['critical_roll'])

(r/'frost-link-raw.json').write_text(json.dumps(o,indent=2))
