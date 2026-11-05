import os,json,hashlib,subprocess,time,shutil
from pathlib import Path
from smoke_runtime import ocr_screenshot,find_text,normalized,wait_basic_commander
from campaign_equipment_runtime import inventory_open,inventory_count,wait_panel_open
from forge_runtime import close_visible_inventory,inventory_header_rows
OUT=Path('/verify');STATE=OUT/'state';GAME=Path('/game')
expected='49562e6fa1f1cf781eb9f0b8a4155e7a15ec22cb'
assert json.loads((GAME/'version.json').read_text())['git_commit']==expected
assert not Path('/nix').exists() and not shutil.which('cargo')
for r in json.loads((GAME/'package-manifest.json').read_text())['files']:
 assert hashlib.sha256((GAME/r['file']).read_bytes()).hexdigest()==r['sha256']
assert hashlib.sha256((STATE/'campaign.save').read_bytes()).hexdigest()=='da5bf0a9ca3f4032a265425f9989bda292d8c0c2622848157df6d2c0223ba825'
env=os.environ.copy();env.update(DISPLAY=':95',WINIT_UNIX_BACKEND='x11',XDG_CONFIG_HOME=str(STATE/'config'),XDG_RUNTIME_DIR=str(STATE),ROGUE_CAMPAIGN_SAVE=str(STATE/'campaign.save'),ROGUE_AUDIO_SAVE=str(STATE/'audio'),ROGUE_COMMANDER_LANG='en',ROGUE_COMMANDER_SEED='0xA11CE',VK_ICD_FILENAMES=str(next(Path('/usr/share/vulkan/icd.d').glob('lvp_icd*.json'))))
ps=[];handles=[];report={'trials':[],'startup':[],'completed':False}
def call(args):return subprocess.check_output(args,env=env,text=True,timeout=20).strip()
def start(args,name):
 f=(OUT/name).open('w');handles.append(f);p=subprocess.Popen(args,env=env,stdout=f,stderr=subprocess.STDOUT,cwd='/tmp');ps.append(p);return p
def shot(name):
 p=OUT/name;call(['import','-window',win,str(p)]);return p
def rows():return ocr_screenshot(shot('current-ui.png'),env)
def key(k):
 call(['xdotool','keydown',k]);time.sleep(.3);call(['xdotool','keyup',k]);time.sleep(1)
def click(x,y):
 call(['xdotool','mousemove','--window',win,str(x),str(y)]);call(['xdotool','mousedown','1']);time.sleep(.2);call(['xdotool','mouseup','1']);time.sleep(.5)
def wait(text):
 end=time.monotonic()+90
 while time.monotonic()<end:
  found=find_text(rows(),text)
  if found:return found
  time.sleep(.2)
 raise AssertionError('Missing '+text)
def clicktext(text):
 _,x,y,r,b=wait(text);click((x+r)//2,(y+b)//2)
try:
 start(['Xvfb',':95','-screen','0','1600x900x24','-nolisten','tcp','-ac'],'xvfb.log');time.sleep(1);start(['openbox'],'wm.log');app=start([str(GAME/'rogue_commander')],'game.log')
 end=time.monotonic()+90
 while True:
  try:win=call(['xdotool','search','--onlyvisible','--pid',str(app.pid)]).splitlines()[0];break
  except (subprocess.CalledProcessError,IndexError):assert time.monotonic()<end;time.sleep(.3)
 call(['xdotool','windowactivate','--sync',win]);wait('EXPEDITIONS');click(289,294)
 wait_basic_commander(shot,env,key,time.monotonic()+90,report['startup'])
 names=[r for r in rows() if normalized(r[0])=='stormsavant' and 150<r[2]<550];assert names
 _,x,y,r,b=min(names,key=lambda r:r[2]);click((x+r)//2,(y+b)//2);time.sleep(1)

 opened=wait_panel_open(shot,env,key,time.monotonic()+60,report.setdefault('open',[]),'selection','inventory')
 assert opened and inventory_count(opened[1])==0
 clicktext('Forge');time.sleep(1)
 from campaign_equipment_runtime import crop_text,INVENTORY_TITLE_CROP,selected_inventory_name
 cards=[('Bloodreaver',287,207),('Astral Prism',443,207),('Storm Bastion',287,339),('Suncoil',443,339),('Night Crown',287,471),('World Engine',443,471)]
 for mode in ['ordinary','held']:
  for index,(name,x,y) in enumerate(cards):
   before=shot(f'{mode}-{index}-before.png')
   caption=crop_text(before,f'144x20+{x-72}+{y+40}',env)
   assert normalized(caption)==normalized(name),(caption,name)
   record={'mode':mode,'index':index,'expected':name,'caption':caption,'observations':[]};report['trials'].append(record)
   call(['xdotool','mousemove','--window',win,str(x),str(y)])
   started=time.monotonic()
   if mode=='ordinary':
    call(['xdotool','click','1']);record['input']='xdotool click 1 (no hold)';time.sleep(.5)
    p=shot(f'{mode}-{index}-after.png');title=crop_text(p,INVENTORY_TITLE_CROP,env)
    record.update(title=title,matched=selected_inventory_name(title)==normalized(name),elapsed=time.monotonic()-started)
   else:
    deadline=time.monotonic()+10;record['down']=time.monotonic()
    try:
     call(['xdotool','mousedown','1'])
     while time.monotonic()<deadline:
      p=shot(f'{mode}-{index}-poll-{len(record["observations"]):02}.png');title=crop_text(p,INVENTORY_TITLE_CROP,env)
      record['observations'].append({'screenshot':p.name,'title':title})
      assert time.monotonic()<deadline,'Held selection observation exceeded budget'
      if selected_inventory_name(title)==normalized(name):break
      time.sleep(.1)
     else:raise AssertionError('Held selection never reached '+name)
    finally:
     call(['xdotool','mouseup','1']);record['up']=time.monotonic()
    p=shot(f'{mode}-{index}-after.png');title=crop_text(p,INVENTORY_TITLE_CROP,env)
    assert selected_inventory_name(title)==normalized(name),(name,title)
    record.update(title=title,matched=True,elapsed=time.monotonic()-started)
   print('SELECTION',mode,index,name,record['matched'],flush=True)
 report['completed']=True;print('EMPTY_FORGE_SELECTION_PASS 6 held actual OS selections',flush=True)

finally:
 (OUT/'result.json').write_text(json.dumps(report,indent=2)+'\n')
 for p in reversed(ps):
  if p.poll() is None:p.terminate()
 for p in reversed(ps):
  try:p.wait(timeout=10)
  except subprocess.TimeoutExpired:p.kill();p.wait()
 for f in handles:f.close()
