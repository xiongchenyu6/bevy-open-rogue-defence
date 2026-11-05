import hashlib,json,pathlib,shutil,subprocess,sys,time
root=pathlib.Path.cwd();out=root/'target/verification/overlap-package-2026-11-06'
expected=sys.argv[1];dest=root/'target/desktop-packages-overlap-nov6';package=dest/'rogue-commander-linux'
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
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==expected
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip(),'Source must be committed and frozen'
assert not package.exists(),'Never overwrite an earlier package'
(out/'source-commit.txt').write_text(expected+'\n')
command(['tools/build_desktop_linux.sh',str(dest)],'build',1800)
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==expected
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
version=json.loads((package/'version.json').read_text());assert version['git_commit']==expected
manifest=json.loads((package/'package-manifest.json').read_text())['files']
for row in manifest:assert sha(package/row['file'])==row['sha256'],row['file']
assert sha(package/'assets/ui/armory/empty-rack.png')==sha(root/'assets/ui/armory/empty-rack.png')
write('package-check.json',{'source_commit':expected,'files_verified':len(manifest),'binary_sha256':sha(package/'rogue_commander'),'new_armory_sha256':sha(package/'assets/ui/armory/empty-rack.png')})
archive=dest/'rogue-commander-linux-0.1.0.tar.gz'
with archive.open('wb') as target:
    tar=subprocess.Popen(['tar','--sort=name','--mtime=@0','--owner=0','--group=0','--numeric-owner','-C',str(dest),'-cf','-','rogue-commander-linux'],stdout=subprocess.PIPE)
    gzip=subprocess.run(['gzip','-n'],stdin=tar.stdout,stdout=target,timeout=180)
    tar.stdout.close();assert tar.wait(timeout=30)==0 and gzip.returncode==0
write('artifact.json',{'source_commit':expected,'package_directory':str(package.relative_to(root)),'tar':str(archive.relative_to(root)),'tar_bytes':archive.stat().st_size,'tar_sha256':sha(archive),'binary_sha256':sha(package/'rogue_commander'),'scope':'Offline read-only Ubuntu24/X11 software Vulkan short flow only; no full chapter, natural forge, hardware or Steam claim'})
folder=out/'smoke';harness=folder/'harness-source';harness.mkdir(parents=True)
shutil.copy2(root/'tools/desktop/smoke_runtime.py',harness/'smoke_runtime.py')
write('harness-provenance.json',{'commit':expected,'files':{'smoke_runtime.py':sha(harness/'smoke_runtime.py')}})
image=subprocess.check_output(['docker','image','inspect','rogue-commander-linux-smoke:ubuntu24-oct8','--format','{{.Id}}'],text=True).strip()
name='rogue-overlap-smoke-nov6'
args=['docker','run','--rm','--name',name,'--network','none','--mount',f'type=bind,src={package},dst=/game,readonly','--mount',f'type=bind,src={folder},dst=/verify','--mount',f'type=bind,src={harness},dst=/harness,readonly','--env','PACKAGE_COMMIT='+expected,'--workdir','/tmp',image,'python3','/harness/smoke_runtime.py']
write('runtime-image.json',{'id':image})
try:command(args,'smoke',600)
finally:subprocess.run(['docker','stop','-t','15',name],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
assert 'DESKTOP_RUNTIME_PASS' in (out/'smoke.log').read_text()
result=json.loads((folder/'result.json').read_text())
for key in ('package_hashes_verified','no_source','no_rust','no_nix','chapter_recruit_battle','fullscreen_restored','language_saved_and_reloaded','normal_quit_twice','real_guide_after_readiness','tutorial_dismissal_saved','restart_skips_automatic_guide_and_runs_battle'):assert result[key] is True,key
print('OVERLAP_PACKAGE_SMOKE_PASS',flush=True)
