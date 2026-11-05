import pathlib,re,json,ast,hashlib
p=pathlib.Path(__file__).resolve().parent
report={}
for kind in ['rift','tide','ember']:
 stages={}
 for stage in ['before','after']:
  text=(p/f'{stage}-{kind}.log').read_text()
  assert 'PASS: real basic attack damage' in text and 'panicked' not in text
  rows=[]
  for line in text.splitlines():
   m=re.search(r'ATTACK_TIMING frame=(\d+) health=([\d.]+) damage=([\d.]+) clips=(.*)',line)
   if m:rows.append(dict(frame=int(m[1]),health=float(m[2]),damage=float(m[3]),clips=ast.literal_eval(m[4])))
  hits=[r for r in rows if r['damage']>0]
  stages[stage]=dict(rows=rows,hits=hits,cadence=[b['frame']-a['frame'] for a,b in zip(hits,hits[1:])])
 a,b=stages['before'],stages['after']
 if kind=='ember':assert a==b,'Ranged control trace changed'
 else:
  paired=min(len(a['hits']),len(b['hits']))
  assert paired>=5
  assert [r['damage'] for r in a['hits'][:paired]]==[r['damage'] for r in b['hits'][:paired]],'Matched real HP-hit deltas changed'
  offsets=[y['frame']-x['frame'] for x,y in zip(a['hits'],b['hits'])];assert set(offsets)=={1},offsets
  assert a['cadence'][:paired-1]==b['cadence'][:paired-1]
 report[kind]=dict(before_hits=a['hits'],after_hits=b['hits'],before_cadence=a['cadence'],after_cadence=b['cadence'],matched_damage_equal=True,ranged_trace_identical=(a==b) if kind=='ember' else None)
(p/'comparison.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS paired real HP-hit deltas/cadence; each dagger hit +1frame; complete Ember trace identical')
