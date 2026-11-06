from pathlib import Path
import hashlib,json
p=Path(__file__).resolve().parent
manifest=json.loads((p/'manifest.json').read_text())
assert set(manifest)=={str(q.relative_to(p))for q in p.rglob('*')if q.is_file()and q.name!='manifest.json'}
for name,row in manifest.items():
 data=(p/name).read_bytes();assert len(data)==row['bytes']and hashlib.sha256(data).hexdigest()==row['sha256'],name
red=(p/'red.log').read_text();assert 'FAILED. 2 passed; 1 failed'in red and 'unrelated allocation must not exchange Glacier critical assignments'in red
assert 'left: ([9886.838, 14935.336], 16263334312487546840)'in red and 'right: ([9935.336, 14886.838], 16263334312487546840)'in red
final=(p/'tests.log').read_text();assert 'ok. 740 passed; 0 failed; 1 ignored'in final
rows=[line for line in final.splitlines()if 'GLACIER_ORDER reverse='in line];assert len(rows)==4 and all('hp=[9935.336, 14886.838]'in row and 'rng=16263334312487546840'in row for row in rows)
assert 'glacier_third_real_pulse_freezes_both_targets ... ok'in final
assert json.loads((p/'validation-inputs-before.json').read_text())==json.loads((p/'validation-inputs-after.json').read_text())
steps=json.loads((p/'validation-processes.json').read_text());assert [s['stage']for s in steps]==['format','tests','clippy','sim-build']and all(s['exit']==0 for s in steps)
state=(p/'validation-unit-state.txt').read_text();assert 'ActiveState=inactive'in state and 'Result=success'in state and 'ExecMainStatus=0'in state
print('Archived Glacier local red/green PASS; full replay equivalence remains unproven')
