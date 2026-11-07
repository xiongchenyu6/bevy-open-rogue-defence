from pathlib import Path
import subprocess,json,os
r=Path.cwd();p=r/'target/verification/phone-draft-readability-2026-11-07/r7';out=p/'build-controls-before';out.mkdir(exist_ok=False)
for scenario in ['build-skill','build-bond']:
 env=os.environ.copy();env.update(ROGUE_CAPTURE_SAVE_FRAMES='49',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(out/(scenario+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(out/(scenario+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(out/(scenario+'-audio.save')))
 cmd=['nix','develop','--command',str(p/'capture'),'frames',str(out/scenario),'180',scenario,'0xC0126','1280x720']
 with(out/(scenario+'.log')).open('w') as log:x=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT)
 (out/(scenario+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode}));print(scenario,x.returncode,flush=True)
