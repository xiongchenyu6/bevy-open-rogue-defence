from pathlib import Path
import json,hashlib
p=Path(__file__).resolve().parent
manifest=json.loads((p/'manifest.json').read_text())
assert set(manifest)=={str(q.relative_to(p))for q in p.rglob('*')if q.is_file()and q.name!='manifest.json'}
for name,row in manifest.items():
 data=(p/name).read_bytes();assert len(data)==row['bytes']and hashlib.sha256(data).hexdigest()==row['sha256'],name
assert json.loads((p/'inputs-before.json').read_text())==json.loads((p/'inputs-after.json').read_text())
processes=json.loads((p/'manager-processes.json').read_text());assert [r['exit']for r in processes]==[0,0,1,0]
cases=json.loads((p/'outcomes.json').read_text());assert len(cases)==2
assert all(c['valid']and c['returncode']==0 and not c['timed_out']and c['last']['finished']and c['last']['victory']for c in cases)
r=json.loads((p/'comparison.json').read_text());assert not r['recorded_numeric_equal']and r['first_numeric']['frame']==81007 and r['first_contact_nonidentity']==46770
assert r['actor_counts']==[100903,99620]and r['contact_counts']==[81893,81117]
window=json.loads((p/'glacier-contact-window.json').read_text())['rows']
for side,damage,hp in [('a','3710.646484375','103200.1015625'),('b','6493.63134765625','100417.1171875')]:
 target=[r for r in window[side]if r['contact']['enemy_bits']=='403726901544'];assert len(target)==1
 assert target[0]['contact']['hp_before']=='106910.75'and target[0]['contact']['raw_damage']==damage and target[0]['contact']['hp_after']==hp
print('Archived post-order full replay NEGATIVE verified; equivalence remains unproven')
