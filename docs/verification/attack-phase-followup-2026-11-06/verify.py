"""Portable offline receipt verification; no builds, simulations or source mutation."""
from pathlib import Path
import gzip,hashlib,json,re,shutil,subprocess,sys,tempfile
r=Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
manifest=json.loads((r/'manifest.json').read_text())
actual={str(p.relative_to(r)) for p in r.rglob('*') if p.is_file() and '__pycache__' not in p.parts}
assert actual==set(manifest)|{'manifest.json'},(actual-set(manifest),set(manifest)-actual)
for n,h in manifest.items():assert sha((r/n).read_bytes())==h,n
provenance=json.loads((r/'provenance.json').read_text());inputs=json.loads((r/'inputs.json').read_text())
assert sha((r/'frozen/attacks.rs').read_bytes())==inputs['src/bin/capture/attacks.rs']==provenance['source_file_sha256']
assert provenance['terminal_observed_by_root']=={'ActiveState':'inactive','Result':'success','ExecMainStatus':0}
assert re.fullmatch('[0-9a-f]{64}',provenance['binary_sha256'])
raw=json.loads((r/'raw-manifest.json').read_text())
with tempfile.TemporaryDirectory() as tmp:
 t=Path(tmp);shutil.copy2(r/'analyze-current.py',t/'analyze-current.py')
 for kind,count in [('ember',24),('storm',13),('sword',0)]:
  n=f'current-{kind}.log';z=(r/(n+'.gz')).read_bytes();b=gzip.decompress(z);proof=raw[n]
  assert sha(z)==proof['gzip_sha256'] and sha(b)==proof['raw_sha256'] and len(b)==proof['raw_bytes']
  (t/n).write_bytes(b);lines=b.decode().splitlines()
  frames=[int(re.search(r'frame=(\d+)',x)[1]) for x in lines if x.startswith('RELEASE_PHASE ')]
  assert frames==list(range(1,211)),kind
  assert sum(x.startswith('PROJECTILE_RELEASE ') for x in lines)==count,kind
  assert sum(x.startswith('ATTACK_TIMING ') for x in lines)==80,kind
  text=b.decode();assert 'PASS: real basic attack damage' in text and 'wrote 210 simulation frames' in text
  assert 'panicked' not in text and 'ERROR' not in text
  receipt=json.loads((r/f'current-{kind}-result.json').read_text());assert receipt['returncode']==0
  assert receipt['command'][-3:]==['attack-'+kind,'0xA11CE','1280x720']
 subprocess.run([sys.executable,str(t/'analyze-current.py')],check=True,stdout=subprocess.DEVNULL)
 assert json.loads((t/'measured-summary.json').read_text())==json.loads((r/'measured-summary.json').read_text())
assert sorted(p.name for p in (r/'images').iterdir())==['ember-009.png','ember-010.png','ember-012.png','ember-015.png']
print('PASS: exact archive, lossless logs, all phase/release/timing rows and recomputed measured summary')
