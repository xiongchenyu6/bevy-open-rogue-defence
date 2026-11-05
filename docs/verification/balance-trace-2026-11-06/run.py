#!/usr/bin/env python3
"""Build after the prior Cargo owner finishes, then two gated replays only."""
import hashlib,json,pathlib,subprocess,time
ROOT=pathlib.Path.cwd()
OUT=ROOT/'target/verification/balance-trace-2026-11-05'
PRIOR='rogue-inventory-split-r2-build-nov5.service'
PRIORLOG=ROOT/'target/verification/inventory-split-r2-2026-11-05/sequence.log'
def sha(data):return hashlib.sha256(data).hexdigest()
def receipt():
 files=[ROOT/'Cargo.toml',ROOT/'Cargo.lock']
 files+=list((ROOT/'src').rglob('*.rs'))+list((ROOT/'vendor/bevy_audio').rglob('*.rs'))
 diff=subprocess.check_output(['git','diff','--no-ext-diff','--binary','HEAD'])
 return dict(head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
   status=subprocess.check_output(['git','status','--porcelain'],text=True),diff_sha256=sha(diff),
   source_sha256={str(p.relative_to(ROOT)):sha(p.read_bytes()) for p in sorted(files)})
def save(name,data):(OUT/name).write_text(json.dumps(data,indent=2)+'\n')
def run(name,args):
 print('START',name,flush=True)
 with (OUT/name).open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
 assert p.returncode==0,(name,p.returncode)
 print('PASS',name,flush=True)
print('START diagnostic verification; single Cargo lane',flush=True)
before=receipt();save('source-before-build.json',before)
run('bin-tests.log',['nix','develop','--command','cargo','test','--locked','--release','--bin','balance_sim','-j2'])
run('clippy.log',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings'])
run('build.log',['nix','develop','--command','cargo','build','--locked','--release','--bin','balance_sim','-j2'])
after=receipt();save('source-after-build.json',after)
assert before['source_sha256']==after['source_sha256'],'source content changed during build; stop before replay'
snapshot=OUT/'balance-sim-snapshot'
import shutil
shutil.copy2(ROOT/'target/release/balance_sim',snapshot)
save('binary.json',dict(path=str(snapshot),sha256=sha(snapshot.read_bytes()),source_head=after['head'],source_diff_sha256=after['diff_sha256']))
common=['--frames','120000','--jobs','1','--case-timeout','900','--chapters','5','--training','0','--seeds','A11CE','--policies','active','--starters','2','--positions','behind','--require-telemetry']
run('diagnostic-commit.log',['git','commit','--only','-m','Add opt-in exact-state balance replay diagnostics','src/bin/balance_sim.rs'])
save('diagnostic-commit.json',dict(head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()))
import os
os.environ['ROGUE_SIM_TRACE_RANGE']='19000:32400'
for label in ('a','b'):
 run(f'replay-{label}.log',['python3','tools/run_balance_matrix.py','--binary',str(snapshot),'--batch',f'trace-repeat-{label}-nov5']+common)
a=ROOT/'target/verification/trace-repeat-a-nov5/active-h2-behind-sa11ce-c5-t0.jsonl'
b=ROOT/'target/verification/trace-repeat-b-nov5/active-h2-behind-sa11ce-c5-t0.jsonl'
p=subprocess.run(['python3','tools/compare_balance_replays.py',str(a),str(b)],capture_output=True,text=True)
(OUT/'comparison.json').write_text(p.stdout)
(OUT/'comparison.stderr').write_text(p.stderr)
def rows(path):return [r for line in path.read_text().splitlines() if line.startswith('{') and (r:=json.loads(line)).get('type')=='checkpoint']
x,y=rows(a),rows(b)
complete=bool(x and y and x[-1]['finished'] and y[-1]['finished'])
report=dict(complete=complete,all_checkpoint_fields_equal=x==y,left_count=len(x),right_count=len(y),strict_comparator_exit=p.returncode,matrix_started=False,scope='recorded checkpoints only; not proof of all internal state determinism')
for i,(u,v) in enumerate(zip(x,y)):
 if u!=v:
  report['first_difference']=dict(checkpoint=i,frame_left=u.get('frame'),frame_right=v.get('frame'),fields={k:[u.get(k),v.get(k)] for k in sorted(set(u)|set(v)) if u.get(k)!=v.get(k)})
  break
save('full-checkpoint-comparison.json',report)
print(json.dumps(report),flush=True)
print('TERMINAL: duplicate gate only; 24-case matrix remains withheld pending root review',flush=True)


def traces(path):
 for line in path.open():
  if line.startswith('{'):
   record=json.loads(line)
   if record.get('type')=='trace':yield record
import itertools
trace_report=dict(scope='exact diagnostic fields only, not full internal state or causal proof',range=[19000,32400],left_count=0,right_count=0,first_raw_order_difference=None,first_canonical_difference=None,first_any_difference=None)
for index,(left,right) in enumerate(itertools.zip_longest(traces(a),traces(b))):
 if left is not None:trace_report['left_count']+=1
 if right is not None:trace_report['right_count']+=1
 if left is None or right is None:
  if trace_report.get('first_missing_row') is None:trace_report['first_missing_row']=dict(index=index,left_frame=left and left.get('frame'),right_frame=right and right.get('frame'))
  continue
 fields={k:[left.get(k),right.get(k)] for k in sorted(set(left)|set(right)) if left.get(k)!=right.get(k)}
 if fields and trace_report['first_any_difference'] is None:trace_report['first_any_difference']=dict(index=index,frame_left=left['frame'],frame_right=right['frame'],fields=fields)
 if left['enemy_order']!=right['enemy_order'] and trace_report['first_raw_order_difference'] is None:trace_report['first_raw_order_difference']=dict(index=index,frame_left=left['frame'],frame_right=right['frame'],left=left['enemy_order'],right=right['enemy_order'])
 canonical={k:v for k,v in fields.items() if k!='enemy_order'}
 if canonical and trace_report['first_canonical_difference'] is None:trace_report['first_canonical_difference']=dict(index=index,frame_left=left['frame'],frame_right=right['frame'],fields=canonical)
trace_report['expected_count']=32400-19000+1
trace_report['range_complete']=trace_report['left_count']==trace_report['right_count']==trace_report['expected_count']
save('trace-comparison.json',trace_report)
print('TRACE_TERMINAL',json.dumps({k:v for k,v in trace_report.items() if not k.startswith('first_')}),flush=True)
