from pathlib import Path
import json,hashlib,sys
sys.dont_write_bytecode=True
p=Path(__file__).resolve().parent;read=lambda n:json.loads((p/n).read_text())
for n,h in read('evidence-hashes.json').items():assert hashlib.sha256((p/n).read_bytes()).hexdigest()==h,n
sys.path.insert(0,str(p/'chapter6/harness-source'))
from campaign_runtime import read_progress
from campaign_equipment_runtime import selected_inventory_name,inventory_icons
from smoke_runtime import normalized
r=read('chapter6/result.json');assert read('chapter6-process.json')['exit']==0
assert r['completed_run'] and r['victory'] and r['terminal']=='CHAPTER CLEARED' and r['actual_speed']==1
assert r['package_version']['git_commit']=='d4540c44df479041fe4ff4fdb818f8fb2f280f99' and r['package_version']['binary_sha256']=='7dce2d9be5a8a8d8fa9027b1adc5fc7b0f3705b6ffadf6cb8652326e0b7aec24'
assert r['requested_seed']=='0xA11CE' and int(r['actual_run_seed'],16)==int('A11CF',16)
assert len(r['equipment_purchases'])==11 and len(r['equipment_resume_proofs'])==12
owned=[];total=0
for item in r['equipment_purchases']:
 owned.append(normalized(item['name']));n=len(owned);assert item['wallet_before']-item['wallet_after']==item['price']==item['offer_ocr']['price']
 assert item['inventory_verified'] and item['inventory_count']==n and len(set(item['owned_names']))==n and set(item['owned_names'])==set(owned)
 assert len(item['inventory_inspections'])==n and {selected_inventory_name(x['detail_ocr']) for x in item['inventory_inspections']}==set(owned)
 assert {tuple(x['icon']) for x in item['inventory_inspections']}==set(inventory_icons(n));total+=n
 prefix=item['before'].removesuffix('-before.png')
 for index in range(n):assert (p/'chapter6'/f'{prefix}-inventory-icon-{index}.png').exists()
 assert any(x['visit']==item['visit'] and x['phase']=='inventory fresh closed verified' for x in r['equipment_visit_timings'])
assert total==66
assert {x['visit'] for x in r['equipment_resume_proofs']}==set(range(1,13)) and all(x['after']<x['before'] for x in r['equipment_resume_proofs'])
assert r['equipment_resume_proofs'][-1]=={'visit':12,'before':32,'after':22}
observations=r['equipment_purchase_observations'];assert len([x for x in observations if x.get('input')=='buy_click'])==11
assert len([x for x in observations if x.get('exact_debit')])==11 and all(x['wallet']==x['hud'] for x in observations if 'wallet' in x)
a,e,u=r['initial_progress'],r['earned_progress'],r['earned_upgrade'];assert a=={'unlocked':6,'currency':645,'cleared':'111110','upgrades':[6,0,0,0,0,0]}
assert e==dict(a,currency=885,cleared='111111') and e['currency']-a['currency']==240
assert r['strength_upgrade_cost']==255 and u==dict(e,currency=630,upgrades=[7,0,0,0,0,0])
assert r['base_attributes']['strength']==32 and r['upgraded_attributes']['strength']==34
assert r['saved_progress_survives_restart'] and r['earned_strength_applies_after_restart'] and r['normal_quit_twice']
assert read_progress(p/'campaign-initial.save')==a and read_progress(p/'campaign-final.save')==u
assert hashlib.sha256((p/'campaign-initial.save').read_bytes()).hexdigest()=='4c9a0973fa90b3f75838ccb1999267be31ff8ff9b896a8c9b165ae5ac3e129f9'
assert hashlib.sha256((p/'campaign-final.save').read_bytes()).hexdigest()=='ffab0a0d890e9ddd012de5516a7ef6de6a444a7b48108a1ae0fcecd43433cfbb'
snap=read('harness-provenance.json');assert snap['harness_commit']=='d4540c44df479041fe4ff4fdb818f8fb2f280f99'
for n,h in snap['harness_files'].items():assert hashlib.sha256((p/'chapter6/harness-source'/n).read_bytes()).hexdigest()==h
print('PASS: actuald454 C6victory,11exactdebits/66ownedinspections/12resumes,+240/-255,32→34,savechecksum,twoquits; requestedA11CE actualA11CF')
