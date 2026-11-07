from pathlib import Path
import subprocess,json,hashlib,os,time
r=Path.cwd();p=r/'target/verification/phone-draft-readability-2026-11-07/r9';out=p/'final-build-controls';out.mkdir(exist_ok=False)
a=json.loads((p/'inputs.json').read_text())
def check():
 for name,value in a.items():assert hashlib.sha256((r/name).read_bytes()).hexdigest()==value,name
check();exe=p/'capture';digest=hashlib.sha256(exe.read_bytes()).hexdigest();assert digest==(p/'binary.sha256').read_text()
(out/'binary.sha256').write_text(digest);(out/'inputs-before.json').write_text(json.dumps(a,indent=2))
for scenario in ['build-skill','build-bond']:
 env=os.environ.copy();env.update(ROGUE_CAPTURE_SAVE_FRAMES='100,179',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(out/(scenario+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(out/(scenario+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(out/(scenario+'-audio.save')))
 cmd=['nix','develop','--command',str(exe),'frames',str(out/scenario),'180',scenario,'0xC0126','1280x720'];start=time.monotonic()
 with(out/(scenario+'.log')).open('w') as log:x=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT)
 (out/(scenario+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode,'elapsed':time.monotonic()-start}));check();assert x.returncode==0,scenario
 text=(out/(scenario+'.log')).read_text();assert 'wrote 180 simulation frames' in text and 'build preview paused; all cards/actions fit viewport' in text
 print(scenario,'TERMINAL PASS',flush=True)
(out/'inputs-after.json').write_text(json.dumps(a,indent=2));print('SAME R9 BINARY BUILD CONTROLS PASS',flush=True)
