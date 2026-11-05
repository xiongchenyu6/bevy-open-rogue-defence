#!/usr/bin/env python3
"""Run only when the audio-backend capture fixture has been built.
Owns only private server/game/recorder children. Does not touch host audio.
"""
import argparse, array, json, os, pathlib, re, shlex, signal, subprocess, time
ROOT=pathlib.Path(__file__).resolve().parent
REPO=ROOT.parents[2]
PULSE='/nix/store/sn55x89zpz63bjpajwp706hab6k5b0z5-pulseaudio-17.0/bin'

class Case:
    def __init__(self, folder, mode, timeout, frames):
        self.folder,self.mode,self.timeout,self.frames=folder,mode,timeout,frames
        folder.mkdir(parents=True,exist_ok=False)
        self.env=os.environ.copy()
        for line in (ROOT/'env.sh').read_text().splitlines():
            key,value=line.removeprefix('export ').split('=',1)
            self.env[key]=shlex.split(value)[0]
        self.env.update(XDG_CONFIG_HOME=str(folder/'preferences'),ROGUE_AUDIO_SAVE=str(folder/'audio-preferences.json'),
                        ROGUE_CAPTURE_AUDIO_MODE=mode,ROGUE_CAPTURE_SAVE_FRAMES=frames)
        self.children=[];self.logs=[];self.server=None;self.game=None;self.offset=0;self.tail='';self.states=[];self.text=''
        self.events=(folder/'driver.jsonl').open('w')
        self.deadline=time.monotonic()+timeout
    def event(self, kind, **data):
        self.events.write(json.dumps(dict(wall_time=time.time(),event=kind,**data))+'\n');self.events.flush()
    def child(self, command, logname):
        log=(self.folder/logname).open('wb');self.logs.append(log)
        child=subprocess.Popen(command,cwd=REPO,env=self.env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
        self.children.append(child);self.event('spawn',pid=child.pid,command=command)
        return child
    def stop(self, child):
        if child and child.poll() is None:
            os.killpg(child.pid,signal.SIGTERM)
            try: child.wait(timeout=8)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid,signal.SIGKILL);child.wait(timeout=5)
            self.event('stop',pid=child.pid,returncode=child.returncode)
    def pulse(self, round):
        self.server=self.child([str(ROOT/'server.sh')],f'server-{round}.log')
        for _ in range(100):
            self.guard(check_game=False)
            if self.server.poll() is not None: raise RuntimeError('private Pulse server exited')
            result=subprocess.run([PULSE+'/pactl','info'],env=self.env,capture_output=True,timeout=3)
            if result.returncode==0 and b'rogue_private' in result.stdout:
                (self.folder/f'server-{round}-info.txt').write_bytes(result.stdout)
                self.event('private_server_ready',round=round)
                return
            time.sleep(.05)
        raise RuntimeError('private Pulse readiness timeout')
    def guard(self, check_game=True):
        if time.monotonic()>self.deadline: raise TimeoutError('case exceeded wall-clock budget')
        if check_game and self.game and self.game.poll() is not None:
            raise RuntimeError(f'game exited early: {self.game.returncode}')
    def poll(self):
        path=self.folder/'capture.log'
        if path.exists():
            with path.open('r',errors='replace') as log:
                log.seek(self.offset);chunk=log.read();self.offset=log.tell()
            self.text+=chunk;self.tail+=chunk
            lines=self.tail.split('\n');self.tail=lines.pop()
            for line in lines:
                if 'AUDIO_BACKEND_STATE ' in line:
                    state=json.loads(line.split('AUDIO_BACKEND_STATE ',1)[1]);self.states.append(state)
                    self.event('state',state=state)
            if 'panicked at' in self.text or 'thread \'main\' panicked' in self.text: raise RuntimeError('native panic')
    def wait(self, predicate, label):
        while True:
            self.poll()
            if predicate():
                self.event('condition',label=label);return
            self.guard();time.sleep(.05)
    def record(self, phase):
        # Latency is explicit: default parecord buffering hid short fixture audio.
        raw=self.folder/f'{phase}.s16le'
        recorder=self.child([PULSE+'/parecord','--raw','--format=s16le','--rate=48000','--channels=2',
            '--latency-msec=20','--device=rogue_private.monitor',str(raw)],f'{phase}-recorder.log')
        start=time.monotonic()
        try:
            while time.monotonic()-start<2.25:
                self.poll();self.guard();time.sleep(.05)
        finally: self.stop(recorder)
        samples=array.array('h',raw.read_bytes())
        result=dict(samples=len(samples),nonzero=sum(x!=0 for x in samples),peak=max(map(abs,samples),default=0))
        assert result['nonzero']>100 and result['peak']>0, f'no actual output at {phase}: {result}'
        self.event('recording',phase=phase,**result)
        return result
    def sink_inputs(self, phase):
        result=subprocess.run([PULSE+'/pactl','--format=json','list','sink-inputs'],env=self.env,capture_output=True,timeout=5)
        (self.folder/f'{phase}-sink-inputs.json').write_bytes(result.stdout)
        assert result.returncode==0,'private sink input listing failed'
        inputs=json.loads(result.stdout)
        matches=[]
        for item in inputs:
            props=item.get('properties',{})
            pid=props.get('application.process.id')
            name=props.get('application.process.binary','')
            if pid==str(self.game.pid) or 'capture' in name:
                matches.append(props)
        assert matches, f'no actual capture playback stream in private server: {inputs}'
        self.event('game_sink_input',phase=phase,matches=matches)
    def run(self):
        assert not pathlib.Path(self.env['PULSE_SERVER'].removeprefix('unix:')).exists(),'private socket already occupied; refusing to touch existing process'
        try:
            self.pulse('initial')
            command=['nix','develop','--command',str(REPO/'target/market-visual/debug/capture'),
                     'frames',str(self.folder/'frames'),'1200','audio-backend','0xA11CE','1280x720']
            (self.folder/'command.json').write_text(json.dumps(dict(command=command,env={k:self.env[k] for k in self.env if k.startswith(('PULSE_','ALSA_','ROGUE_CAPTURE_')) or k in ('XDG_CONFIG_HOME','ROGUE_AUDIO_SAVE')}),indent=2)+'\n')
            self.game=self.child(command,'capture.log')
            self.wait(lambda: bool(self.states and self.states[-1].get('phase') == 'initial' and self.states[-1].get('available') and self.states[-1].get('sink_present')),'initial actual sink')
            initial=self.states[-1].copy()
            self.sink_inputs('initial');before=self.record('initial')
            self.stop(self.server)
            self.wait(lambda: bool(self.states and not self.states[-1].get('available') and not self.states[-1].get('sink_present')),'actual output unavailable and sink detached')
            hold=time.monotonic()
            while time.monotonic()-hold<6:
                self.poll();self.guard()
                assert not self.states[-1].get('available'), 'output unexpectedly available while private server absent'
                time.sleep(.05)
            self.event('outage_hold_complete',seconds=time.monotonic()-hold)
            self.pulse('restarted')
            self.wait(lambda: bool(self.states and self.states[-1].get('available') and self.states[-1].get('sink_present') and self.states[-1].get('sink_generation',0)>initial.get('sink_generation',0)),'new recovered actual sink')
            recovered=self.states[-1].copy()
            assert recovered['music_entity']==initial['music_entity'],'music entity replaced'
            assert abs(recovered['gain']-initial['gain'])<1e-6,'music gain changed'
            assert abs(recovered['settings_gain']-initial['settings_gain'])<1e-6,'playback settings changed'
            assert recovered['phase']=='restored','native did not acknowledge actual recovery'
            self.sink_inputs('recovered');after=self.record('recovered')
            self.wait(lambda: 'AUDIO_BACKEND_PASS' in self.text,'native explicit recovery pass')
            while self.game.poll() is None:
                self.poll()
                if time.monotonic()>self.deadline: raise TimeoutError('normal completion exceeded wall budget')
                time.sleep(.05)
            self.poll()
            assert self.game.returncode==0,f'game returncode {self.game.returncode}'
            assert '[capture] wrote 1200 simulation frames to ' in self.text,'missing normal frame completion'
            result=dict(mode=self.mode,passed=True,initial=initial,recovered=recovered,recordings=dict(initial=before,recovered=after))
            (self.folder/'result.json').write_text(json.dumps(result,indent=2)+'\n')
            self.event('PASS');return result
        except BaseException as error:
            self.event('FAIL',error=repr(error));raise
        finally:
            for child in reversed(self.children):self.stop(child)
            for log in self.logs:log.close()
            self.events.close()

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--timeout',type=float,default=240)
    parser.add_argument('--save-frames',default='20,80,200,500,1199')
    parser.add_argument('--mode',choices=['both','paused','8x'],default='both')
    args=parser.parse_args()
    batch=ROOT/('native-'+time.strftime('%Y%m%d-%H%M%S')+f'-{os.getpid()}')
    batch.mkdir(exist_ok=False)
    results=[]
    for mode in (['paused','8x'] if args.mode=='both' else [args.mode]):
        results.append(Case(batch/mode,mode,args.timeout,args.save_frames).run())
    (batch/'summary.json').write_text(json.dumps(results,indent=2)+'\n')
    print(batch)
if __name__=='__main__': main()
