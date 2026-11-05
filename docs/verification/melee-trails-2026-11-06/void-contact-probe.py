import pathlib,subprocess,os
root=pathlib.Path.cwd();out=root/'target/verification/melee-trails-2026-11-06'
for stage,exe in [('before',root/'target/verification/dagger-rhythm-2026-11-06/after-capture'),('after',out/'after-capture')]:
 state=out/('state-contact-probe-'+stage);state.mkdir(exist_ok=True)
 env=os.environ.copy();env.update(RUST_LOG='rogue_commander::projectile_hit=trace',ROGUE_CAPTURE_ATTACK_TRACE='1',ROGUE_CAPTURE_SAVE_FRAMES='65,66,67,68',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'))
 with (out/(stage+'-void-probe.log')).open('w') as log:r=subprocess.run(['nix','develop','--command',str(exe),'frames',str(out/(stage+'-void-probe')),'210','attack-void','0xA11CE','1280x720'],env=env,stdout=log,stderr=subprocess.STDOUT)
 text=(out/(stage+'-void-probe.log')).read_text();assert r.returncode==0 and 'PROJECTILE_HIT' in text
 print('PASS real Void contact probe',stage,flush=True)
