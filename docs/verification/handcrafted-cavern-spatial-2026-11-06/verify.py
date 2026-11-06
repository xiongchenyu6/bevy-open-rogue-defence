import hashlib,json,pathlib
root=pathlib.Path(__file__).resolve().parent
for name,expected in json.loads((root/'SHA256.json').read_text()).items():
 assert hashlib.sha256((root/name).read_bytes()).hexdigest()==expected,name
results=json.loads((root/'results.json').read_text());assert len(results)==2
for row in results:
 assert row['exit']==0
 case=root/row['case'];log=(case/'game.log').read_text()
 assert '[capture] PASS den-preview:' in log and 'wrote 400 simulation frames' in log
 assert 'ERROR' not in log and 'panicked' not in log
 assert len(list(case.glob('*.png')))==9
check=json.loads((root/'independent-path.json').read_text())
assert check['path_audit']['height_bands']==96 and check['path_audit']['collisions']==[]
assert hashlib.sha256((root/'SPATIAL-PROTOTYPE-NOT-FINAL.glb').read_bytes()).hexdigest()==check['asset_sha256']
assert 'error:' not in (root/'clippy.log').read_text()
print('PASS: archived native spatial evidence; final art and production camera not accepted')
