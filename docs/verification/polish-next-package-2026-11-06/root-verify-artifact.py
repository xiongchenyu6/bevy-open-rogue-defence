import hashlib,json,pathlib,tarfile
root=pathlib.Path.cwd();out=root/'target/verification/polish-next-package-2026-11-06'
receipt=json.loads((out/'artifact.json').read_text());package=root/receipt['package_directory']
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
version=json.loads((package/'version.json').read_text())
assert version['git_commit']=='0c12230d3bf25c2b1a070c80950e5388a3b7b2ec'
manifest=json.loads((package/'package-manifest.json').read_text())['files']
expected={row['file']:row for row in manifest}
actual={str(p.relative_to(package)) for p in package.rglob('*') if p.is_file()}
assert actual==set(expected)|{'package-manifest.json'}
for name,row in expected.items():
 path=package/name
 assert sha(path)==row['sha256'] and path.stat().st_size==row['bytes'],name
assert sha(package/'rogue_commander')==version['binary_sha256']==receipt['binary_sha256']
archive=root/receipt['tar'];assert sha(archive)==receipt['tar_sha256']
seen=set()
with tarfile.open(archive,'r:gz')as handle:
 for member in handle.getmembers():
  assert member.uid==0 and member.gid==0 and member.mtime==0
  if member.isdir():continue
  assert member.isfile() and member.name.startswith('rogue-commander-linux/')
  name=member.name[len('rogue-commander-linux/'):]
  assert name not in seen and handle.extractfile(member).read()==(package/name).read_bytes()
  seen.add(name)
assert seen==actual
before=json.loads((out/'input-audit-before.json').read_text());after=json.loads((out/'input-audit-after.json').read_text())
assert before['files']==after['files']
print('PASS root independent package/tar byte audit:',len(expected),'manifest files,',len(actual),'actual files,',len(before['files']),'frozen inputs; runtime acceptance separate')
