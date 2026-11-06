import pathlib,subprocess,sys,json,time,hashlib
r=pathlib.Path.cwd();o=pathlib.Path(__file__).resolve().parent
results=[]
def run(name,args,allow_difference=False):
 print('STAGE',name,'begin',flush=True);t=time.monotonic()
 with (o/(name+'.log')).open('w') as log:q=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
 row={'stage':name,'args':args,'exit':q.returncode,'seconds':time.monotonic()-t};results.append(row);(o/'manager-processes.json').write_text(json.dumps(results,indent=2)+'\n');print('STAGE',name,'terminal',q.returncode,flush=True)
 assert q.returncode==0 or (allow_difference and q.returncode==1),row
run('build',[sys.executable,str(o/'build.py')])
run('pair',[sys.executable,str(o/'run-full.py')])
f=o/'frozen';paths=[str(f/'target/verification'/('post-order-full-'+s+'-nov6')/'active-h2-behind-sbeef-c5-t0.jsonl') for s in 'ab']
run('checkpoint-comparison',[sys.executable,str(f/'tools/compare_balance_replays.py'),*paths],True)
run('recorded-comparison',[sys.executable,str(o/'analysis/compare-recorded.py'),*paths,str(o/'full-recorded')])
print('PAIR_AND_ANALYSIS_FINISHED; inspect reports before equality claims',flush=True)
