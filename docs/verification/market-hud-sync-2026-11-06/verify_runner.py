from pathlib import Path
import subprocess,time,json,os
root=Path.cwd();out=root/'target/verification/market-hud-sync-2026-11-06';records=[]
commands=[('full-tests.log',['nix','develop','--command','cargo','test','--locked','--release','--lib','-j2']),('clippy.log',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings']),('capture-build.log',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j2','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])]
for name,args in commands:
 t=time.monotonic()
 with (out/name).open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
 records.append(dict(log=name,args=args,exit=p.returncode,elapsed=time.monotonic()-t));(out/'build-process.json').write_text(json.dumps(records,indent=2)+'\n')
 if p.returncode:raise SystemExit(p.returncode)
records=[]
for name,language,size in [('en-desktop','en','1280x720'),('zh-compact','zh-CN','667x375')]:
 folder=out/name;folder.mkdir();state=folder/'state';state.mkdir();env=os.environ.copy()
 for key in ['CAMPAIGN','TUTORIAL','AUDIO','DETAIL','DISPLAY']:env['ROGUE_'+key+'_SAVE']=str(state/(key.lower()+'.save'))
 env.update(ROGUE_CAPTURE_SAVE_FRAMES='35,44,45,46,65,145,185',ROGUE_COMMANDER_LANG=language,BEVY_ASSET_ROOT=str(root),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json')
 args=['nix','develop','--command','target/market-visual/debug/capture','frames',str(folder),'200','market-wallet','0xA11CE',size];t=time.monotonic()
 with (folder/'capture.log').open('w') as log:p=subprocess.run(args,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=300)
 text=(folder/'capture.log').read_text();passed=p.returncode==0 and 'PASS first purchase frame session=' in text and 'PASS accessible close toggle' in text and 'wrote 200 simulation frames' in text and 'panicked at' not in text
 records.append(dict(name=name,args=args,exit=p.returncode,passed=passed,elapsed=time.monotonic()-t));(out/'native-process.json').write_text(json.dumps(records,indent=2)+'\n')
 if not passed:raise SystemExit(1)
