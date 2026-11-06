"""Controlled actual-attack/output capture; private Pulse, no host audio changes."""
import os,pathlib,json,hashlib,subprocess,signal,time,wave,array,re,shutil
ROOT=pathlib.Path.cwd(); OUT=ROOT/'target/verification/melee-whoosh-implementation-2026-11-06/native-r5'
PULSE=pathlib.Path('/nix/store/sn55x89zpz63bjpajwp706hab6k5b0z5-pulseaudio-17.0/bin')
def receipt():
 paths=[ROOT/'Cargo.toml',ROOT/'Cargo.lock']+list((ROOT/'src').rglob('*.rs'))+list((ROOT/'assets').rglob('*'))
 return {str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths) if p.is_file()}
print('STAGE native build beginning',flush=True)
assert not (OUT/"hold-native").exists(), "Hold before native build to remove unrelated recursive rustfmt noise"
a=receipt();(OUT/'native-inputs-before.json').write_text(json.dumps(a,indent=2)+'\n')
cmd=['nix','develop','--command','cargo','build','--locked','--bin','capture','-j2','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0']
with (OUT/'native-build.log').open('w') as log:r=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT)
assert r.returncode==0
print('STAGE native build terminal exit0',flush=True)
b=receipt();(OUT/'native-inputs-after.json').write_text(json.dumps(b,indent=2)+'\n');assert a==b
exe=OUT/'capture';shutil.copy2(ROOT/'target/market-visual/debug/capture',exe)
(OUT/'native-binary.json').write_text(json.dumps(dict(command=cmd,sha256=hashlib.sha256(exe.read_bytes()).hexdigest()),indent=2)+'\n')
private=pathlib.Path('/tmp/rogue-melee-whoosh-'+str(os.getuid())+'-nov6-r5');private.mkdir(mode=0o700,exist_ok=False)
pa=OUT/'private.pa';pa.write_text(f'load-module module-native-protocol-unix socket={private}/native auth-anonymous=1\nload-module module-null-sink sink_name=rogue_melee rate=48000 channels=2\nset-default-sink rogue_melee\nset-default-source rogue_melee.monitor\n')
alsa=OUT/'private-alsa.conf';alsa.write_text(f'pcm.!default {{ type pulse server "unix:{private}/native" }}\nctl.!default {{ type pulse server "unix:{private}/native" }}\n')
env=os.environ.copy();env.update(PULSE_SERVER='unix:'+str(private/'native'),PULSE_RUNTIME_PATH=str(private),PULSE_STATE_PATH=str(private/'state'),ALSA_CONFIG_PATH=str(alsa),ALSA_PLUGIN_DIR='/nix/store/3q9c80jj2j0gw4h6si34ka66h2mchxf4-alsa-plugins-1.2.12/lib/alsa-lib',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json')
children=[];logs=[];result={}
def spawn(args,log,extra=None):
 f=open(log,'wb');logs.append(f);p=subprocess.Popen(args,cwd=ROOT,env=extra or env,stdout=f,stderr=subprocess.STDOUT,start_new_session=True);children.append(p);return p
def stop(p):
 if p.poll() is None:
  os.killpg(p.pid,signal.SIGTERM)
  try:p.wait(timeout=8)
  except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);p.wait(timeout=3)
