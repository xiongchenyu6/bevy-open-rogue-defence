from pathlib import Path
import subprocess,json,hashlib,os,time
r=Path.cwd();p=r/'target/verification/phone-draft-readability-2026-11-07/r7';out=p/'supplement';out.mkdir(exist_ok=False)
a=json.loads((p/'inputs.json').read_text())
def check():
 for name,value in a.items():assert hashlib.sha256((r/name).read_bytes()).hexdigest()==value,name
check()
state=subprocess.check_output(['systemctl','--user','show','rogue-phone-draft-readability-r7-nov7.service','-p','ActiveState','-p','Result','-p','ExecMainStatus'],text=True)
assert 'ActiveState=inactive' in state and 'Result=success' in state and 'ExecMainStatus=0' in state
assert 'PHONE DRAFT READABILITY CHAIN PASS' in (p/'manager.log').read_text()
exe=p/'capture';assert hashlib.sha256(exe.read_bytes()).hexdigest()==(p/'binary.sha256').read_text()
(out/'source-inputs.json').write_text(json.dumps(a,indent=2));(out/'binary.sha256').write_text((p/'binary.sha256').read_text())
for name,size,scenario,frames in [('desktop','1280x720','crown-detail',240),('compact','844x390','crown-detail',240),('continuous-desktop','1280x720','immediate-skill',320)]:
 env=os.environ.copy();env.update(EXPECT_CROWN_DETAIL_FIXED='1',ROGUE_CAPTURE_DRAFT_LAYOUT='1',ROGUE_COMMANDER_LANG='en',ROGUE_CAPTURE_SAVE_FRAMES='100,180,239',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(out/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(out/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(out/(name+'-audio.save')))
 cmd=['nix','develop','--command',str(exe),'frames',str(out/name),str(frames),scenario,'0xC0126',size];start=time.monotonic()
 with(out/(name+'.log')).open('w') as log:x=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT)
 (out/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode,'elapsed':time.monotonic()-start}));check();assert x.returncode==0,name
 assert f'wrote {frames} simulation frames' in (out/(name+'.log')).read_text()
 print(name,'TERMINAL PASS',flush=True)
(out/'inputs-after.json').write_text(json.dumps(a,indent=2));print('PHONE DRAFT SUPPLEMENT PASS',flush=True)
