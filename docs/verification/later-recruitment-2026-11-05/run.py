import os,pathlib,subprocess
root=pathlib.Path.cwd();out=root/'target/verification/later-recruitment-r3-2026-11-05'
commands=[('clippy.log',['nix','develop','--command','cargo','clippy','--locked','--bin','capture','-j2','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0','--','-D','warnings']),('build.log',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j2','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])]
for name,args in commands:
 print('START',name,flush=True)
 with (out/name).open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
 assert p.returncode==0,(name,p.returncode)
 print('PASS',name,flush=True)
for route,lang,size in [('10','en','1280x720')]+[(r,l,s) for r in ('20','40','fruit') for l,s in [('zh-CN','1280x720'),('en','667x375')]]:
 name=f'{route}-{lang}-{size}';folder=out/name;folder.mkdir();state=folder/'state';state.mkdir()
 env=os.environ.copy()
 for key in ('CAMPAIGN','TUTORIAL','AUDIO','DETAIL','DISPLAY'):env['ROGUE_'+key+'_SAVE']=str(state/(key.lower()+'.save'))
 env.update(ROGUE_CAPTURE_RECRUIT_ROUTE=route,ROGUE_CAPTURE_SAVE_FRAMES='14,44,74,81,93,100,119,127,149,158,169',ROGUE_COMMANDER_LANG=lang,BEVY_ASSET_ROOT=str(root),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json')
 with (folder/'capture.log').open('w') as log:p=subprocess.run(['nix','develop','--command','target/market-visual/debug/capture','frames',str(folder),'180','asset-readiness-recruit','0xA11CE',size],env=env,stdout=log,stderr=subprocess.STDOUT,timeout=360)
 text=(folder/'capture.log').read_text()
 assert p.returncode==0 and 'panicked at' not in text and 'RECRUIT_READINESS_PASS' in text and 'wrote 180 simulation frames' in text,(name,p.returncode)
 print('PASS',name,flush=True)
