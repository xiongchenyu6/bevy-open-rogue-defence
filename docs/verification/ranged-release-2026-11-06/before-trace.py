import pathlib,subprocess,os,hashlib,json
r=pathlib.Path.cwd();p=r/'target/verification/ranged-release-2026-11-06';old=r/'target/verification/attack-phase-closeup-2026-11-06';exe=old/'capture';inputs=json.loads((old/'inputs.json').read_text());assert hashlib.sha256(exe.read_bytes()).hexdigest()==(old/'binary.sha256').read_text()
for kind in ['ember','storm']:
 name='before-trace-'+kind;env=os.environ.copy();env.update(ROGUE_CAPTURE_ATTACK_TRACE='1',ROGUE_CAPTURE_RELEASE_TRACE='1',RUST_LOG='info,rogue_commander::projectile_hit=trace',ROGUE_CAPTURE_SAVE_FRAMES='10,13,20,23' if kind=='ember' else '15,19,30,34',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')))
 cmd=['nix','develop','--command',str(exe),'frames',str(p/name),'210','attack-'+kind,'0xA11CE','1280x720']
 with(p/(name+'.log')).open('w') as log:a=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':a.returncode,'binary_sha256':hashlib.sha256(exe.read_bytes()).hexdigest()}));assert a.returncode==0
 text=(p/(name+'.log')).read_text();assert 'PROJECTILE_HIT' in text and 'wrote 210 simulation frames' in text
 assert all(hashlib.sha256((r/f).read_bytes()).hexdigest()==h for f,h in inputs.items() if f.startswith('assets/'))
 print(name,'actual production baseline trace terminalPASS',flush=True)
print('BEFORE TRACE TERMINAL PASS',flush=True)
