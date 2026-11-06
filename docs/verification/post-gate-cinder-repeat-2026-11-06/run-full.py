import pathlib,json,hashlib,subprocess,os,time
r=pathlib.Path.cwd();o=pathlib.Path(__file__).resolve().parent;f=o/'frozen'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
source=json.loads((o/'inputs-after.json').read_text());assert source==json.loads((o/'inputs-before.json').read_text())
for n,h in source.items():assert sha(r/n)==h,n
pinned=json.loads((o/'frozen-files.json').read_text());binary=json.loads((o/'binary.json').read_text());assert sha(f/'balance-sim-snapshot')==binary['sha256']
for n,h in pinned.items():assert sha(f/n)==h,n
results=[]
for suffix in ('a','b'):
 name='post-order-full-'+suffix+'-nov6';env={k:v for k,v in os.environ.items()if not k.startswith('ROGUE_')and k!='RUST_LOG'}
 env.update(ROGUE_SIM_ASSET_ROOT=str(f/'assets'),ROGUE_SIM_TRACE_RANGE='1:120000',RUST_LOG='rogue_commander::projectile_hit=trace')
 a=['python3',str(f/'tools/run_balance_matrix.py'),'--binary',str(f/'balance-sim-snapshot'),'--batch',name,'--frames','120000','--jobs','1','--case-timeout','900','--chapters','5','--training','0','--seeds','BEEF','--policies','active','--starters','2','--positions','behind','--require-telemetry']
 print('STAGE',name,'begin',flush=True);start=time.monotonic()
 with(o/(name+'.log')).open('w')as log:q=subprocess.run(a,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=1000)
 row={'batch':name,'exit':q.returncode,'seconds':time.monotonic()-start,'args':a};results.append(row);(o/'full-processes.json').write_text(json.dumps(results,indent=2)+'\n');assert q.returncode==0,row
 summary=json.loads((f/'target/verification'/name/'summary.json').read_text());assert len(summary['cases'])==1
 case=summary['cases'][0];assert case['valid']and not case['timed_out']and case['returncode']==0,case
 assert summary['binary_sha256']==binary['sha256']
 path=pathlib.Path(case['log']);log=path.read_text();assert 'BALANCE_ASSET_ROOT '+str(f/'assets')in log
 stderr=pathlib.Path(case['stderr']).read_text();assert 'PROJECTILE_HIT'in stderr and 'panicked'not in stderr and 'ERROR'not in stderr
 for n,h in pinned.items():assert sha(f/n)==h,n
 for n,h in source.items():assert sha(r/n)==h,n
 print('STAGE',name,'valid',case['outcome'],'actual_seed',case['last']['actual_run_seed'],'elapsed',case['last']['elapsed'],flush=True)
print('CURRENT_FULL_PAIR_EXECUTED; no equality claim until comparisons',flush=True)
