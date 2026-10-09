#!/usr/bin/env python3
"""Public UI equipment proof with copied real earned campaign state; no resource injection."""
import hashlib,json,os,re,shutil,subprocess,threading,time
from pathlib import Path
from smoke_runtime import ocr_screenshot,find_text,normalized
from campaign_runtime import read_progress
GAME,OUT=Path('/game'),Path('/verify')
def main():
    OUT.mkdir(exist_ok=True)
    version=json.loads((GAME/'version.json').read_text())
    assert version['git_commit']==os.environ['PACKAGE_COMMIT'],'Fresh fixed package required'
    state=OUT/'state'; initial=read_progress(state/'campaign.save')
    assert initial==dict(unlocked=5,currency=575,cleared='111100',upgrades=[5,0,0,0,0,0])
    assert not Path('/nix').exists() and not shutil.which('cargo')
    for row in json.loads((GAME/'package-manifest.json').read_text())['files']:
        assert hashlib.sha256((GAME/row['file']).read_bytes()).hexdigest()==row['sha256']
    env=os.environ.copy()
    for key in ('BEVY_ASSET_ROOT','CARGO_MANIFEST_DIR','WAYLAND_DISPLAY','ROGUE_TUTORIAL_SAVE'):env.pop(key,None)
    env.update(DISPLAY=':94',WINIT_UNIX_BACKEND='x11',XDG_CONFIG_HOME=str(state/'config'),XDG_RUNTIME_DIR=str(state),ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_AUDIO_SAVE=str(state/'audio'),ROGUE_COMMANDER_LANG='en',ROGUE_COMMANDER_SEED='0xA11CE',VK_ICD_FILENAMES=str(next(Path('/usr/share/vulkan/icd.d').glob('lvp_icd*.json'))))
    ps=[];files=[];stop=threading.Event(); worker=None
    report=dict(package_version=version,initial_progress=initial,scope='Packaged public X11 UI, earned kills, random market stock',purchases=[],crafting='Not attempted: requires genuine acquired recipe materials')
    def call(args):return subprocess.check_output(args,env=env,text=True).strip()
    def start(args,name):
        file=(OUT/name).open('w');files.append(file);p=subprocess.Popen(args,env=env,cwd='/tmp',stdout=file,stderr=subprocess.STDOUT);ps.append(p);return p
    def shot(name):
        p=OUT/name;call(['import','-window',win,str(p)]);return p
    def lines():return ocr_screenshot(shot('current-ui.png'),env)
    def wait(text,seconds=90):
        end=time.monotonic()+seconds
        while time.monotonic()<end:
            assert app.poll() is None
            found=find_text(lines(),text)
            if found:return found
            time.sleep(.3)
        raise RuntimeError('Missing UI: '+text)
    def wait_contains(text,seconds=90):
        end=time.monotonic()+seconds
        while time.monotonic()<end:
            assert app.poll() is None
            matches=[row for row in lines() if normalized(text) in normalized(row[0])]
            if matches:return matches
            time.sleep(.3)
        raise RuntimeError('Missing visible text: '+text)
    def wait_absent(prefix):
        deadline=time.monotonic()+20
        while time.monotonic()<deadline:
            if not any(normalized(t).startswith(normalized(prefix)) for t,*_ in lines()):return
            time.sleep(.5)
        raise RuntimeError('Panel did not close: '+prefix)
    def timer_seconds():
        native=shot('resume-timer-source.png');crop=OUT/'resume-timer-crop.png'
        call(['convert',str(native),'-crop','100x30+1170+20','-resize','500%','-colorspace','Gray','-negate',str(crop)])
        raw=call(['tesseract',str(crop),'stdout','-l','eng','--psm','7'])
        report.setdefault('resume_timer_ocr',[]).append(raw)
        m=re.fullmatch(r'(\d{2}):(\d{2})',raw.strip())
        assert m,('Actual resumed timer unreadable',raw)
        return int(m[1])*60+int(m[2])
    def click(x,y):
        call(['xdotool','mousemove','--window',win,str(x),str(y)]);call(['xdotool','mousedown','1']);time.sleep(.2);call(['xdotool','mouseup','1']);time.sleep(.5)
    def key(k):
        call(['xdotool','keydown',k]);time.sleep(.3);call(['xdotool','keyup',k]);time.sleep(1)
    def clicktext(text):
        _,x,y,r,b=wait(text);click((x+r)//2,(y+b)//2)
    def wallet(required=True):
        native=shot('wallet-source.png');crop=OUT/'wallet-crop.png'
        call(['convert',str(native),'-crop','120x20+1140+266','-resize','600%','-colorspace','Gray','-negate',str(crop)])
        raw=call(['tesseract',str(crop),'stdout','-l','eng','--psm','7'])
        report.setdefault('wallet_ocr',[]).append(raw)
        m=re.search(r'KILLS\s*(\d+)',raw,re.I)
        if not m:
            if required:raise RuntimeError('Actual isolated wallet unreadable: '+repr(raw))
            return None
        return int(m[1])
    def hud_wallet():
        native=shot('hud-wallet-source.png');crop=OUT/'hud-wallet-crop.png'
        call(['convert',str(native),'-crop','70x24+166+155','-resize','600%','-colorspace','Gray','-negate',str(crop)])
        raw=call(['tesseract',str(crop),'stdout','-l','eng','--psm','7'])
        report.setdefault('hud_wallet_ocr',[]).append(raw)
        assert re.fullmatch(r'\d+',raw.strip()),('Actual isolated HUD kill counter unreadable',raw)
        return int(raw.strip())
    def read_speed(depth=0):
        assert depth<20,"Cannot drain real modal choices before reading toolbar"
        native=shot('speed-source.png');crop=OUT/'speed-crop.png'
        call(['convert',str(native),'-crop','38x28+1029+32','-resize','600%','-colorspace','Gray','-negate',str(crop)])
        raw=call(['tesseract',str(crop),'stdout','-l','eng','--psm','7'])
        report.setdefault('speed_ocr',[]).append(raw)
        match=re.fullmatch(r'(1|2|4|8|16|32)\s*[x×]?',raw.strip(),re.I)
        if not match:
            rows=lines();terminal(rows)
            if any(any(title in normalized(t) for title in ('forgeanewbond','chooseyourskill','chooseyour','shapeyourarsenal')) for t,*_ in rows):
                report.setdefault('speed_hidden_by_choice',[]).append(rows);key('1');return read_speed(depth+1)
            raise RuntimeError('Actual speed crop unreadable: '+repr(raw))
        return int(match[1])
    def terminal(rows):
        title=next((title for title in ('NO LIVES REMAIN','CHAPTER CLEARED') if find_text(rows,title)),None)
        if title:
            shot('terminal.png');report['terminal']=title
            raise RuntimeError('Terminal reached before equipment proof: '+title)
    def operate():
        while not stop.is_set() and app.poll() is None:
            key('1');key('g');stop.wait(.2)
    try:
        start(['Xvfb',':94','-screen','0','1600x900x24','-nolisten','tcp','-ac'],'xvfb.log');time.sleep(1);start(['openbox'],'wm.log');time.sleep(1)
        app=start([str(GAME/'rogue_commander')],'game.log')
        end=time.monotonic()+120
        while True:
            try:win=call(['xdotool','search','--onlyvisible','--pid',str(app.pid)]).splitlines()[0];break
            except (subprocess.CalledProcessError,IndexError):
                assert time.monotonic()<end;time.sleep(.3)
        call(['xdotool','windowactivate','--sync',win]);wait('EXPEDITIONS');shot('startup.png');click(289,294)
        wait('Click a basic commander to begin')
        names=[r for r in lines() if normalized(r[0])=='stormsavant' and 150<r[2]<550];assert names
        _,x,y,r,b=min(names,key=lambda r:r[2]);click((x+r)//2,(y+b)//2)
        worker=threading.Thread(target=operate,daemon=True)
        end=time.monotonic()+90
        while time.monotonic()<end:
            if any('24:' in t.replace(' ','') for t,*_ in lines()):break
            key('1')
        else:raise RuntimeError('Battle countdown never advanced')
        print('stage: battle started; correcting actual toolbar to4x',flush=True)
        end=time.monotonic()+90
        while time.monotonic()<end:
            speeds={read_speed()}
            if speeds=={4}:
                time.sleep(1)
                if read_speed()==4:break
                continue
            if len(speeds)==1:click(995 if speeds.pop()<4 else 943,44)
            else:raise RuntimeError('Cannot read actual toolbar speed')
        else:raise RuntimeError('4x toolbar not verified')
        report['actual_speed']=4;shot('speed4.png');worker.start()
        print('stage: earn kills and wait for real3min unlock',flush=True)
        unlock_deadline=time.monotonic()+300
        while time.monotonic()<unlock_deadline:
            native=shot('unlock-source.png');crop=OUT/'timer-crop.png'
            call(['convert',str(native),'-crop','100x30+1170+20','-resize','500%','-colorspace','Gray','-negate',str(crop)])
            raw=call(['tesseract',str(crop),'stdout','-l','eng','--psm','7'])
            report.setdefault('unlock_timer_ocr',[]).append(raw)
            m=re.fullmatch(r'(\d{2}):(\d{2})',raw.strip())
            print('stage: real unlock countdown '+repr(raw),flush=True)
            if m and int(m[1])*60+int(m[2])<22*60:break
            if not m:terminal(lines())
            time.sleep(3)
        else:raise RuntimeError('Real3minute unlock not observed')
        shot('unlocked.png')
        for attempt in range(20):
            stop.set();worker.join(5);assert not worker.is_alive()
            for _ in range(6):key('1')
            actual=read_speed()
            for _ in range(5):
                if actual==4:break
                click(995 if actual<4 else 943,44);actual=read_speed()
            assert actual==4,'Speed drift could not be corrected'
            # Drain existing choice through public1 before B; a fresh draw never injected.
            for _ in range(6):key('1')
            key('b');shot(f'market-request-{attempt:02}.png');rows=lines();terminal(rows)
            report.setdefault('market_requests',[]).append(dict(attempt=attempt,visible=rows))
            print(json.dumps(dict(stage='market request',attempt=attempt,visible=[t[:120] for t,*_ in rows if '\n' not in t and any(w in t.upper() for w in ('KILLS','MARKET','EQUIPMENT','PAUSED','RESUME'))])),flush=True)
            if wallet(required=False) is not None:
                time.sleep(3);rows=lines();before=wallet();shot(f'market-{attempt:02}.png')
                candidates=[]
                for row in rows:
                    t,x,y,r,b=row;m=re.search(r'\[.*?GEAR\]\s*(.*?)\s+(\d+)\s*KILLS',t,re.I)
                    if m and x>900 and 290<y<600 and int(m[2])<=before:candidates.append((int(m[2]),m[1],row))
                if candidates:
                    price,name,row=min(candidates)
                    before_hud=hud_wallet();assert before_hud==before,('HUD/header wallets disagree',before_hud,before)
                    shot('before-purchase.png');report['pending_purchase']=dict(name=name,price=price,wallet_before=before,offer_ocr=row)
                    _,x,y,r,b=row;click((x+r)//2,(y+b)//2);time.sleep(3);afterrows=lines();after=hud_wallet()
                    assert before-after==price,(before,after,price)
                    report['purchases'].append(dict(name=name,price=price,wallet_before=before,wallet_after=after,offer_ocr=row));shot('purchase.png');key('b')
                    close_deadline=time.monotonic()+20
                    while time.monotonic()<close_deadline:
                        if wallet(required=False) is None:break
                        time.sleep(.5)
                    else:raise RuntimeError('Market did not close')
                    key('i');wait_contains('EQUIPMENT');shot('inventory.png')
                    wait_contains(name);clicktext('Forge');shot('recipes.png');report['crafting_visible_ocr']=lines();clicktext('Close');wait_absent('EQUIPMENT')
                    click(178,44);wait('ATTRIBUTES');shot('attributes.png');report['attributes_ocr']=lines();key('Escape');wait_absent('ATTRIBUTES');shot('resumed.png');report['inventory_verified']=True;break
                key('b')
            stop.clear();worker=threading.Thread(target=operate,daemon=True);worker.start();time.sleep(5)
        assert report['purchases'],'No naturally affordable equipment found within20 public market requests'
        before=timer_seconds();time.sleep(2);after=timer_seconds()
        report['resume_countdown_before']=before;report['resume_countdown_after']=after
        assert after<before,'Actual battlefield countdown did not resume'
        print('stage: acquired equipment, inspected inventory/recipes/attributes, actual countdown resumed',flush=True)
        click(640,30);wait('Settings');clicktext('Quit');time.sleep(1)
        if app.poll() is None:clicktext('Quit game')
        report['quit_exit']=app.wait(30);assert report['quit_exit']==0
        report['final_progress']=read_progress(state/'campaign.save');assert report['final_progress']==initial
        report['completed']=True
    finally:
        stop.set()
        if worker:worker.join(5)
        (OUT/'result.json').write_text(json.dumps(report,indent=2))
        for p in reversed(ps):
            if p.poll() is None:p.terminate()
        for f in files:f.close()
if __name__=='__main__':main()
