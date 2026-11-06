import pathlib,os,subprocess,time,json
p=pathlib.Path('/verify');s=p/'profile';assert not list(s.iterdir())
e=os.environ.copy();e.update(DISPLAY=':97',WINIT_UNIX_BACKEND='x11',XDG_CONFIG_HOME=str(s/'config'),XDG_RUNTIME_DIR=str(s),ROGUE_CAMPAIGN_SAVE=str(s/'campaign.save'),ROGUE_AUDIO_SAVE=str(s/'audio'),ROGUE_COMMANDER_LANG='en',ROGUE_COMMANDER_SEED='0xA11CE',VK_ICD_FILENAMES=str(next(pathlib.Path('/usr/share/vulkan/icd.d').glob('lvp_icd*.json'))))
ps=[];fs=[]
def start(a,n):
 f=(p/n).open('w');fs.append(f);q=subprocess.Popen(a,env=e,cwd='/tmp',stdout=f,stderr=subprocess.STDOUT);ps.append(q);return q
try:
 start(['Xvfb',':97','-screen','0','1600x1000x24','-nolisten','tcp','-ac'],'xvfb.log');time.sleep(1);start(['openbox'],'wm.log');time.sleep(1);a=start(['/game/rogue_commander'],'game.log');(p/'pid').write_text(str(a.pid));c=a.wait();(p/'exit.json').write_text(json.dumps({'game_exit':c})+'\n');assert c==0
finally:
 for q in reversed(ps):
  if q.poll()is None:q.terminate()
