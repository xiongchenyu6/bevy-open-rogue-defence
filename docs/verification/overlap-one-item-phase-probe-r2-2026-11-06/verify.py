from pathlib import Path
import json,hashlib
p=Path(__file__).resolve().parent;h=json.loads((p/'evidence-hashes.json').read_text())
for n,v in h['files'].items():assert hashlib.sha256((p/n).read_bytes()).hexdigest()==v,n
r=json.loads((p/'chapter6/result.json').read_text());assert json.loads((p/'chapter6-process.json').read_text())['exit']==0
assert r['probe_completed'] and r['normal_quit'] and r['quit_exit']==0 and r['actual_speed']==1
assert not r['completed_run'] and not r['victory'] and h['save_unchanged']
assert len(r['equipment_purchases'])==1
x=r['equipment_purchases'][0];assert x['name']=='Wind Boots' and x['wallet_before']==188 and x['wallet_after']==28 and x['price']==160
assert x['inventory_verified'] and x['inventory_count']==1 and x['owned_names']==['windboots']
assert [v['input'] for v in x['close_observations'] if 'input' in v]==['mouseDown','mouseUp']
assert any(v['visit']==x['visit'] and v['phase']=='inventory fresh closed verified' for v in r['equipment_visit_timings'])
t=r['equipment_resume_proofs'][-1];assert t['visit']==x['visit'] and t['before']==1147 and t['after']==1137
assert [v['input'] for v in r['equipment_purchase_observations'] if 'input' in v]==['buy_click']
assert r['equipment_visit_timings'][-1]['phase']=='resume verified'
print('PASS: actual one item exact debit/allowned/Close/freshclosed/doubletimer/normalquit0/saveunchanged; not chapter completion')
