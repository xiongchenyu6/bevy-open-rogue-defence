from pathlib import Path
import hashlib,json
p=Path(__file__).resolve().parent
m=json.loads((p/'manifest.json').read_text());assert set(m)=={str(q.relative_to(p))for q in p.rglob('*')if q.is_file()and q.name!='manifest.json'}
for n,row in m.items():
 b=(p/n).read_bytes();assert len(b)==row['bytes']and hashlib.sha256(b).hexdigest()==row['sha256'],n
assert json.loads((p/'inputs-before.json').read_text())==json.loads((p/'inputs-after.json').read_text())
steps=json.loads((p/'manager-processes.json').read_text());assert [s['exit']for s in steps]==[0,0,1,0]
rows=json.loads((p/'outcomes.json').read_text());assert len(rows)==2 and all(r['valid']and r['returncode']==0 and not r['timed_out']and r['last']['finished']and r['last']['victory']for r in rows)
r=json.loads((p/'comparison.json').read_text());assert not r['recorded_numeric_equal']and r['first_numeric']['frame']==94475 and r['first_contact_nonidentity']==93232
assert r['actor_counts']==[100727,100735]and r['contact_counts']==[96198,96172]
c=json.loads((p/'first-contact-context.json').read_text());assert all('element: Frost, class: Attack'in c[s]['profile']and 'market_proc: true, card_proc: true'in c[s]['profile']for s in ['a','b'])
a,b=[r['last']for r in rows]
for k in ['level','kills','deaths','gate_breaks','attacks','damage_skill_casts','skill_ranks','gold','wood']:assert a[k]==b[k],k
assert a['rng_state']!=b['rng_state']
print('Archived late post-Glacier exact-state NEGATIVE verified; no equivalence claim')
