import pathlib,subprocess,hashlib,json,os
r=pathlib.Path.cwd();p=r/'target/verification/crown-detail-2026-11-06';a=json.loads((p/'after-inputs.json').read_text())
for stage in ['before','after']:
 exe=p/(stage+'-capture');assert hashlib.sha256(exe.read_bytes()).hexdigest()==(p/(stage+'-binary.sha256')).read_text()
 for lang in ['en','zh']:
  name=stage+'-portrait-'+lang;env=os.environ.copy();env.update(ROGUE_COMMANDER_LANG=lang,EXPECT_CROWN_DETAIL_FIXED='0' if stage=='before' else '1',ROGUE_CAPTURE_SAVE_FRAMES='99,179',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')))
  cmd=['nix','develop','--command',str(exe),'frames',str(p/name),'240','crown-detail','0xA11CE','720x1280']
  with (p/(name+'-native.log')).open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
  (p/(name+'-native-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode}))
  assert x.returncode==0
  text=(p/(name+'-native.log')).read_text();assert 'PASS crown-detail:' in text and 'wrote 240 simulation frames' in text and 'all 2 screenshot(s) saved' in text
  assert all(hashlib.sha256((r/f).read_bytes()).hexdigest()==h for f,h in a.items())
  print(name,'actual native240 terminal PASS',flush=True)
print('PORTRAIT CHAIN TERMINAL PASS',flush=True)
