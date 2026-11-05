import pathlib,subprocess,os,json,itertools,re,hashlib,struct
root=pathlib.Path.cwd();out=root/'target/verification/designated-strike-replay-2026-11-06'
binary=root/'target/verification/designated-strike-2026-11-06/balance-sim-snapshot'
print('WAIT after-fix verification gate',flush=True)
while True:
 p=subprocess.run(['systemctl','--user','show','rogue-designated-strike-after-nov6.service','-p','ActiveState','-p','Result'],capture_output=True,text=True,check=True)
 state=dict(line.split('=',1) for line in p.stdout.splitlines() if '=' in line)
 if state.get('ActiveState') not in ('active','activating','deactivating'):
  assert state.get('Result')=='success',state
  assert 'TERMINAL PASS' in (binary.parent/'after-sequence.log').read_text(),'verification gate did not complete'
  break
 import time
 time.sleep(5)
expected=json.loads((binary.parent/'binary.json').read_text())['sha256']
assert hashlib.sha256(binary.read_bytes()).hexdigest()==expected
(out/'scope.json').write_text(json.dumps(dict(scope='Normal full-prefix duplicate replay diagnostics; no automatic balance matrix or tuning',binary=str(binary),binary_sha256=expected,frame_cap=120000,actor_range=[1,120000],matrix_started=False),indent=2)+'\n')
os.environ['ROGUE_SIM_TRACE_RANGE']='1:120000';os.environ['RUST_LOG']='rogue_commander::projectile_hit=trace'
common=['--frames','120000','--jobs','1','--case-timeout','900','--chapters','5','--training','0','--seeds','A11CE','--policies','active','--starters','2','--positions','behind','--require-telemetry']
terminals=[]
for c in 'ab':
 print('START full replay',c,flush=True)
 with (out/f'probe-{c}.log').open('w') as log:p=subprocess.run(['python3','tools/run_balance_matrix.py','--binary',str(binary),'--batch',f'designated-fix-repeat-{c}-nov6']+common,stdout=log,stderr=subprocess.STDOUT)
 assert p.returncode==0,(c,p.returncode)
 receipt=json.loads((root/f'target/verification/designated-fix-repeat-{c}-nov6/summary.json').read_text())['cases'][0]
 assert receipt['valid'],receipt
 terminals.append(receipt)
 print('PASS normal replay',c,receipt['outcome'],flush=True)
paths=[root/f'target/verification/designated-fix-repeat-{c}-nov6/active-h2-behind-sa11ce-c5-t0.jsonl' for c in 'ab']
p=subprocess.run(['python3','tools/compare_balance_replays.py',*map(str,paths)],capture_output=True,text=True)
(out/'checkpoint-comparison.json').write_text(p.stdout)
def traces(p):
 for line in p.open():
  if line.startswith('{') and (r:=json.loads(line)).get('type')=='trace':yield r
report=dict(scope='all recorded scalar + actor numeric multiset fields; strips only entity identity/order; unrecorded state remains outside scope',first_strict=None,first_numeric=None,first_raw_order=None,trace_counts=[0,0],frame_cap=120000,outcomes=[dict(outcome=r['outcome'],censored=r['censored'],last=r['last'],runtime_secs=r['runtime_secs']) for r in terminals])
prev=None;context={}
for index,(a,b) in enumerate(itertools.zip_longest(*(traces(p) for p in paths))):
 for i,r in enumerate([a,b]):report['trace_counts'][i]+=r is not None
 if a is None or b is None:report.setdefault('missing_row',index);continue
 d={k:[a.get(k),b.get(k)] for k in sorted(set(a)|set(b)) if a.get(k)!=b.get(k)}
 if d and report['first_strict'] is None:report['first_strict']={'frame':a['frame'],'fields':d}
 if a['enemy_order']!=b['enemy_order'] and report['first_raw_order'] is None:report['first_raw_order']={'frame':a['frame'],'a':a['enemy_order'],'b':b['enemy_order']}
 d={k:[a[k],b[k]] for k in a if k not in ['enemy_order','enemies','projectiles','hero_positions'] and a[k]!=b[k]}
 for k in ['enemies','projectiles','hero_positions']:
  x=sorted(row[1:] for row in a[k]);y=sorted(row[1:] for row in b[k])
  if x!=y:d[k+'_numeric_multiset']=[x,y]
 if d and report['first_numeric'] is None:
  report['first_numeric']={'frame':a['frame'],'elapsed_bits_a':a['elapsed_bits'],'elapsed_bits_b':b['elapsed_bits'],'fields':d}
  context={'previous':prev,'first':{'a':a,'b':b}}
 prev={'a':a,'b':b}
assert report['trace_counts']==[r['last']['frame'] for r in terminals],report['trace_counts']
(out/'actor-comparison.json').write_text(json.dumps(report,indent=2)+'\n');(out/'first-difference-context.json').write_text(json.dumps(context,indent=2)+'\n')
ansi=re.compile(r'\x1b\[[0-9;]*m');counts=[]
for c,path in zip('ab',paths):
 events=[];count=0
 bits=report['first_numeric'] and report['first_numeric']['elapsed_bits_'+c]
 for line in path.with_suffix('.stderr').open():
  if 'PROJECTILE_HIT' not in line:continue
  count+=1;clean=ansi.sub('',line.rstrip())
  match=re.search(r'\belapsed_bits=(\d+)',clean)
  if bits is not None and match and int(match.group(1))==bits:
   event={'raw':clean}
   for k,v in re.findall(r'\b([a-z_]+)=([-+0-9.eE]+)',clean):event[k]=float(v) if any(t in v for t in '.eE') else int(v)
   event['profile']=re.search(r'\bprofile=(.*?)\s+base_damage=',clean).group(1);events.append(event)
 assert count>0,'real contact trace required';counts.append(count)
 if context and context['previous']:
  before={r[0]:r for r in context['previous'][c]['enemies']};after={r[0]:r for r in context['first'][c]['enemies']}
  for e in events:e['target_previous_enemy_row']=before.get(e['enemy_bits']);e['target_after_enemy_row']=after.get(e['enemy_bits'])
 (out/f'first-difference-contacts-{c}.json').write_text(json.dumps(events,indent=2)+'\n')
(out/'contact-counts.json').write_text(json.dumps(dict(counts=counts,scope='all raw contacts retained in original stderr; only first-difference extracts derived'),indent=2)+'\n')
print('TERMINAL NORMAL FULL-PREFIX REPLAYS',json.dumps(dict(first_strict=report['first_strict'] and report['first_strict']['frame'],first_numeric=report['first_numeric'] and report['first_numeric']['frame'],contact_counts=counts,actor_trace_counts=report['trace_counts'])),flush=True)
