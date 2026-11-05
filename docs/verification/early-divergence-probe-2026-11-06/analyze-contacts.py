import pathlib,re,json,struct,itertools
root=pathlib.Path.cwd();out=root/'target/verification/early-divergence-probe-2026-11-06'
ansi=re.compile(r'\x1b\[[0-9;]*m')
def events(c):
 p=root/f'target/verification/early-divergence-{c}-nov6/active-h2-behind-sa11ce-c5-t0.stderr'
 records=[]
 for line in p.open():
  if 'PROJECTILE_HIT' not in line:continue
  line=ansi.sub('',line.rstrip())
  r={k:(float(v) if any(x in v for x in '.eE') else int(v)) for k,v in re.findall(r'\b([a-z_]+)=([-+0-9.eE]+)',line)}
  r['profile']=re.search(r'\bprofile=(.*?)\s+base_damage=',line).group(1);r['raw']=line;records.append(r)
 return records
rows=[events(c) for c in 'ab'];result={'scope':'Prefix contact-sequence comparisons only; no actor mapping or causal conclusion','counts':list(map(len,rows))}
exclude={'raw','projectile','enemy','projectile_bits','enemy_bits','outer_ordinal','candidate_ordinal'}
for i,(a,b) in enumerate(zip(*rows)):
 fields={k:[a.get(k),b.get(k)] for k in a.keys()|b.keys() if k not in exclude and a.get(k)!=b.get(k)}
 if fields:
  result['first_contact_numeric_sequence_difference']={'index':i,'fields':fields,'a':a,'b':b};break
for i,(a,b) in enumerate(zip(*rows)):
 if {k:v for k,v in a.items() if k!='raw'}!={k:v for k,v in b.items() if k!='raw'}:
  result['first_strict_contact_difference']={'index':i,'a':a,'b':b};break
first=result.get('first_contact_numeric_sequence_difference')
if first:
 bits=first['a']['elapsed_bits'];result['same_elapsed_a']= [r for r in rows[0] if r['elapsed_bits']==bits];result['same_elapsed_b']=[r for r in rows[1] if r['elapsed_bits']==bits]
(out/'contact-prefix-analysis.json').write_text(json.dumps(result,indent=2)+'\n')
print('counts',result['counts'])
for k in ['first_strict_contact_difference','first_contact_numeric_sequence_difference']:
 r=result.get(k);print(k, r and {'index':r['index'],'elapsed_a':struct.unpack('f',struct.pack('I',r['a']['elapsed_bits']))[0],'elapsed_b':struct.unpack('f',struct.pack('I',r['b']['elapsed_bits']))[0],'fields':list(r.get('fields',{}))})
if first:
 for case in ['a','b']:
  print('GROUP',case)
  for r in result['same_elapsed_'+case]:print({k:r[k] for k in ['outer_ordinal','candidate_ordinal','projectile_bits','enemy_bits','profile','base_damage','crit_multiplier','hp_before','hp_after','critical_roll','rng_before_crit','rng_after_crit']})
