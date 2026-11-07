import hashlib,json,pathlib,shutil,subprocess,sys,time,os,tarfile
root=pathlib.Path.cwd();out=root/'target/verification/ricochet-package-2026-11-08'
authorization=json.loads((out/'authorization.json').read_text())
assert authorization['build_and_smoke_authorized'] is True, 'Prepared only; await root stable-source GO'
expected=sys.argv[1]
assert isinstance(expected,str) and len(expected)==40 and all(c in '0123456789abcdef' for c in expected), 'Full final reviewed commit required'
assert expected==authorization['expected_source_commit'], 'Exact final source pin required'
dest=root/'target/desktop-packages-ricochet-nov8';package=dest/'rogue-commander-linux'
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def write(name,value):(out/name).write_text(json.dumps(value,indent=2)+'\n')
def command(args,name,timeout):
    started=time.monotonic()
    try:
        with (out/(name+'.log')).open('w') as log:
            p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT,timeout=timeout)
        result={'exit':p.returncode,'elapsed':time.monotonic()-started,'args':args}
    except subprocess.TimeoutExpired:
        result={'timeout':True,'elapsed':time.monotonic()-started,'args':args}
    write(name+'-process.json',result)
    print(name,result,flush=True)
    assert result.get('exit')==0,name
ALLOWED_ART_TOOLS = frozenset(('tools/build_demon_cavern.py', 'tools/verify_demon_cavern.py'))
INPUT_DIRS = ('src', 'assets', 'vendor', '.cargo', 'licenses', 'tools')
INPUT_FILES = ('docs/DESKTOP_PLAYER_README.md', 'Cargo.toml', 'Cargo.lock', 'build.rs', 'rust-toolchain', 'rust-toolchain.toml')
def audit_inputs(phase):
    assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==expected
    assert not subprocess.check_output(['git','status','--porcelain','--untracked-files=no'],text=True).strip(), \
        'Tracked source/index must match the frozen HEAD'
    untracked=set(filter(None,subprocess.check_output(
        ['git','ls-files','--others','--exclude-standard','-z']).decode().split('\0')))
    assert untracked <= ALLOWED_ART_TOOLS, ('Unreviewed untracked files', sorted(untracked-ALLOWED_ART_TOOLS))
    tree={}
    for line in subprocess.check_output(['git','ls-tree','-r','-z',expected]).split(b'\0'):
        if not line: continue
        metadata,name=line.split(b'\t',1);mode,kind,oid=metadata.decode().split();name=os.fsdecode(name)
        if '/' not in name or name in INPUT_FILES or any(name.startswith(folder+'/') for folder in INPUT_DIRS):
            assert kind=='blob', ('Unsupported input tree entry',name,kind)
            tree[name]=(mode,oid)
    for name in INPUT_FILES:
        if (root/name).exists() or (root/name).is_symlink():
            assert name in tree, ('Untracked root/package input, even if gitignored',name)
    ignored_python_cache=[]
    for folder in INPUT_DIRS:
        for path in (root/folder).rglob('*'):
            if not path.is_file() and not path.is_symlink(): continue
            name=str(path.relative_to(root))
            if folder == 'tools' and '__pycache__' in path.parts and path.suffix in ('.pyc','.pyo'):
                ignored_python_cache.append(name); continue
            assert name in tree or name in ALLOWED_ART_TOOLS, ('Untracked build/runtime input, even if gitignored',name)
    files={}
    reader=subprocess.Popen(['git','cat-file','--batch'],stdin=subprocess.PIPE,stdout=subprocess.PIPE)
    try:
        for name,(mode,oid) in sorted(tree.items()):
            reader.stdin.write((oid+'\n').encode());reader.stdin.flush()
            header=reader.stdout.readline().decode().split();assert header[1]=='blob',name
            blob=reader.stdout.read(int(header[2]));assert reader.stdout.read(1)==b'\n'
            path=root/name
            actual=os.readlink(path).encode() if mode=='120000' else path.read_bytes()
            assert actual==blob, ('Tracked input bytes differ from HEAD',name)
            files[name]=hashlib.sha256(actual).hexdigest()
    finally:
        reader.stdin.close();assert reader.wait(timeout=30)==0
    entrypoints=('tools/build_desktop_linux.sh','tools/desktop/Dockerfile','tools/package_desktop.py','Cargo.toml')
    for name in entrypoints:
        text=(root/name).read_text()
        assert all(pathlib.Path(tool).name not in text for tool in ALLOWED_ART_TOOLS), ('Art tool referenced by build entrypoint',name)
    report={'source_commit':expected,'files':files,'allowed_untracked_art_tools':{name:sha(root/name) for name in sorted(untracked)},
        'ignored_only_python_cache':sorted(ignored_python_cache),
        'art_exclusion_basis':'Outside Docker context tools/desktop; not referenced by SDK/package entrypoints; packaging copies assets/README/metadata/runtime libraries/licenses, never tools/. Workspace RO mount still contains these files; they are not declared compilation/package inputs.'}
    write('input-audit-'+phase+'.json',report)
    return report
