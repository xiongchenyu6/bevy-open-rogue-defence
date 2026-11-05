#!/usr/bin/env python3
"""Private PulseAudio feasibility proof. Never accesses the host Pulse socket."""
import array, json, os, pathlib, signal, subprocess, time
ROOT = pathlib.Path(__file__).resolve().parent
REPO = ROOT.parents[2]
PULSE = '/nix/store/sn55x89zpz63bjpajwp706hab6k5b0z5-pulseaudio-17.0'
PLUGIN = '/nix/store/3q9c80jj2j0gw4h6si34ka66h2mchxf4-alsa-plugins-1.2.12/lib/alsa-lib'
RUNTIME = pathlib.Path(f'/tmp/rogue-audio-{os.getuid()}-nov5')
RUNTIME.mkdir(mode=0o700, exist_ok=True)
(RUNTIME/'state').mkdir(exist_ok=True)
socket = RUNTIME/'native'
config = ROOT/'private.pa'
config.write_text(f'''load-module module-native-protocol-unix socket={socket} auth-anonymous=1
load-module module-null-sink sink_name=rogue_private rate=48000 channels=2
set-default-sink rogue_private
set-default-source rogue_private.monitor
''')
alsa = ROOT/'private-alsa.conf'
alsa.write_text(f'''pcm.!default {{
 type pulse
 server "unix:{socket}"
}}
ctl.!default {{
 type pulse
 server "unix:{socket}"
}}
''')
env = os.environ.copy()
env.update(PULSE_SERVER=f'unix:{socket}', PULSE_RUNTIME_PATH=str(RUNTIME),
           PULSE_STATE_PATH=str(RUNTIME/'state'), ALSA_CONFIG_PATH=str(alsa), ALSA_PLUGIN_DIR=PLUGIN)
keys=['PULSE_SERVER','PULSE_RUNTIME_PATH','PULSE_STATE_PATH','ALSA_CONFIG_PATH','ALSA_PLUGIN_DIR']
import shlex
(ROOT/'env.sh').write_text('\n'.join(f'export {key}={shlex.quote(env[key])}' for key in keys)+'\n')
command=[PULSE+'/bin/pulseaudio','-n','--use-pid-file=no','--daemonize=no','--exit-idle-time=-1','--disable-shm=yes','--file='+str(config)]
(ROOT/'server.sh').write_text('#!/bin/sh\n. '+shlex.quote(str(ROOT/'env.sh'))+'\nexec '+shlex.join(command)+'\n')
(ROOT/'server.sh').chmod(0o755)

def ready(proc):
    for _ in range(100):
        if proc.poll() is not None: raise RuntimeError('private daemon exited')
        result=subprocess.run([PULSE+'/bin/pactl','info'],env=env,capture_output=True)
        if result.returncode==0:
            assert b'rogue_private' in result.stdout
            return result.stdout.decode()
        time.sleep(.05)
    raise RuntimeError('private daemon not ready')

def proof(round, mode):
    raw=ROOT/f'monitor-{round}-{mode}.s16le'
    recorder=subprocess.Popen([PULSE+'/bin/parecord','--raw','--format=s16le','--rate=48000','--channels=2','--latency-msec=20','--device=rogue_private.monitor',str(raw)],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
    try:
        time.sleep(.25)
        command = [PULSE+'/bin/paplay'] if mode == 'pulse' else ['python3',str(ROOT/'alsa-play.py')]
        subprocess.run(command+[str(REPO/'assets/audio/ultimate.wav')],env=env,check=True)
        time.sleep(1.)
    finally:
        recorder.send_signal(signal.SIGINT)
        recorder.communicate(timeout=5)
    samples=array.array('h',raw.read_bytes())
    peak=max(map(abs,samples),default=0)
    nonzero=sum(x !=0 for x in samples)
    assert peak>0 and nonzero>100
    return dict(round=round,mode=mode,samples=len(samples),nonzero=nonzero,peak=peak)

results=[]
for round in (1,2):
    with (ROOT/f'server-{round}.log').open('w') as log:
        proc=subprocess.Popen(command,env=env,stdout=log,stderr=subprocess.STDOUT)
        try:
            info=ready(proc)
            (ROOT/f'info-{round}.txt').write_text(info)
            results.append(proof(round, "pulse"))
            results.append(proof(round, "alsa"))
        finally:
            proc.terminate()
            proc.wait(timeout=10)
    assert not socket.exists(), 'private socket removed on clean shutdown'
(ROOT/'result.json').write_text(json.dumps(dict(scope='real private PulseAudio null sink, same socket restarted; not physical device or game recovery',env={k:env[k] for k in keys},results=results),indent=2)+'\n')
print(json.dumps(results))
