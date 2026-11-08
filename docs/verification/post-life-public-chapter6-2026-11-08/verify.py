import argparse,ast,collections,gzip,hashlib,json,re
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('archive',type=Path);p.add_argument('--local-root',type=Path);args=p.parse_args();D=args.archive.resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
def obj(n):return json.loads(raw[n])
r=obj('chapter6/result.json');process=obj('process.json');provenance=obj('provenance.json')
assert process['returncode']==0 and process['host_profile_unchanged'] and process['harness_unchanged']
assert process['host_profile_after']==provenance['host_profile_before']=='060649ac32b34ce2cda231a9cf1ebcdfd8e8c12f0b0b3f94e527714f6993d911'
assert r['stage']==5 and r['continued'] and r['requested_speed']==r['actual_speed']==1
assert r['completed_run'] and r['victory'] and r['terminal']=='CHAPTER CLEARED' and r['normal_quit_twice']
assert r['requested_seed']=='0xA11CE' and r['actual_run_seed']=='0x00000000000a11cf' and r['actual_seed_ocr']=='SEED 0x00000000000A11CF'
initial={'unlocked':6,'currency':645,'cleared':'111110','upgrades':[6,0,0,0,0,0]};earned={'unlocked':6,'currency':885,'cleared':'111111','upgrades':[6,0,0,0,0,0]};final={'unlocked':6,'currency':630,'cleared':'111111','upgrades':[7,0,0,0,0,0]}
assert r['initial_progress']==initial and r['earned_progress']==earned and r['earned_upgrade']==final
assert r['earned_progress']['currency']-r['initial_progress']['currency']==240
assert r['strength_upgrade_cost']==255 and earned['currency']-final['currency']==255
assert r['saved_progress_survives_restart'] and r['earned_strength_applies_after_restart']
assert r['base_attributes']['hero']==r['upgraded_attributes']['hero']=='StormSavant'
assert r['base_attributes']['level']==r['upgraded_attributes']['level']==1
assert r['base_attributes']['strength']==32 and r['upgraded_attributes']['strength']==34
def save(path):
 lines=path.read_text().splitlines();assert len(lines)==6 and lines[0]=='RCAMPAIGN1';h=0xcbf29ce484222325
 for byte in ('\n'.join(lines[:5])+'\n').encode():h=((h^byte)*0x100000001b3)&((1<<64)-1)
 assert int(lines[5],16)==h
 return {'unlocked':int(lines[1]),'currency':int(lines[2]),'cleared':lines[3],'upgrades':list(map(int,lines[4].split(',')))}
assert save(D/'initial-campaign.save')==initial and save(D/'final-campaign.save')==final
assert sha((D/'initial-campaign.save').read_bytes())==provenance['initial_save_sha256']=='4c9a0973fa90b3f75838ccb1999267be31ff8ff9b896a8c9b165ae5ac3e129f9'
def norm(s):return ''.join(c.lower() for c in s if c.isalnum())
inspections={x['visit']:x for x in r['equipment_inspections']};resume={x['visit']:x for x in r['equipment_resume_proofs']}
assert set(inspections)==set(resume)==set(range(1,13)) and len(r['equipment_visit_budgets'])==12
for x in resume.values():assert 0<x['after']<x['before']
owned=[];purchases=r['equipment_purchases'];assert len(purchases)==10
observations=r['equipment_purchase_observations'];assert sum(x.get('input')=='buy_click' for x in observations)==10
exact=[x for x in observations if x.get('exact_debit')];assert len(exact)==10
for index,(buy,ack) in enumerate(zip(purchases,exact),1):
 assert buy['wallet_before']-buy['wallet_after']==buy['price']>0
 assert int(buy['after_wallet_ocr'])==ack['hud']==ack['wallet']==buy['wallet_after']
 header_parts=buy['after_market_ocr'].split(' | ')
 headers=[re.fullmatch(r'(200|300|600)%: LV([A-Z0-9]+) KILLS (\d+)',part) for part in header_parts[:3]]
 assert all(headers) and {x[1] for x in headers}=={'200','300','600'}
 assert len({x[2] for x in headers})==1 and {int(x[3]) for x in headers}=={buy['wallet_after']}
 if not headers[0][2].isdigit():
  assert len(header_parts)==4 and header_parts[3].startswith('isolated level pixels: ')
  level_reads=ast.literal_eval(header_parts[3].split(': ',1)[1]);assert len({x[0] for x in level_reads})>=2
  levels=[re.fullmatch(r'LV(\d+)',x[1]) for x in level_reads];assert all(levels) and len({int(x[1]) for x in levels})==1 and int(levels[0][1])>0
 assert buy['inventory_verified'] is True and buy['inventory_count']==index
 owned.append(norm(buy['name']));assert collections.Counter(buy['owned_names'])==collections.Counter(owned)
 details=[norm(re.sub(r'^\[[A-Z]+\]\s*','',x['detail_ocr'])) for x in buy['inventory_inspections']];assert collections.Counter(details)==collections.Counter(owned)
 assert {x['index'] for x in buy['selection_observations'] if x.get('selected') is True}==set(range(index))
 close=buy['close_observations'];assert any(x.get('input')=='mouseDown' for x in close) and any(x.get('input')=='mouseUp' for x in close) and any(x.get('inventory_open') is False for x in close)
 visit=buy['visit'];assert visit in resume;scan=inspections[visit];assert scan['wallet']==buy['wallet_before']
 eligible=[x for x in scan['offers'] if x['category']=='Equipment' and x['price']<=scan['wallet']];choice=min(eligible,key=lambda x:(x['price'],x['index']));assert choice['price']==buy['price'] and norm(choice['name'])==norm(buy['name'])
 assert buy['offer_ocr']['category']=='Equipment' and norm(buy['offer_ocr']['tile_ocr'])==norm(buy['name']) and int(buy['offer_ocr']['price_ocr'])==buy['price']
assert r['equipment_pause_budget_seconds']==3480 and r['battle_timeout_seconds']==5280
join=json.loads((D/'source-join.json').read_text());assert len(join['harness_files'])==3
for name,h in join['harness_files'].items():assert sha(raw['chapter6/harness-source/'+name])==h
artifact=json.loads((D/'package/artifact.json').read_text());assert join['package_commit']==artifact['source_commit']==r['package_version']['git_commit']=='df51a6cddc908e5358f2cac9fa07068f46a6a5cc'
assert artifact['binary_sha256']==r['package_version']['binary_sha256']=='8d69ed5dec8fb555e6d605eb553ce1e12f0c7c38439a8d305a69564cff05e0d9'
review=obj('review/mapping.json');assert len(review)==16
images=json.loads((D/'original-image-index.json').read_text());selected=json.loads((D/'selected-images.json').read_text())
for name in selected:assert sha((D/'frames'/name).read_bytes())==images[name]['sha256']
for row in review:assert sha((D/'frames/chapter6'/row['source']).read_bytes())==row['sha256'] and (D/'review'/row['review']).exists()
if args.local_root:
 base=args.local_root/'target/verification/post-life-public-chapter6-2026-11-08-r9'
 for name,x in images.items():f=base/name;assert f.stat().st_size==x['bytes'] and sha(f.read_bytes())==x['sha256']
 print('PASS read-only all-original PNG hashes; no runtime launched')
state=(D/'unit-state.txt').read_text();assert 'Result=success' in state and 'ExecMainStatus=0' in state
print('PASS exact archive/raw bytes,10 exact debits/full owned sets,12 resumes,victory/reward/training/restart,seed distinction and earned-save checksums.')
