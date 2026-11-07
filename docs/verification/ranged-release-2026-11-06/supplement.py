import pathlib,subprocess,hashlib,json,os
r=pathlib.Path.cwd();p=r/'target/verification/ranged-release-2026-11-06';a=json.loads((p/'after-inputs.json').read_text());old=r/'target/verification/attack-phase-closeup-2026-11-06/capture';new=p/'capture'
assert hashlib.sha256(new.read_bytes()).hexdigest()==(p/'binary.sha256').read_text()
cases=[('after-trace-'+k,new,'attack-'+k,'1280x720','10,13,20,23' if k=='ember' else '15,19,30,34') for k in ['ember','storm']]
for stage,binary in [('before',old),('after',new)]:
 for kind,scenario in [('ember','weapon-growth-archer-starter'),('storm','weapon-growth-mage-starter')]:
  cases.append((stage+'-portrait-'+kind,binary,scenario,'720x1280',','.join(map(str,[*range(9,24),*range(29,36)]))))
for name,binary,scenario,size,frames in cases:
 count=250 if 'portrait' in name else 210
 env=os.environ.copy();env.update(ROGUE_CAPTURE_ATTACK_TRACE='1',ROGUE_CAPTURE_RELEASE_TRACE='1',RUST_LOG='info,rogue_commander::projectile_hit=trace',ROGUE_CAPTURE_SAVE_FRAMES=frames,VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')))
 cmd=['nix','develop','--command',str(binary),'frames',str(p/name),str(count),scenario,'0xA11CE',size]
 with(p/(name+'.log')).open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest()}));assert x.returncode==0
 text=(p/(name+'.log')).read_text();assert 'PROJECTILE_HIT' in text and f'wrote {count} simulation frames' in text and 'PASS: real basic attack damage' in text
 if 'portrait' in name:assert 'PASS weapon-growth-' in text
 assert all(hashlib.sha256((r/f).read_bytes()).hexdigest()==h for f,h in a.items())
 print(name,'actual native terminalPASS',flush=True)
print('SUPPLEMENT TERMINAL PASS',flush=True)
