"""Offline comparison of actual native trace records. No gameplay runs."""
from pathlib import Path
import json,re,collections,math
r=Path(__file__).parent
ansi=re.compile(r'\x1b\[[0-9;]*m')
def events(path):
 for line in path.read_text().splitlines():
  if 'PROJECTILE_HIT ' not in line:continue
  body=ansi.sub('',line).split('PROJECTILE_HIT ',1)[1]
  fields=list(re.finditer(r'(?<!\S)([a-z_]+)=',body))
  yield {m[1]:body[m.end():fields[i+1].start() if i+1<len(fields) else len(body)].strip() for i,m in enumerate(fields)}
result={}
for label in ['desktop','phone']:
 pair=[]
 for card in [0,1]:
  p=r/f'r3-{label}-card{card}.log';text=p.read_text();all_events=list(events(p))
  skill=[e for e in all_events if 'skill_index: Some(7)' in e['profile'] and 'market_proc: false' in e['profile'] and 'card_proc: false' in e['profile']]
  assert skill and all('arrow: true' in e['profile'] for e in skill)
  noncrit=[e for e in skill if float(e['crit_multiplier'])==1.]
  assert noncrit
  scale=[float(e['scaled_base_damage'])/float(e['base_damage']) for e in skill]
  assert max(scale)-min(scale)<1e-5
  observation=re.search(r'ARROW_CARD_OBSERVATION frame=295 card=(\w+) attacks=(\d+) base_attack=(\S+) learned_rank=(\d+) caster_every=4',text)
  assert observation and observation[1]==str(bool(card)).lower() and observation[4]=='1'
  pair.append({'skill7_contacts':len(skill),'skill7_noncrit_contacts':len(noncrit),'skill7_scale_min':min(scale),'skill7_scale_max':max(scale),'skill7_distinct_raw_bases':sorted(set(e['base_damage'] for e in skill)),'actual_attacks_frame295':int(observation[2]),'actual_build_attack_frame295':observation[3],'actual_noncrit_samples':noncrit[:3]})
 assert pair[0]['actual_attacks_frame295']==pair[1]['actual_attacks_frame295']
 assert pair[0]['actual_build_attack_frame295']==pair[1]['actual_build_attack_frame295']
 assert pair[0]['skill7_contacts']==pair[1]['skill7_contacts']
 assert pair[0]['skill7_distinct_raw_bases']==pair[1]['skill7_distinct_raw_bases']
 assert abs(pair[1]['skill7_scale_min']/pair[0]['skill7_scale_min']-1.15)<1e-5
 result[label]=pair
result['scope']='skill7 contacts isolated by actual profile. Native/support Attack profiles coincide after classification, so this native trace does not distinguish those carriers. Unit actual exhausted-quiver native fixture supplies that separate evidence. RNG seeds equal, individual critical assignments may differ due to support arrows; noncritical and precritical scalars compared.'
(r/'native-comparison.json').write_text(json.dumps(result,indent=2))
print('PASS actual paired skill7 counts/base values/commit counts and1.15 precritical multiplier')
