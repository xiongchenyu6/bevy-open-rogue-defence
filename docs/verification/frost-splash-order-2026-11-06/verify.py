from pathlib import Path
import json,hashlib
p=Path(__file__).resolve().parent
m=json.loads((p/'manifest.json').read_text());assert set(m)=={str(q.relative_to(p))for q in p.rglob('*')if q.is_file()and q.name!='manifest.json'}
for n,r in m.items():
 b=(p/n).read_bytes();assert len(b)==r['bytes']and hashlib.sha256(b).hexdigest()==r['sha256'],n
invalid=(p/'red.log').read_text();assert 'FAILED. 0 passed; 3 failed'in invalid and 'left: 30.000002'in invalid
red=(p/'red2.log').read_text();assert 'FAILED. 2 passed; 1 failed'in red and 'physical neighbors must retain their splash critical rolls'in red
assert 'left: ([9970.0, 14947.5], 5218726287330236675)'in red and 'right: ([9947.5, 14970.0], 5218726287330236675)'in red
final=(p/'tests.log').read_text();assert 'ok. 743 passed; 0 failed; 1 ignored'in final
rows=[x for x in final.splitlines()if 'FROST_MARKET_SPLASH reverse='in x];assert len(rows)==6
for row in rows:
 assert 'rng=5218726287330236675'in row
 if 'tied=true'in row:assert 'hp=[9970.0, 14947.5]'in row
 else:assert 'hp=[9947.5, 14970.0]'in row
assert sum('tied=true'in row for row in rows)==4
assert json.loads((p/'validation-inputs-before.json').read_text())==json.loads((p/'validation-inputs-after.json').read_text())
steps=json.loads((p/'validation-processes.json').read_text());assert [s['stage']for s in steps]==['format','tests','clippy','sim-build']and all(s['exit']==0 for s in steps)
s=(p/'validation-unit-state.txt').read_text();assert 'ActiveState=inactive'in s and 'Result=success'in s and 'ExecMainStatus=0'in s
trace=json.loads((p/'trace/frost-distance-evidence.json').read_text())
for side in ['a','b']:assert [x['d2_bits']for x in trace[side]['all_neighbor_candidates_sorted_distance_ID'][:2]]==[1072524639]*2
print('Archived inherited Frost market-splash local red/green PASS; full replay unproven')
