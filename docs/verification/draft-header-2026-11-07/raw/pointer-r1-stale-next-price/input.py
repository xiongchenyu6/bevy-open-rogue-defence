import pathlib,subprocess,hashlib,json,os
r=pathlib.Path.cwd();p=r/'target/verification/draft-header-2026-11-07';exe=p/'capture'
m=json.loads((p/'inputs.json').read_text());assert hashlib.sha256(exe.read_bytes()).hexdigest()==(p/'binary.sha256').read_text()
for size in ['1280x720','360x800']:
 for scenario in ['immediate-skill','immediate-bond','immediate-skill-paid']:
  name='pointer-'+size+'-'+scenario;env=os.environ.copy();env.update(ROGUE_COMMANDER_LANG='en',ROGUE_CAPTURE_SAVE_FRAMES='30,100,174,289',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')))
  cmd=['nix','develop','--command',str(exe),'frames',str(p/name),'320',scenario,'0xC0126',size]
  with(p/(name+'.log')).open('w') as log:a=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
  (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':a.returncode}));assert a.returncode==0
  text=(p/(name+'.log')).read_text();assert 'wrote 320 simulation frames' in text and 'PASS '+scenario+':' in text
  assert all(hashlib.sha256((r/f).read_bytes()).hexdigest()==h for f,h in m.items())
  print(name,'TERMINAL PASS',flush=True)
print('DRAFT HEADER POINTER CHAIN TERMINAL PASS',flush=True)
name='warm-360x800-en';env=os.environ.copy();env.update(EXPECT_CROWN_DETAIL_FIXED='1',ROGUE_CAPTURE_DRAFT_LAYOUT='1',ROGUE_COMMANDER_LANG='en',ROGUE_CAPTURE_SAVE_FRAMES='100,180,239',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')))
cmd=['nix','develop','--command',str(exe),'frames',str(p/name),'240','crown-detail','0xC0126','360x800']
with(p/(name+'.log')).open('w') as log:a=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
(p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':a.returncode}));assert a.returncode==0
text=(p/(name+'.log')).read_text();assert 'wrote 240 simulation frames' in text and text.count('PASS draft-layout frame=')==2
assert all(hashlib.sha256((r/f).read_bytes()).hexdigest()==h for f,h in m.items())
print('DRAFT HEADER WARM REPEAT TERMINAL PASS',flush=True)
