import pathlib,json,hashlib
p=pathlib.Path(__file__).resolve().parent
m=json.loads((p/'verification.json').read_text())
for n,want in m['files_sha256'].items():
 assert hashlib.sha256((p/n).read_bytes()).hexdigest()==want,n
r=json.loads((p/'r3-test-process.json').read_text());assert r['exit']==0
assert 'Ran 121 tests' in (p/'r3-tests.log').read_text() and (p/'r3-tests.log').read_text().rstrip().endswith('OK')
print('PASS helper-only121tests and archived hashes; no live forge claim')
