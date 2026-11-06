"""Read-only checks of actual earned-state compatibility and preserved interruption."""
from pathlib import Path
import hashlib,json,re,subprocess,collections
P=Path(__file__).resolve().parent;ROOT=P.parents[2];EXPECTED='ffab0a0d890e9ddd012de5516a7ef6de6a444a7b48108a1ae0fcecd43433cfbb';COMMIT='0c12230d3bf25c2b1a070c80950e5388a3b7b2ec'
sha=lambda b:hashlib.sha256(b).hexdigest();load=lambda p:json.loads(p.read_text());norm=lambda s:re.sub('[^a-z0-9]','',s.lower())
initial=(P/'initial-campaign.save').read_bytes();assert sha(initial)==EXPECTED
lines=initial.decode().splitlines();assert lines[:5]==['RCAMPAIGN1','6','630','111111','7,0,0,0,0,0']
h=0xcbf29ce484222325
for byte in ('\n'.join(lines[:5])+'\n').encode():h=((h^byte)*0x100000001b3)&((1<<64)-1)
assert h==int(lines[5],16)
old=load(P/'failed-r1/result.json');assert load(P/'failed-r1/process.json')['exit']==1 and old['completed'] is False and old.get('quit_exit') is None
assert (P/'failed-r1-profile-after/campaign.save').read_bytes()==initial
for name,profile in [('restart','profile-after'),('art-observation-r3','art-r3-profile-after')]:
 d=P/name;r=load(d/'result.json');assert load(d/'process.json')['exit']==0
 assert r['completed'] is True and r['result']=='PASS' and r['quit_exit']==0
 assert r['package_version']['git_commit']==COMMIT
 assert r['previous_package_commit']=='d4540c44df479041fe4ff4fdb818f8fb2f280f99'
 assert r['progress_before']==dict(unlocked=6,currency=630,cleared='111111',upgrades=[7,0,0,0,0,0])
 assert norm(r['menu_shards_ocr'][0])=='630legacyshards'
 assert {x['chapter'] for x in r['six_cleared_public_ocr']}==set(range(1,7))
 assert all(any(norm(row[0])=='cleared'for row in x['rows'])for x in r['six_cleared_public_ocr'])
 assert r['attributes']['strength']==34 and r['attributes']['level']==1 and r['attributes']['speed']==1 and r['attributes']['hero']=='StormSavant'
 assert any(re.match(r'^Strength\s+34\.0(?:\s|$)',row[0])for row in r['attributes']['strength_ocr'])
 assert r['equipment_count']==0 and r['resume_countdown']['after']<r['resume_countdown']['before']
 assert r['save_before_sha256']==r['save_after_sha256']==EXPECTED and (P/profile/'campaign.save').read_bytes()==initial
 assert set(load(d/'profile-before.json')['initial_files'])=={'campaign.save'}
 request=r['commander_click'];assert request['held_after_down_return_seconds']>=.15
 c=request['caption_ocr'];assert norm(c[0])=='stormsavant' and request['target']==[(c[1]+c[3])//2,(c[2]+c[4])//2]
 assert r['commander_settlement'][-1]['state']=='battle' and not r['commander_settlement'][-1]['chooser']
 for f,h in load(d/'harness-provenance.json')['files'].items():
  body=(d/'harness-source'/f).read_bytes();assert sha(body)==h
  if f!='ffab_restart_runtime.py':assert body==subprocess.check_output(['git','show',COMMIT+':tools/desktop/'+f],cwd=ROOT)
 args=load(d/'executed-command.json');assert args[args.index('--network')+1]=='none'
 assert any('dst=/game,readonly'in x for x in args) and any('dst=/previous-proof,readonly'in x for x in args)
 assert 'GENUINE_FFAB_RESTART_PASS' in (d/'manager.log').read_text()
# Independent non-background comparison, not an exact loading-time assertion.
def pixels(path):
 raw=subprocess.check_output(['convert',str(path),'-crop','200x128+565+155','+repage','-depth','8','rgba:-'],stderr=subprocess.DEVNULL)
 assert len(raw)==200*128*4;return list(zip(*(iter(raw),)*4))
a=pixels(P/'restart/restart-empty-inventory.png');b=pixels(P/'art-observation-r3/restart-empty-inventory.png');c=pixels(P/'art-observation-r3/restart-empty-inventory-later.png')
assert len(set(a))==1 and len(set(c))==6653 and sum(x!=y for x,y in zip(a,c))==12161 and b==c
assert load(P/'art-observation-r3/result.json')['empty_rack_late_observation']['wait_seconds']==6
if (P/'files.sha256.json').exists():
 for f,h in load(P/'files.sha256.json').items():assert sha((P/f).read_bytes())==h,f
print('GENUINE_FFAB_ARCHIVE_VERIFIED R1incomplete; R2child0/34/gear0/1456->1447; R3child0/rack-visible; untouchedffab; no new chapter clear')
