import pathlib,subprocess,os
root=pathlib.Path.cwd();out=root/'target/verification/inventory-split-r2-2026-11-05';assert 'PASS capture-build.log' in (out/'sequence.log').read_text()
cases=[('equipment-en-desktop','equipment','en','1280x720',310),('equipment-zh-compact','equipment','zh-CN','667x375',310),('equipment-en-portrait','equipment','en','390x844',310),('crafting-0-en-desktop','crafting-0','en','1280x720',310),('crafting-1-zh-desktop','crafting-1','zh-CN','1280x720',310),('crafting-1-en-compact','crafting-1','en','667x375',310),('crafting-0-zh-compact','crafting-0','zh-CN','667x375',310),('crafting-1-zh-portrait','crafting-1','zh-CN','390x844',310),('controller-desktop','controller-input','en','1280x720',2400),('controller-compact','controller-input','en','667x375',2400),('controller-scroll-compact','controller-scroll','en','667x375',1600)]
for name,scene,lang,size,count in cases:
 folder=out/name;folder.mkdir();state=folder/'state';state.mkdir();env=os.environ.copy()
 for key in ('CAMPAIGN','TUTORIAL','AUDIO','DETAIL','DISPLAY'):env['ROGUE_'+key+'_SAVE']=str(state/(key.lower()+'.save'))
 frames='35,210,230,260,290' if scene=='equipment' else ('35,215,230,250,270,274,283' if scene.startswith('crafting') else '300,600,900,1200,1500')
 env.update(ROGUE_CAPTURE_SAVE_FRAMES=frames,ROGUE_COMMANDER_LANG=lang,BEVY_ASSET_ROOT=str(root),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json')
 print('START',name,flush=True)
 with (folder/'capture.log').open('w') as log:p=subprocess.run(['nix','develop','--command','target/market-visual/debug/capture','frames',str(folder),str(count),scene,'0xA11CE',size],env=env,stdout=log,stderr=subprocess.STDOUT,timeout=480)
 text=(folder/'capture.log').read_text();expected='CONTROLLER_ROUTE_PASS' if scene.startswith('controller') else ('PASS equipment:' if scene=='equipment' else 'PASS '+scene+':')
 assert p.returncode==0 and expected in text and 'panicked at' not in text and f'wrote {count} simulation frames' in text,(name,p.returncode)
 print('PASS',name,flush=True)
