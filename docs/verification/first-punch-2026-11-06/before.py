import pathlib,subprocess,json,hashlib,shutil,os
root=pathlib.Path.cwd();p=root/'target/verification/first-punch-r2-2026-11-06'
def inputs():
 return {str(f.relative_to(root)):hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted([root/'Cargo.toml',root/'Cargo.lock',*root.glob('src/**/*.rs'),*root.glob('assets/**/*')]) if f.is_file()}
a=inputs();(p/'before-inputs.json').write_text(json.dumps(a,indent=2)+'\n')
for f in ['src/game/hit_feedback.rs','src/game/hit_feedback/enemy_flash.rs','src/bin/capture/hit_material.rs']:
 shutil.copy2(root/f,p/('before-'+pathlib.Path(f).name))
with (p/'before-tests.log').open('w') as log:r=subprocess.run(['nix','develop','--command','cargo','test','--locked','--release','--lib','large_real_frame_keeps_new_hit_punch_visible','-j2','--','--nocapture'],stdout=log,stderr=subprocess.STDOUT)
(p/'before-tests-exit.txt').write_text(str(r.returncode)+'\n');assert r.returncode==101 and '0 passed; 1 failed' in (p/'before-tests.log').read_text();assert inputs()==a
print('BEFORE actual negative test PASS',flush=True)
with (p/'before-build.log').open('w') as log:r=subprocess.run(['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'],stdout=log,stderr=subprocess.STDOUT)
assert r.returncode==0 and inputs()==a
exe=p/'before-capture';shutil.copy2(root/'target/market-visual/debug/capture',exe);(p/'before-binary.sha256').write_text(hashlib.sha256(exe.read_bytes()).hexdigest()+'\n')
state=p/'state-before';state.mkdir(exist_ok=True)
env=os.environ.copy();env.update(ROGUE_CAPTURE_HIT_MATERIAL_TRACE='1',ROGUE_CAPTURE_HIT_PUNCH_LOW_FPS='1',EXPECT_HIT_FLASH='1',EXPECT_HIT_PUNCH_FIRST_FRAME='0',ROGUE_CAPTURE_ATTACK_TRACE='1',ROGUE_CAPTURE_SAVE_FRAMES=','.join(map(str,range(0,40))),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'))
with (p/'before-native.log').open('w') as log:r=subprocess.run(['nix','develop','--command',str(exe),'frames',str(p/'before'),'210','attack-sword','0xA11CE','1280x720'],env=env,stdout=log,stderr=subprocess.STDOUT)
assert r.returncode==0 and inputs()==a
text=(p/'before-native.log').read_text();assert 'HIT_PUNCH_FIRST' in text and 'visible=false real_delta=0.16' in text and 'HIT_MATERIAL_PASS' in text and 'PASS: real basic attack damage' in text
print('TERMINAL BEFORE PASS: actual native contact, first squash absent',flush=True)
