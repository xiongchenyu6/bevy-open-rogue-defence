import os, pathlib, subprocess
root=pathlib.Path.cwd()
out=root/'target/verification/passive-batch-2026-11-05/native'
for scenario in ('passive-storm-batch','passive-storm-split','passive-frost-batch','passive-frost-split','ur28-surge'):
    folder=out/scenario
    folder.mkdir(parents=True,exist_ok=True)
    state=folder/'state';state.mkdir(exist_ok=True)
    env=os.environ.copy()
    for key in ('CAMPAIGN','TUTORIAL','AUDIO','DETAIL','DISPLAY'):
        env['ROGUE_'+key+'_SAVE']=str(state/(key.lower()+'.save'))
    env.update(ROGUE_CAPTURE_SAVE_FRAMES='9,10,11,12,13,14,20,25,26,27,35,45' if scenario!='ur28-surge' else '60,180,239,264,300,395',ROGUE_COMMANDER_LANG='en',BEVY_ASSET_ROOT=str(root),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json')
    with (out/(scenario+'.log')).open('w') as log:
        p=subprocess.run(['nix','develop','--command','target/market-visual/debug/capture','frames',str(folder),'400' if scenario=='ur28-surge' else '50',scenario,'0xA11CE','1280x720'],env=env,stdout=log,stderr=subprocess.STDOUT,timeout=360)
    text=(out/(scenario+'.log')).read_text()
    assert p.returncode==0 and 'panicked at' not in text, (scenario,p.returncode)
    assert ('PASSIVE_BATCH_CHECK frame=45' in text if scenario!='ur28-surge' else '[capture] PASS' in text),scenario
    print(scenario,'PASS',flush=True)
