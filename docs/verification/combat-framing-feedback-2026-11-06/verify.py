import hashlib,json,pathlib,sys
p=pathlib.Path(__file__).resolve().parent
for name,expected in json.loads((p/'SHA256.json').read_text()).items():
 assert hashlib.sha256((p/name).read_bytes()).hexdigest()==expected,name
r=json.loads((p/'native/results.json').read_text());assert len(r)==18 and sum(x['frames']for x in r)==5320 and all(x['exit']==0 for x in r)
assert '726 passed; 0 failed; 1 ignored' in (p/'checks/full-tests.log').read_text()
assert all(x['exit']==0 for x in json.loads((p/'checks/processes.json').read_text()))
for x in r:
 log=(p/'native'/x['case']/'game.log').read_text();assert 'ERROR'not in log and 'panicked'not in log
if '--repo'in sys.argv:
 root=p.parents[2]
 for name,expected in json.loads((p/'checks/inputs-after.json').read_text()).items():
  assert hashlib.sha256((root/name).read_bytes()).hexdigest()==expected,name
print('ARCHIVED_COMBAT_FRAMING_FEEDBACK_PASS; recorded evidence, not a new native run')
