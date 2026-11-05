import pathlib,subprocess,os
root=pathlib.Path.cwd();out=root/'target/verification/dagger-rhythm-2026-11-06';exe=out/'before-capture'
for kind in ['rift','tide']:
 state=out/('state-contact-'+kind);state.mkdir(exist_ok=True)
 env=os.environ.copy();env.update(ROGUE_CAPTURE_ATTACK_TRACE='1',ROGUE_CAPTURE_SAVE_FRAMES=','.join(map(str,range(10,21))),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'))
 with (out/('before-contact-'+kind+'.log')).open('w') as log:r=subprocess.run(['nix','develop','--command',str(exe),'frames',str(out/('before-contact-'+kind)),'210','attack-'+kind,'0xA11CE','1280x720'],env=env,stdout=log,stderr=subprocess.STDOUT)
 assert r.returncode==0
 print('PASS exact contact shots',kind,flush=True)
