#!/usr/bin/env python3
import hashlib,itertools,json,os,subprocess,time
from pathlib import Path
root=Path.cwd();out=root/'target/verification/late-chapter-balance-oct14';out.mkdir(exist_ok=True)
source=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
subprocess.run(['nix','develop','--command','cargo','build','--locked','--bin','balance_sim','-j2'],check=True)
binary=root/'target/debug/balance_sim'
(out/'scope.json').write_text(json.dumps({'source':source,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'chapters':[5,6],'training':0,'seeds':['A11CE','BEEF'],'heroes':[0,1,2],'policies':['active','passive'],'frame_cap':120000,'scope':'Exploratory untrained late chapters; production systems, no renderer. Not earned chapter access or packaged victory; late determinism not established.'},indent=2))
results=[]
for chapter,hero,seed,policy in itertools.product([4,5],range(3),['A11CE','BEEF'],['active','passive']):
 name=f'{policy}-h{hero}-s{seed.lower()}-c{chapter}-t0';folder=out/name;folder.mkdir()
 env=os.environ.copy();env.update(ROGUE_SIM_CHAPTER=str(chapter),ROGUE_SIM_TRAINING='0',ROGUE_CAMPAIGN_SAVE=str(folder/'campaign.save'),ROGUE_AUDIO_SAVE=str(folder/'audio.save'),XDG_CONFIG_HOME=str(folder/'config'))
 (out/'progress.json').write_text(json.dumps({'completed':len(results),'total':24,'current':name,'stage':'running'}))
 start=time.monotonic()
 with (out/(name+'.jsonl')).open('w') as stdout,(out/(name+'.stderr')).open('w') as stderr:
  try:
   p=subprocess.run([str(binary),policy,seed,str(hero),'behind','120000'],env=env,stdout=stdout,stderr=stderr,timeout=1800)
   result={'case':name,'exit':p.returncode,'seconds':time.monotonic()-start}
  except subprocess.TimeoutExpired:
   result={'case':name,'timeout':True,'seconds':time.monotonic()-start}
 results.append(result);(out/'receipts.json').write_text(json.dumps(results,indent=2));print(json.dumps(result),flush=True)
(out/'progress.json').write_text(json.dumps({'completed':len(results),'total':24,'stage':'terminal'}))
subprocess.run(['python3','tools/analyze_balance.py',str(out),'--output',str(out/'analysis')],check=True)