before_inputs=audit_inputs('before')
assert not package.exists(),'Never overwrite an earlier package'
(out/'source-commit.txt').write_text(expected+'\n')
command(['tools/build_desktop_linux.sh',str(dest)],'build',1800)
after_inputs=audit_inputs('after')
assert after_inputs['files']==before_inputs['files'], 'Frozen tracked build/runtime input hashes changed during SDK build'
version=json.loads((package/'version.json').read_text());assert version['git_commit']==expected
manifest=json.loads((package/'package-manifest.json').read_text())['files']
assert version['linux_linkage']['checked'] is True and version['linux_linkage']['nix_dependencies']==[], 'No Nix runtime linkage permitted'
for row in manifest:assert sha(package/row['file'])==row['sha256'],row['file']
assert not any(row['file'].startswith('tools/') or pathlib.Path(row['file']).name in {pathlib.Path(name).name for name in ALLOWED_ART_TOOLS} for row in manifest), 'Art tool entered delivered package'
assert not any(row['sha256'] in set(before_inputs['allowed_untracked_art_tools'].values()) for row in manifest), 'Excluded art tool bytes entered package'
for row in manifest:
    if row['file'].startswith('assets/'):
        assert sha(root/row['file'])==row['sha256'],row['file']
assert sha(package/'assets/ui/armory/empty-rack.png')==sha(root/'assets/ui/armory/empty-rack.png')
write('package-check.json',{'source_commit':expected,'files_verified':len(manifest),'binary_sha256':sha(package/'rogue_commander'),'armory_sha256':sha(package/'assets/ui/armory/empty-rack.png')})
archive=dest/'rogue-commander-linux-0.1.0.tar.gz'
with archive.open('wb') as target:
    tar=subprocess.Popen(['tar','--sort=name','--mtime=@0','--owner=0','--group=0','--numeric-owner','-C',str(dest),'-cf','-','rogue-commander-linux'],stdout=subprocess.PIPE)
    gzip=subprocess.run(['gzip','-n'],stdin=tar.stdout,stdout=target,timeout=180)
    tar.stdout.close();assert tar.wait(timeout=30)==0 and gzip.returncode==0
delivered={str(f.relative_to(package)):f for f in package.rglob('*') if f.is_file()}
assert set(delivered)=={row['file'] for row in manifest}|{'package-manifest.json'}, 'Exact delivered fileset required'
with tarfile.open(archive,'r:gz') as packed:
    members={m.name.removeprefix('rogue-commander-linux/'):m for m in packed.getmembers() if m.isfile()}
    assert set(members)==set(delivered), 'Tar fileset differs from package'
    for filename,member in members.items():
        assert member.uid==member.gid==member.mtime==0, filename
        assert packed.extractfile(member).read()==delivered[filename].read_bytes(), filename
write('tar-check.json',{'files_verified':len(delivered),'exact_bytes':True,'normalized_metadata':True})
write('artifact.json',{'source_commit':expected,'package_directory':str(package.relative_to(root)),'tar':str(archive.relative_to(root)),'tar_bytes':archive.stat().st_size,'tar_sha256':sha(archive),'binary_sha256':sha(package/'rogue_commander'),'scope':'Offline read-only Ubuntu24/X11 software Vulkan short flow only; no full chapter, natural forge, hardware or Steam claim'})
folder=out/'smoke';harness=folder/'harness-source';harness.mkdir(parents=True)
helpers=('smoke_runtime.py','campaign_runtime.py','campaign_equipment_runtime.py','forge_runtime.py','forge_refresh.py','forge_restart_runtime.py')
for name in helpers:
    blob=subprocess.check_output(['git','show',expected+':tools/desktop/'+name])
    assert blob==(root/'tools/desktop'/name).read_bytes(),name
    (harness/name).write_bytes(blob)
write('harness-provenance.json',{'commit':expected,'files':{name:sha(harness/name) for name in helpers},'scope':'Short flow only now; six modules include ForgeV3 forge_refresh import transitively required by restart'})
image=subprocess.check_output(['docker','image','inspect','rogue-commander-linux-smoke:ubuntu24-oct8','--format','{{.Id}}'],text=True).strip()
name='rogue-ricochet-smoke-nov8'
args=['docker','run','--rm','--name',name,'--network','none','--mount',f'type=bind,src={package},dst=/game,readonly','--mount',f'type=bind,src={folder},dst=/verify','--mount',f'type=bind,src={harness},dst=/harness,readonly','--env','PACKAGE_COMMIT='+expected,'--workdir','/tmp',image,'python3','/harness/smoke_runtime.py']
write('runtime-image.json',{'id':image})
try:command(args,'smoke',600)
finally:subprocess.run(['docker','stop','-t','15',name],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
assert 'DESKTOP_RUNTIME_PASS' in (out/'smoke.log').read_text()
result=json.loads((folder/'result.json').read_text())
for key in ('package_hashes_verified','no_source','no_rust','no_nix','chapter_recruit_battle','fullscreen_restored','language_saved_and_reloaded','normal_quit_twice','real_guide_after_readiness','tutorial_dismissal_saved','restart_skips_automatic_guide_and_runs_battle','public_portrait_skill_draw','portrait_skill_click_resumed_battle'):assert result[key] is True,key
assert 1 <= result['portrait_skill_hands'] <= 8
print('CURRENT_PACKAGE_SMOKE_PASS',flush=True)
