from pathlib import Path
import subprocess,os,json,time
root=Path.cwd();out=root/'target/verification/equipment-mouse-r3-2026-11-06';results=[]
assert [r['exit'] for r in json.loads((out/'build-process.json').read_text())]==[0,0]
for name,lang,size in [('en-desktop','en','1280x720'),('zh-compact','zh-CN','667x375')]:
 folder=out/name;folder.mkdir();state=folder/'state';state.mkdir();env=os.environ.copy()
 for key in ['CAMPAIGN','TUTORIAL','AUDIO','DETAIL','DISPLAY']:env['ROGUE_'+key+'_SAVE']=str(state/(key.lower()+'.save'))
 env.update(ROGUE_CAPTURE_SAVE_FRAMES='210,222,242,262,282,302,322,355',ROGUE_COMMANDER_LANG=lang,BEVY_ASSET_ROOT=str(root),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json')
 args=['nix','develop','--command','target/market-visual/debug/capture','frames',str(folder),'380','equipment-mouse','0xA11CE',size];started=time.monotonic()
 with (folder/'capture.log').open('w') as log:p=subprocess.run(args,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=300)
 text=(folder/'capture.log').read_text();passed=p.returncode==0 and 'EQUIPMENT_MOUSE_PASS all 5 same-tick + 5 held Item selections' in text and 'wrote 380 simulation frames' in text and 'panicked at' not in text
 results.append({'name':name,'language':lang,'size':size,'args':args,'exit':p.returncode,'passed':passed,'elapsed':time.monotonic()-started});(out/'native-process.json').write_text(json.dumps(results,indent=2)+'\n')
 if not passed:raise SystemExit(1)
