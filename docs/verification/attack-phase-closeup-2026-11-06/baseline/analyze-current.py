from pathlib import Path
import re,json,ast,hashlib
r=Path(__file__).parent;o={}
for kind,duration in [('ember',1.0666666666666667),('storm',.9333333333333333),('sword',1.0666666666666667)]:
 p=r/f'current-{kind}.log';lines=p.read_text().splitlines();events=[];phases=[]
 for l in lines:
  if l.startswith('PROJECTILE_RELEASE '):
   frame=int(re.search(r'frame=(\d+)',l)[1]);clips=ast.literal_eval(l.split('clips=')[1]);events.append({'frame':frame,'line':l,'clips':clips,'attack_seek_fraction':[x[1]/duration for x in clips if x[0]==3]})
  if l.startswith('RELEASE_PHASE '):
   phases.append({'frame':int(re.search(r'frame=(\d+)',l)[1]),'clips':ast.literal_eval(l.split('clips=')[1])})
 starts=[p for i,p in enumerate(phases) if p['clips'] and p['clips'][0][0]==3 and (i==0 or phases[i-1]['clips'][0][0]!=3 or p['clips'][0][1]<phases[i-1]['clips'][0][1])]
 o[kind]={'log_sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'phase_observations':len(phases),'projectile_first_observations':len(events),'first_two_release_observations':events[:2],'all_release_frames':[e['frame'] for e in events],'first_two_attack_starts':starts[:2],'canonical_attack_duration_seconds':duration,'first_two_cycle_phase_rows':phases[:starts[2]['frame']-1] if len(starts)>2 else phases,'scope':'Last first observation after travel, not exact spawn/hand release; no short-lived same-update carriers'}
(r/'measured-summary.json').write_text(json.dumps(o,indent=2));print({k:{'phase_rows':v['phase_observations'],'births':v['projectile_first_observations'],'starts':v['first_two_attack_starts']}for k,v in o.items()})
