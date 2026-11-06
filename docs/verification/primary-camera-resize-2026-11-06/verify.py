import hashlib,json,pathlib
p=pathlib.Path(__file__).resolve().parent
for n,h in json.loads((p/'SHA256.json').read_text()).items():assert hashlib.sha256((p/n).read_bytes()).hexdigest()==h,n
assert '728 passed; 0 failed; 1 ignored'in(p/'checks/full-tests.log').read_text()
assert all(x['exit']==0 for x in json.loads((p/'checks/processes.json').read_text()))
assert json.loads((p/'before/smoke-process.json').read_text())['exit']==1
assert 'Attachments have differing sizes'in(p/'before/smoke/game.log').read_text()
print('ARCHIVED_CAMERA_RESIZE_SOURCE_PASS; rebuilt GPU/public-input acceptance is separate')
