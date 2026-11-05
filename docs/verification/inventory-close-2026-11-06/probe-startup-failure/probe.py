import os,json,hashlib,subprocess,time,shutil
from pathlib import Path
from smoke_runtime import ocr_screenshot,find_text,normalized,wait_basic_commander
from campaign_equipment_runtime import inventory_open,inventory_count,wait_panel_open
from forge_runtime import close_visible_inventory
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
 clicktext('STORM SAVANT');time.sleep(1)
 for index in range(3):
  opened=wait_panel_open(shot,env,key,time.monotonic()+60,report.setdefault('open',[]),f'trial-{index}','inventory')
  assert opened is not None and inventory_count(opened[1])==0
  before=shot(f'trial-{index}-before.png');started=time.monotonic();trial={'index':index,'ordinary':{},'held':[]};report['trials'].append(trial)
  _,x,y,r,b=wait('Close');click((x+r)//2,(y+b)//2)
  image=shot(f'trial-{index}-ordinary.png');observed=ocr_screenshot(image,env)
  trial['ordinary']={'elapsed':time.monotonic()-started,'inventory_open':inventory_open(observed),'close':find_text(observed,'Close'),'screenshot':image.name}
  # Reopen if ordinary click succeeded; held trial always begins from actual open empty panel.
  if not inventory_open(observed):
   assert not find_text(observed,'Close')
   opened=wait_panel_open(shot,env,key,time.monotonic()+60,report['open'],f'held-{index}','inventory');assert opened and inventory_count(opened[1])==0
  def observed_rows():
   p=shot(f'trial-{index}-held-{len(trial["held"]):02}.png');result=ocr_screenshot(p,env);trial['held'].append({'screenshot':p.name,'rows':result});return result
  def down(x,y):
   call(['xdotool','mousemove','--window',win,str(x),str(y)]);call(['xdotool','mousedown','1'])
  heldstart=time.monotonic()
  close_visible_inventory(observed_rows,down,lambda:call(['xdotool','mouseup','1']),time.monotonic()+10,trial['held'])
  actual=rows();assert not inventory_open(actual) and not find_text(actual,'Close')
  trial['held_elapsed']=time.monotonic()-heldstart;shot(f'trial-{index}-closed.png');print('PASS empty Close trial',index,flush=True)
 report['completed']=True;print('EMPTY_INVENTORY_CLOSE_PASS 3 actual OS trials',flush=True)
finally:
 (OUT/'result.json').write_text(json.dumps(report,indent=2)+'\n')
 for p in reversed(ps):
  if p.poll() is None:p.terminate()
 for p in reversed(ps):
  try:p.wait(timeout=10)
  except subprocess.TimeoutExpired:p.kill();p.wait()
 for f in handles:f.close()
