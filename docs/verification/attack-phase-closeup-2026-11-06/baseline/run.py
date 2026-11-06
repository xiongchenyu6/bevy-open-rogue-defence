import pathlib,subprocess,hashlib,json,os
r=pathlib.Path.cwd();p=r/'target/verification/attack-phase-followup-2026-11-06';c=r/'target/verification/crown-detail-2026-11-06';a=json.loads((c/'after-inputs.json').read_text());exe=c/'after-capture'
assert hashlib.sha256(exe.read_bytes()).hexdigest()==(c/'after-binary.sha256').read_text()
(p/'inputs.json').write_text(json.dumps(a,indent=2))
for kind in ['ember','storm','sword']:
 stage='current-'+kind;env=os.environ.copy();env.update(ROGUE_CAPTURE_ATTACK_TRACE='1',ROGUE_CAPTURE_RELEASE_TRACE='1',ROGUE_CAPTURE_SAVE_FRAMES=','.join(map(str,range(0,61))),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(stage+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(stage+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(stage+'-audio.save')))
 cmd=['nix','develop','--command',str(exe),'frames',str(p/stage),'210','attack-'+kind,'0xA11CE','1280x720']
 with (p/(stage+'.log')).open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 (p/(stage+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode}))
 assert x.returncode==0
 text=(p/(stage+'.log')).read_text();assert 'wrote 210 simulation frames' in text and 'RELEASE_PHASE frame=210' in text and 'PASS: real basic attack damage' in text
 assert all(hashlib.sha256((r/f).read_bytes()).hexdigest()==h for f,h in a.items())
 print(stage,'actual native210 terminal PASS',flush=True)
print('PHASE CAPTURE TERMINAL PASS',flush=True)
