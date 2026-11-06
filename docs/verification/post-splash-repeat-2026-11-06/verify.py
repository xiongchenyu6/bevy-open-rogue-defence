from pathlib import Path
import json,hashlib
p=Path(__file__).resolve().parent
m=json.loads((p/'manifest.json').read_text());assert set(m)=={str(q.relative_to(p))for q in p.rglob('*')if q.is_file()and q.name!='manifest.json'}
for n,r in m.items():
 b=(p/n).read_bytes();assert len(b)==r['bytes']and hashlib.sha256(b).hexdigest()==r['sha256'],n
assert json.loads((p/'inputs-before.json').read_text())==json.loads((p/'inputs-after.json').read_text())
steps=json.loads((p/'manager-processes.json').read_text());assert [x['exit']for x in steps]==[0,0,0,0]
a=json.loads((p/'checkpoint-comparison.log').read_text());assert a['prefix_equal']and a['complete_replay']and a['status']=='complete_equal_replay'and a['checkpoint_counts']==[57,57]
r=json.loads((p/'comparison.json').read_text());assert r['recorded_numeric_equal']and r['actor_counts']==[100735,100735]and r['contact_counts']==[96172,96172]
assert r['first_numeric']is None and r['first_missing_row']is None and r['first_contact_nonidentity']is None
assert r['first_raw_order']['frame']==5031 and r['first_contact_identity']==11684
assert json.loads((p/'first-numeric-context.json').read_text())is None and json.loads((p/'first-contact-context.json').read_text())is None
rows=json.loads((p/'outcomes.json').read_text());assert len(rows)==2
assert all(c['valid']and not c['timed_out']and c['returncode']==0 and c['last']['finished']and c['last']['victory']for c in rows)
assert rows[0]['last']==rows[1]['last']
s=(p/'unit-terminal-state.txt').read_text();assert 'ActiveState=inactive'in s and 'Result=success'in s and 'ExecMainStatus=0'in s
print('Archived complete recorded-projection pair PASS; hidden/global equivalence unproven')
