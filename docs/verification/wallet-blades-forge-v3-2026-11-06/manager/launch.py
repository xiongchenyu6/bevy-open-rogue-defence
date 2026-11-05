from pathlib import Path
import subprocess,sys,time,json
p=Path(__file__).resolve().parent
started=time.monotonic()
with (p/'manager.log').open('w') as log:r=subprocess.run([sys.executable,'-u',str(p/'manager.py')],stdout=log,stderr=subprocess.STDOUT)
(p/'manager-process.json').write_text(json.dumps({'exit':r.returncode,'elapsed':time.monotonic()-started},indent=2)+'\n')
raise SystemExit(r.returncode)