try:
 server=spawn([str(PULSE/'pulseaudio'),'-n','--use-pid-file=no','--daemonize=no','--exit-idle-time=-1','--disable-shm=yes','--file='+str(pa)],OUT/'private-pulse.log')
 for _ in range(100):
  r=subprocess.run([str(PULSE/'pactl'),'info'],env=env,capture_output=True)
  if r.returncode==0 and b'rogue_melee' in r.stdout:break
  assert server.poll() is None;time.sleep(.05)
 else:raise RuntimeError('private audio not ready')
 (OUT/'pulse-info.txt').write_bytes(r.stdout)
 for kind in ['axe','daggers']:
  print('STAGE actual native '+kind+' beginning',flush=True)
  folder=OUT/('native-'+kind);folder.mkdir();state=folder/'state';state.mkdir()
  raw=folder/'output.raw';record_start=time.time();rec=spawn([str(PULSE/'parec'),'--raw','--format=s16le','--rate=48000','--channels=2','--latency-msec=20','--device=rogue_melee.monitor',str(raw)],folder/'record.log')
  e=env.copy();e.update(ROGUE_CAPTURE_MELEE_AUDIO='1',ROGUE_CAPTURE_ATTACK_TRACE='1',ROGUE_CAPTURE_SAVE_FRAMES='10,12,14,16,18,20,22,24,30,40,50,60,70',ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'))
  started=time.monotonic();game=spawn(['nix','develop','--command',str(exe),'frames',str(folder/'frames'),'2000','attack-'+kind,'0xA11CE','1280x720'],folder/'game.log',e)
  code=game.wait(timeout=300);time.sleep(.2);stop(rec);record_end=time.time()
  (folder/'process.json').write_text(json.dumps(dict(child_exit=code,wall_seconds=time.monotonic()-started,fixture='Automatic; 1 APS; virtual max_delta50ms; music0; actual first commit target displacement until pending retirement AND real>=.65s'),indent=2)+'\n')
  print('STAGE native '+kind+' game terminal '+str(code),flush=True)
  text=(folder/'game.log').read_text();assert code==0 and 'PASS: real basic attack damage' in text and 'panicked' not in text
  samples=array.array('h',raw.read_bytes());assert len(samples)>4800 and sum(s!=0 for s in samples)>100
  with wave.open(str(folder/'output.wav'),'wb') as w:w.setparams((2,2,48000,0,'NONE','not compressed'));w.writeframes(raw.read_bytes())
  voices=re.findall(r'MELEE_AUDIO frame=(\d+) real=([\d.]+).*?entity=(.*?) source=(.*?) sink=(true|false) health=([\d.]+) damage=([\d.]+)',text)
  contacts=re.findall(r'MELEE_CONTACT frame=(\d+) real=([\d.]+).*?damage=([\d.]+)',text)
  assert voices and any(v[4]=='true' for v in voices)
  first_voice=int(voices[0][0]);first_contact=next(int(v[0]) for v in contacts if float(v[2])>0)
  assert first_voice<=first_contact # Large frame device buffering is explicitly not an audible-lead guarantee.
  miss_begin=re.search(r'MELEE_MISS_BEGIN .*?wall=([\d.]+)',text)
  miss_end=re.search(r'MELEE_MISS_END .*?wall=([\d.]+)',text)
  assert miss_begin and miss_end, 'actual first miss not observed'
  miss_voices=[v for v in voices if int(v[0])<first_contact and ('swing_'+kind+'.wav') in v[3]]
  assert miss_voices and any(v[4]=='true' for v in miss_voices), 'no actual matching source/sink before first hit'
  duration=len(samples)/96000
  inferred_start=record_end-duration
  damage_wall=re.search(r'MELEE_DAMAGE_WALL frame=(\d+) wall=([\d.]+)',text)
  assert damage_wall and 'pending=0' in re.search(r'MELEE_MISS_END[^\n]*',text)[0]
  assert float(damage_wall[2])>float(miss_end[1])
  lo=max(0,int((float(miss_begin[1])-inferred_start)*48000))
  moved_hi=min(len(samples)//2,int((float(miss_end[1])-inferred_start)*48000))
  hi=min(len(samples)//2,int((float(damage_wall[2])-inferred_start)*48000))
  moved=samples[lo*2:moved_hi*2]
  with wave.open(str(folder/'targets-moved-window.wav'),'wb') as w:w.setparams((2,2,48000,0,'NONE','not compressed'));w.writeframes(moved.tobytes())
  isolated=samples[lo*2:hi*2]
  assert len(isolated)>4800 and sum(v!=0 for v in isolated)>100, 'no recorded energy in controlled zero-hit/music-muted window'
  with wave.open(str(folder/'zero-damage-window.wav'),'wb') as w:w.setparams((2,2,48000,0,'NONE','not compressed'));w.writeframes(isolated.tobytes())
  result[kind]=dict(child_exit=code,wall_seconds=time.monotonic()-started,voices=len(set(v[2] for v in voices)),saw_real_sink=True,first_voice_frame=first_voice,first_damage_frame=first_contact,recorded_seconds=len(samples)/96000,nonzero_samples=sum(s!=0 for s in samples),peak=max(map(abs,samples)),listening_verified=False,first_pending_retired_before_restore=True,targets_moved_begin_wall=float(miss_begin[1]),targets_moved_end_wall=float(miss_end[1]),first_damage_wall=float(damage_wall[2]),targets_moved_nonzero_samples=sum(v!=0 for v in moved),targets_moved_peak=max(map(abs,moved),default=0),zero_damage_nonzero_samples=sum(v!=0 for v in isolated),zero_damage_peak=max(map(abs,isolated)),window_may_include_followup_prehit_whoosh=True,record_start_wall=record_start,record_end_wall=record_end,inferred_pcm_start_wall=inferred_start,pcm_window_alignment='end-time minus actual PCM duration; approximate server/recorder latency, not sample-accurate event timestamp')
  (OUT/'native-result.json').write_text(json.dumps(result,indent=2)+'\n');print(kind,result[kind],flush=True)
 print('MELEE_WHOOSH_NATIVE_PASS',flush=True)
finally:
 for p in reversed(children):stop(p)
 for f in logs:f.close()
