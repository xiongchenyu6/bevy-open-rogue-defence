import pathlib,subprocess,os,json,hashlib
r=pathlib.Path.cwd();p=r/'target/verification/hit-flash-tier-2026-11-06';inputs=json.loads((p/'after-inputs.json').read_text())
for stage in ['baseline','after']:
 env=os.environ.copy();env.update(ROGUE_CAPTURE_SAVE_FRAMES='18,19,20,24,25,100,101,102,103,104,108,109,110,120,249',ROGUE_CAPTURE_HIT_MATERIAL_TRACE='1',EXPECT_HIT_FLASH='1',ROGUE_CAPTURE_ATTACK_TRACE='1',RUST_LOG='info,rogue_commander::projectile_hit=trace',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(stage+'-portrait-campaign.save')),ROGUE_AUDIO_SAVE=str(p/(stage+'-portrait-audio.save')),ROGUE_TUTORIAL_SAVE=str(p/(stage+'-portrait-tutorial.save')))
 cmd=['nix','develop','--command',str(p/(stage+'-capture')),'frames',str(p/(stage+'-portrait')),'250','weapon-growth-sword-starter','0xA11CE','720x1280']
 with (p/(stage+'-portrait-native.log')).open('w') as log:res=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 (p/(stage+'-portrait-result.json')).write_text(json.dumps({'command':cmd,'returncode':res.returncode}))
 assert res.returncode==0
 text=(p/(stage+'-portrait-native.log')).read_text()
 for expected in ['HIT_MATERIAL_PASS','PASS: real basic attack damage','PASS weapon-growth-sword-starter','wrote 250 simulation frames','all 15 screenshot(s) saved']:assert expected in text,(stage,expected)
 assert all(hashlib.sha256((r/f).read_bytes()).hexdigest()==h for f,h in inputs.items())
 print(stage,'portrait actual250 PASS',flush=True)
print('PORTRAIT TERMINAL PASS',flush=True)
