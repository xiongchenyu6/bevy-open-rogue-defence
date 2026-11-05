import pathlib,json,hashlib,subprocess,sys
p=pathlib.Path(__file__).resolve().parent
m=json.loads((p/'verification.json').read_text())
for n,want in m['files_sha256'].items():
 assert hashlib.sha256((p/n).read_bytes()).hexdigest()==want,n
for a,b in [('before-inputs.json','before-inputs-after.json'),('after-inputs.json','after-inputs-after.json'),('release-inputs-before.json','release-inputs-after.json')]:
 assert json.loads((p/a).read_text())==json.loads((p/b).read_text())
assert json.loads((p/'after-inputs-after.json').read_text())==json.loads((p/'release-inputs-after.json').read_text())
assert '689 passed; 0 failed; 1 ignored' in (p/'full-tests.log').read_text()
assert 'TERMINAL RELEASE PASS' in (p/'release-sequence.log').read_text()
subprocess.run([sys.executable,str(p/'compare.py')],check=True)
print('PASS native/source/asset/hash/release proof; scoped dagger timing only')
