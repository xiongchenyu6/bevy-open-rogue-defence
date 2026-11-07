from pathlib import Path
import hashlib,json,subprocess,tarfile
r=Path.cwd();p=r/'target/verification/forge-package-2026-11-08'
def sha(b):return hashlib.sha256(b).hexdigest()
def obj(n):return json.loads((p/n).read_text())
a=obj('input-audit-before.json');b=obj('input-audit-after.json');assert a['files']==b['files'];commit=a['source_commit'];assert commit==subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
reader=subprocess.Popen(['git','cat-file','--batch'],stdin=subprocess.PIPE,stdout=subprocess.PIPE)
for name,h in a['files'].items():
 reader.stdin.write((commit+':'+name+'\n').encode());reader.stdin.flush();header=reader.stdout.readline().split();assert header[1]==b'blob';data=reader.stdout.read(int(header[2]));assert reader.stdout.read(1)==b'\n';assert sha(data)==h;assert sha((r/name).read_bytes())==h
reader.stdin.close();assert reader.wait()==0
artifact=obj('artifact.json');package=r/artifact['package_directory'];archive=r/artifact['tar'];manifest=json.loads((package/'package-manifest.json').read_text())['files']
for row in manifest:assert sha((package/row['file']).read_bytes())==row['sha256']
assert sha((package/'rogue_commander').read_bytes())==artifact['binary_sha256'];assert sha(archive.read_bytes())==artifact['tar_sha256']
with tarfile.open(archive) as tar:
 files={m.name.removeprefix('rogue-commander-linux/'):m for m in tar.getmembers() if m.isfile()};assert set(files)=={row['file'] for row in manifest}|{'package-manifest.json'}
 for name,m in files.items():assert m.uid==m.gid==m.mtime==0;assert tar.extractfile(m).read()==(package/name).read_bytes()
elf=subprocess.check_output(['readelf','-d',str(package/'rogue_commander')]);(p/'root-readelf.txt').write_bytes(elf);assert b'/nix/store' not in elf
results=json.loads((p/'smoke/result.json').read_text());pairs={name:[v['first']['clock_seconds'],v['second']['clock_seconds']] for name,v in results['battle_clock_proofs'].items()}
receipt={'result':'PASS','source_commit':commit,'source_inputs':len(a['files']),'manifest_files':len(manifest),'tar_files':len(files),'binary_sha256':artifact['binary_sha256'],'tar_sha256':artifact['tar_sha256'],'three_clock_pairs':pairs,'root_viewed_originals':0,'known_visual_findings':[]}
(p/'root-artifact-check.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
