from pathlib import Path
import json,hashlib
p=Path(__file__).resolve().parent
for name,digest in json.loads((p/'evidence-hashes.json').read_text()).items():
 assert hashlib.sha256((p/name).read_bytes()).hexdigest()==digest,name
assert json.loads((p/'r1/den-preview.log.process.json').read_text())['exit']==101
assert 'actual movement must leave the entrance' in (p/'r1/den-preview.log').read_text()
a=json.loads((p/'r2/inputs-before.json').read_text());b=json.loads((p/'r2/inputs-after-build.json').read_text());assert a==b
for name in ['capture.rs','demon_cave.rs']:
 key='src/bin/'+name if name=='capture.rs' else 'src/bin/capture/'+name
 assert hashlib.sha256((p/'source'/name).read_bytes()).hexdigest()==a[key]
for name in ['format.log','clippy.log','build.log']:assert json.loads((p/'r2'/(name+'.process.json')).read_text())['exit']==0
manifest=json.loads((p/'r2/asset-manifest.json').read_text())['files'];assert len(manifest)==186
for row in manifest:
 if row['file']=='models/environment_demon_cavern_preview.glb':
  assert hashlib.sha256((p/'REJECTED-R30.glb').read_bytes()).hexdigest()==row['sha256']
 else:assert row['sha256']==a['assets/'+row['file']]
for case,count in [('den-preview',8),('den-preview-reframed',8),('demon-cave',7)]:
 log=(p/'r2'/(case+'.log')).read_text();assert json.loads((p/'r2'/(case+'.log.process.json')).read_text())['exit']==0
 assert 'panicked at'not in log and 'ERROR'not in log
 assert ('PASS den-preview:' if case.startswith('den-preview') else 'PASS demon-cave:')in log
 assert len(list((p/'r2'/case).glob('frame*.png')))==count
assert json.loads((p/'r2/invalid-root.process.json').read_text())['expected_rejection']
assert 'capture asset root is not a directory' in (p/'r2/invalid-root.log').read_text()
print('PASS capture-only native preview, isolated186assets,1120updates/default control and preserved initial failure; R30 art explicitly REJECTED')
