from pathlib import Path
import hashlib,json
p=Path(__file__).resolve().parent
h=json.loads((p/'evidence-hashes.json').read_text())
for n,v in h['files'].items():assert hashlib.sha256((p/n).read_bytes()).hexdigest()==v,n
r=json.loads((p/'chapter6/result.json').read_text());process=json.loads((p/'chapter6-process.json').read_text())
assert process['exit']==0 and r['probe_completed'] and r['normal_quit'] and r['quit_exit']==0
records=r['buy_os_probe'];assert [x['input'] for x in records if 'input' in x]==['ordinary_click']
reads=[x for x in records if 'wallet' in x];assert [x['wallet'] for x in reads]==[336,336,176]
assert all(x['wallet']==x['hud'] for x in reads)
assert reads[-1]['exact_debit'] and reads[0]['offer']==reads[1]['offer']
assert len(r['equipment_purchases'])==1
item=r['equipment_purchases'][0];assert item['wallet_before']-item['wallet_after']==item['price']==160
assert item['inventory_verified'] and item['inventory_count']==1 and item['owned_names']==['windboots']
assert r['equipment_resume_proofs'][-1]['after']<r['equipment_resume_proofs'][-1]['before']
assert not r['completed_run'] and not r['victory'] and h['save_unchanged']
print('PASS: hashes, one ordinary click, delayed exact debit, complete Wind Boots ownership, resume, normal quit; not chapter completion')
