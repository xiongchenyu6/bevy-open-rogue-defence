import pathlib,subprocess,sys,time,json
p=pathlib.Path(__file__).resolve().parent;base=['docker','exec','rogue-cave-feel-manual-x11-nov6','env','DISPLAY=:97']
records=[]
def run(a):
 t=time.time();q=subprocess.run(base+a,capture_output=True,text=True);records.append({'args':a,'start_wall':t,'end_wall':time.time(),'exit':q.returncode,'stdout':q.stdout,'stderr':q.stderr});q.check_returncode();return q.stdout.strip()
try:
 win=run(['xdotool','search','--onlyvisible','--pid',(p/'pid').read_text().strip()]).splitlines()[0]
 kind,*a=sys.argv[1:]
 if kind=='shot':
  run(['xdotool','getwindowgeometry','--shell',win]);run(['import','-window',win,'/verify/'+a[0]+'.png'])
 elif kind=='click':
  run(['xdotool','windowactivate','--sync',win]);run(['xdotool','mousemove','--window',win,*a]);run(['xdotool','mousedown','1']);time.sleep(.15);run(['xdotool','mouseup','1'])
 elif kind=='key':
  run(['xdotool','windowactivate','--sync',win]);run(['xdotool','keydown',a[0]]);time.sleep(.3);run(['xdotool','keyup',a[0]])
 elif kind=='size':run(['xdotool','windowsize',win,*a])
 else:raise ValueError(kind)
finally:
 with(p/'actions.jsonl').open('a')as f:
  for row in records:f.write(json.dumps(row)+'\n')
