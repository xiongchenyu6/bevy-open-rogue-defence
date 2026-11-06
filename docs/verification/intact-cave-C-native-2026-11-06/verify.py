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
commands=json.loads((root/'commands.json').read_text());assert len(commands)==2 and all(row['exit']==0 for row in commands)
check=json.loads((root/'independent-path.json').read_text())
assert check['asset_sha256']=='7cc6fe0dfbda094226f5f051492f8aee8c4a449fc90844c574cedf36995b7176'
assert check['path_audit']['height_bands']==96 and check['path_audit']['collisions']==[]
assert hashlib.sha256((root/'capture-source.rs').read_bytes()).hexdigest()==json.loads((root/'source-inputs.json').read_text())['src/bin/capture/demon_cave.rs']
print('PASS native default/C400+400,18 PNGs,strict capture Clippy/build,actual body SAT; production camera/input adoption remains open')
