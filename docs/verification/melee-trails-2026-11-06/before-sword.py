import pathlib,subprocess,os
root=pathlib.Path.cwd();out=root/'target/verification/melee-trails-2026-11-06';state=out/'state-before-sword';state.mkdir(exist_ok=True)
env=os.environ.copy();env.update(ROGUE_CAPTURE_ATTACK_TRACE='1',ROGUE_CAPTURE_SAVE_FRAMES='14,15,16,17,18,19,20',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'))
with (out/'before-sword.log').open('w') as log:r=subprocess.run(['nix','develop','--command',str(root/'target/verification/dagger-rhythm-2026-11-06/after-capture'),'frames',str(out/'before-sword'),'210','attack-sword','0xA11CE','1280x720'],env=env,stdout=log,stderr=subprocess.STDOUT)
assert r.returncode==0
print('PASS before-sword current frozen d454 artifact',flush=True)
