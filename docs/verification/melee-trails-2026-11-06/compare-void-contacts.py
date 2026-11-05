import re,pathlib,json,struct,itertools
p=pathlib.Path(__file__).resolve().parent
runs=[];raw=[]
for stage in ['before','after']:
 events=[]
 for line in (p/f'{stage}-void-probe.log').read_text().splitlines():
  if 'PROJECTILE_HIT' not in line:continue
  line=re.sub(r'\x1b\[[0-9;]*m','',line)
  e={k:v for k,v in re.findall(r'\b([a-z_]+)=([-+0-9.eE]+)',line)}
  e['profile']=re.search(r'profile=(.*?) base_damage=',line)[1];events.append(e)
 raw.append(events)
 runs.append([{k:v for k,v in e.items() if k not in ('outer_ordinal','projectile','projectile_bits','enemy','enemy_bits')} for e in events])
assert runs[0]==runs[1] and len(runs[0])==77
# At the observed frame67 contact time, reconstruct real HP from actual resolver
# writes. Enemy IDs differ between builds; numeric HP multisets are compared.
states=[]
for events in raw:
 times=list(dict.fromkeys(e['elapsed_bits'] for e in events));cutoff='1074720488';assert cutoff in times;allowed=times[:times.index(cutoff)+1];state={}
 for e in events:
  if e['elapsed_bits'] not in allowed:break
  state[e['enemy_bits']]=float(e['hp_after'])
 states.append(sorted(state.values()))
assert states[0]==states[1] and len(states[0])==3
f32=lambda v:struct.unpack('<f',struct.pack('<f',v))[0]
sums=sorted({f32(f32(f32(0.+v[0])+v[1])+v[2]) for v in itertools.permutations(states[0])})
r=dict(scope='All actual resolver numeric/profile fields match for77contacts; IDs/outerordinal differ. Reconstructed frame67 actual HP multiset equal; f32 query sum permits both observed totals.',contacts=77,actual_contact_fields_equal=True,frame67_hp_multisets=states,possible_f32_query_sums=sums)
(p/'void-contact-comparison.json').write_text(json.dumps(r,indent=2)+'\n')
print(json.dumps(r))
