import pathlib,json,hashlib
p=pathlib.Path(__file__).resolve().parent
m=json.loads((p/'verification.json').read_text())
for n,want in m['files_sha256'].items():
 assert hashlib.sha256((p/n).read_bytes()).hexdigest()==want,n
a=json.loads((p/'r2-after-source-before.json').read_text())
b=json.loads((p/'r2-after-source-after.json').read_text())
assert a['source_sha256']==b['source_sha256']
assert '686 passed; 0 failed; 1 ignored' in (p/'r2-after-full-tests.log').read_text()
assert 'TERMINAL PASS' in (p/'r2-after-sequence.log').read_text()
print('PASS archived hashes, unchanged build source, 686 tests and terminal gate')
