from pathlib import Path
import json,hashlib
p=Path(__file__).resolve().parent
for name,digest in json.loads((p/'evidence-hashes.json').read_text()).items():assert hashlib.sha256((p/name).read_bytes()).hexdigest()==digest,name
a=json.loads((p/'inputs-before.json').read_text());assert a==json.loads((p/'inputs-after-build.json').read_text())
assert hashlib.sha256((p/'demon_cave.rs').read_bytes()).hexdigest()==a['src/bin/capture/demon_cave.rs']
for step in ['format.log','clippy.log','build.log','front.log']:assert json.loads((p/(step+'.process.json')).read_text())['exit']==0
manifest=json.loads((p/'asset-reference.json').read_text())['manifest']['files'];assert len(manifest)==186
candidate=next(row for row in manifest if row['file']=='models/environment_demon_cavern_preview.glb');assert hashlib.sha256((p/'REJECTED-R31.glb').read_bytes()).hexdigest()==candidate['sha256']
for row in manifest:
 if row is not candidate:assert a['assets/'+row['file']]==row['sha256']
log=(p/'front.log').read_text();assert 'PASS den-preview:'in log and 'camera=front-side proposal'in log and 'panicked at'not in log and 'ERROR'not in log
assert len(list((p/'front').glob('frame*.png')))==8
assert json.loads((p/'result.json').read_text())['execution_passed']
print('PASS capture-only front-camera400updates and immutable186asset clone; camera/R31 art NOT adopted')
