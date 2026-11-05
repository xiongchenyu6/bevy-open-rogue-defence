from pathlib import Path
import subprocess,json,time,os
root=Path.cwd();out=root/'target/verification/empty-armory-r2-2026-11-06';results=[]
commands=[('full-tests.log',['nix','develop','--command','cargo','test','--locked','--release','--lib','-j2']),('clippy.log',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings']),('capture-build.log',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j2','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])]
for name,args in commands:
 started=time.monotonic()
 with (out/name).open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
 results.append({'log':name,'args':args,'exit':p.returncode,'elapsed':time.monotonic()-started});(out/'build-process.json').write_text(json.dumps(results,indent=2)+'\n')
 if p.returncode:raise SystemExit(p.returncode)
cases=[('en-desktop','equipment-empty','en','1280x720',150),('zh-compact','equipment-empty','zh-CN','667x375',150),('en-portrait','equipment-empty','en','390x844',150),('missing-recovery','equipment-empty-missing','en','1280x720',150),('owned-regression','equipment','en','1280x720',310)]
results=[]
for name,scene,lang,size,count in cases:
 folder=out/name;folder.mkdir();state=folder/'state';state.mkdir();env=os.environ.copy()
 for key in ['CAMPAIGN','TUTORIAL','AUDIO','DETAIL','DISPLAY']:env['ROGUE_'+key+'_SAVE']=str(state/(key.lower()+'.save'))
 frames='35,65,125' if scene.startswith('equipment-empty') else '35,210,230,260,290'
 env.update(ROGUE_CAPTURE_SAVE_FRAMES=frames,ROGUE_COMMANDER_LANG=lang,BEVY_ASSET_ROOT=str(root),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json')
 args=['nix','develop','--command','target/market-visual/debug/capture','frames',str(folder),str(count),scene,'0xA11CE',size];started=time.monotonic()
 with (folder/'capture.log').open('w') as log:p=subprocess.run(args,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=300)
 text=(folder/'capture.log').read_text();marker='EQUIPMENT_EMPTY_PASS' if scene.startswith('equipment-empty') else 'PASS equipment:'
 passed=p.returncode==0 and marker in text and f'wrote {count} simulation frames' in text and 'panicked at' not in text
 if scene.endswith('missing'):passed=passed and 'EQUIPMENT_EMPTY_MISSING_PASS actual image load failed' in text
 results.append({'name':name,'args':args,'scene':scene,'lang':lang,'size':size,'count':count,'exit':p.returncode,'passed':passed,'elapsed':time.monotonic()-started});(out/'native-process.json').write_text(json.dumps(results,indent=2)+'\n')
 if not passed:raise SystemExit(1)
