import gzip,hashlib,json,sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
def obj(n):return json.loads(raw[n])
f=obj('forge/result.json');r=obj('restart/result.json')
assert f['completed'] is True and f['quit_exit']==0 and r['completed'] is True and r['quit_exit']==0
assert obj('forge-process.json')['returncode']==0 and obj('restart-process.json')['returncode']==0
assert [(x['name'],x['price'],x['wallet_before'],x['wallet_after']) for x in f['purchases']]==[('Arc Lens',180,300,120),('Echo Core',300,332,32)]
assert f['acquired_materials']==['Arc Lens','Echo Core'] and f['actual_speed']==1
assert f['inventory_verified'] is True and f['crafted_inventory_ocr']=={'count':1,'names':['astralprism']}
assert (f['resume_countdown_before'],f['resume_countdown_after'])==(924,909)
refresh=f['refresh_observations'];assert len(refresh)==2 and refresh[0]['fee']==60 and refresh[0]['wallet_before']==392
assert refresh[1]['wallet']==refresh[1]['hud']==332 and refresh[1]['exact_debit'] is True and refresh[1]['stock_changed'] is True
assert r['attributes']['hero']=='StormSavant' and r['attributes']['strength']==30 and r['attributes']['speed']==1
assert r['equipment_count']==0 and r['resume_countdown']=={'before':1469,'after':1461}
expected='da5bf0a9ca3f4032a265425f9989bda292d8c0c2622848157df6d2c0223ba825'
assert (D/'initial-campaign.save').read_bytes()==(D/'final-campaign.save').read_bytes()
assert sha((D/'initial-campaign.save').read_bytes())==expected
assert f['initial_save_sha256']==f['final_save_sha256']==r['save_before_sha256']==r['save_after_sha256']==expected
assert f['initial_progress']==f['final_progress']=={'unlocked':5,'currency':575,'cleared':'111100','upgrades':[5,0,0,0,0,0]}
join=json.loads((D/'source-join.json').read_text());h=obj('forge/harness-snapshot.json');assert join['harness_commit']==h['harness_commit'] and join['package_commit']==h['package_commit']
assert len(join['files'])==6
for n,hsh in join['files'].items():assert sha(raw['forge/harness-source/'+n])==hsh
assert json.loads((D/'package/artifact.json').read_text())['source_commit']==join['package_commit']==f['package_version']['git_commit']==r['package_version']['git_commit']
state=raw['unit-state.txt'].decode();assert 'Result=success' in state and 'ExecMainStatus=0' in state
images=json.loads((D/'local-image-manifest.json').read_text());selected=json.loads((D/'selected-images.json').read_text());assert len(selected)==14
for n in selected:assert sha((D/'frames'/n).read_bytes())==images[n]['sha256']
attributes=f['magic_penetration_observations'];assert len(attributes)==3
assert attributes[0]['value']==attributes[0]['expected']==10
assert attributes[1]['expected']==20 and attributes[1]['read_error'] and attributes[1]['input']=='scroll_down_2'
assert attributes[2]['value']==attributes[2]['expected']==20
assert attributes[0]['rows'] and attributes[2]['rows']
print('PASS exact archive/raw bytes,natural purchases/refresh,actual10-to20% attribute observations,Prism ownership,same-profile restart and unchanged earned save.')
