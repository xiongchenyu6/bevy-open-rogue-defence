from pathlib import Path
import hashlib,json
p=Path(__file__).resolve().parent;read=lambda n:json.loads((p/n).read_text())
for n,h in read('evidence-hashes.json').items():assert hashlib.sha256((p/n).read_bytes()).hexdigest()==h,n
f,s=read('forge/result.json'),read('restart/result.json');expected='da5bf0a9ca3f4032a265425f9989bda292d8c0c2622848157df6d2c0223ba825'
assert read('forge/process.json')['exit']==read('restart/process.json')['exit']==0
assert f['completed'] and f['quit_exit']==0 and f['inventory_verified']
assert [(v['name'],v['wallet_before'],v['wallet_after'],v['price']) for v in f['purchases']]==[('Arc Lens',332,152,180),('Echo Core',310,10,300)]
assert f['crafted_inventory_ocr']=={'count':1,'names':['astralprism']}
assert f['resume_countdown_before']==1003 and f['resume_countdown_after']==992
assert f['initial_save_sha256']==f['final_save_sha256']==s['save_before_sha256']==s['save_after_sha256']==expected
assert not f.get('refresh_observations') and all(not v.get('completed') for v in f['refresh_policy'])
assert s['completed'] and s['result']=='PASS' and s['quit_exit']==0 and s['equipment_count']==0
assert s['attributes']['strength']==30 and s['attributes']['level']==1 and s['attributes']['speed']==1
assert s['resume_countdown']=={'before':1449,'after':1439}
a,b=read('restart/profile-before.json'),read('restart/profile-after.json');assert a['copied'] is False and b['copied'] is False and a['exact_source_profile']==b['exact_source_profile']
assert hashlib.sha256((p/'campaign-after.save').read_bytes()).hexdigest()==expected
for folder in ['forge','restart']:
 snap=read(folder+'/harness-snapshot.json');assert snap['package_commit']=='d4540c44df479041fe4ff4fdb818f8fb2f280f99' and len(snap['files'])==6 and 'forge_refresh.py' in snap['files']
 for n,h in snap['files'].items():assert hashlib.sha256((p/folder/'harness-source'/n).read_bytes()).hexdigest()==h
print('PASS: natural ArcLens+EchoCore→Prism, complete owned/debit/resume/quit, SAMEprofile restart and unchangedsave; ZERO paidrefreshes, not C6')
