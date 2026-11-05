#!/usr/bin/env python3
"""Public UI equipment proof with copied real earned campaign state; no resource injection."""
import hashlib,json,os,re,shutil,subprocess,threading,time
from pathlib import Path
from smoke_runtime import ocr_screenshot,find_text,normalized,read_hud_wallet,read_toolbar_speed
from campaign_runtime import read_progress
from campaign_equipment_runtime import (TILE_LEFTS, PREVIEW_CROP, tile_name_text,
    screenshot_wallet, inspected_offer, choice_pending, crop_text, inventory_count,
    inventory_icons, inventory_open, selected_inventory_name, market_closed,
    has_gear_rows, verify_deduction, screenshot_battle_seconds, wait_inventory_open)

MATERIALS = frozenset(('Arc Lens', 'Echo Core'))


def visible_timer(shot, read_timer, read_rows, choose, deadline, now=time.monotonic):
    """Handle a late automatic choice using the same public screenshot.

    A missing timer alone never authorizes input. Only a positively recognized
    choice heading permits one ordinal; other unreadable HUDs fail unchanged.
    """
    while now() < deadline:
        image = shot()
        try:
            value = read_timer(image)
        except AssertionError:
            rows = read_rows(image)
            if not choice_pending(rows):
                raise
            assert now() < deadline, 'Choice timer deadline exhausted before input'
            choose('1')
        else:
            assert now() < deadline, 'Timer read exceeded deadline'
            return value
    raise AssertionError('Recognized choices did not expose a timer within90seconds')


def material_name(name):
    return next((canonical for canonical in MATERIALS if normalized(name) == normalized(canonical)), None)


def offer_identity(offer):
    return (normalized(offer['name']), offer['price'], normalized(offer['category']), offer['index'])


def poll_preview(sample, index, expected=None, timeout=8, pause=time.sleep):
    """Retry stale/public unreadable preview pixels; never click to select."""
    deadline = time.monotonic() + timeout
    errors = []
    for _ in range(10):
        assert time.monotonic() < deadline, ('Preview polling expired', errors)
        try:
            tile, price, preview = sample()
            offer = inspected_offer(tile, price, preview, index)
            if expected is not None:
                assert offer_identity(offer) == offer_identity(expected), 'Selected offer identity changed'
            return offer
        except AssertionError as error:
            errors.append(str(error))
            pause(.2)
    raise AssertionError(('Preview never matched exact visible tile/expected identity', errors))


def natural_material(offers, wallet, acquired):
    assert len(offers) == 5 and {offer['index'] for offer in offers} == set(range(5))
    acquired_keys = {normalized(name) for name in acquired}
    candidates = [dict(offer, name=material_name(offer['name']), observed_name=offer['name'])
                  for offer in offers if normalized(offer['category']) == 'equipment'
                  and material_name(offer['name']) is not None
                  and normalized(offer['name']) not in acquired_keys
                  and 0 < offer['price'] <= wallet]
    return min(candidates, key=lambda offer: (offer['price'], offer['index']), default=None)


def verify_recipe_selection(title, materials, action):
    assert selected_inventory_name(title) == 'astralprism', ('Wrong recipe selected', title)
    parts = materials.strip().split('+')
    assert len(parts) == 2, ('Recipe needs exactly two materials', materials)
    names = [re.sub(r"^[^A-Za-z0-9]+", '', part.strip()) for part in parts]
    assert [normalized(name) for name in names] == ['arclens', 'echocore'], \
        ('Unreadable exact ordered recipe materials', materials)
    assert action.strip() == 'Forge equipment', ('Recipe is not ready', action)


def verify_crafted_inventory(count, details, acquired):
    assert set(acquired) == MATERIALS and len(acquired) == 2
    assert count == 1 and details == ['astralprism'], \
        ('Ingredients not replaced by exactly one Astral Prism', count, details)

GAME,OUT=Path('/game'),Path('/verify')
def main():
    OUT.mkdir(exist_ok=True)
    version=json.loads((GAME/'version.json').read_text())
    assert version['git_commit']==os.environ['PACKAGE_COMMIT'],'Fresh fixed package required'
    state=OUT/'state'; initial=read_progress(state/'campaign.save')
    initial_save_hash=hashlib.sha256((state/'campaign.save').read_bytes()).hexdigest()
    assert initial==dict(unlocked=5,currency=575,cleared='111100',upgrades=[5,0,0,0,0,0])
    assert not Path('/nix').exists() and not shutil.which('cargo')
    for row in json.loads((GAME/'package-manifest.json').read_text())['files']:
        assert hashlib.sha256((GAME/row['file']).read_bytes()).hexdigest()==row['sha256']
    env=os.environ.copy()
    for key in ('BEVY_ASSET_ROOT','CARGO_MANIFEST_DIR','WAYLAND_DISPLAY','ROGUE_TUTORIAL_SAVE'):env.pop(key,None)
    env.update(DISPLAY=':94',WINIT_UNIX_BACKEND='x11',XDG_CONFIG_HOME=str(state/'config'),XDG_RUNTIME_DIR=str(state),ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_AUDIO_SAVE=str(state/'audio'),ROGUE_COMMANDER_LANG='en',ROGUE_COMMANDER_SEED='0xA11CE',VK_ICD_FILENAMES=str(next(Path('/usr/share/vulkan/icd.d').glob('lvp_icd*.json'))))
    ps=[];files=[];stop=threading.Event(); worker=None
    report=dict(package_version=version,initial_progress=initial,scope='Packaged public X11 UI, earned kills, random market stock',purchases=[],crafting='Pending same-run Arc Lens + Echo Core -> Astral Prism', acquired_materials=[])
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
    def timer_seconds():
        def choice_rows(image):
            rows=ocr_screenshot(image,env)
            if choice_pending(rows):
                evidence=OUT/f'resume-choice-{len(report.get("resume_clock_choices",[]))}.png'
                shutil.copy2(image,evidence)
                report.setdefault('resume_clock_choices',[]).append(dict(screenshot=evidence.name,rows=rows))
            return rows
        value=visible_timer(lambda:shot('resume-timer-source.png'),
            lambda image:screenshot_battle_seconds(image,env),choice_rows,key,
            time.monotonic()+90)
        report.setdefault('resume_timer_reads',[]).append(value)
        return value
    def click(x,y):
        call(['xdotool','mousemove','--window',win,str(x),str(y)]);call(['xdotool','mousedown','1']);time.sleep(.2);call(['xdotool','mouseup','1']);time.sleep(.5)
    def key(k):
        call(['xdotool','keydown',k]);time.sleep(.3);call(['xdotool','keyup',k]);time.sleep(1)
    def clicktext(text):
        _,x,y,r,b=wait(text);click((x+r)//2,(y+b)//2)
    def wallet():
        value, raw = screenshot_wallet(shot('wallet-source.png'), env)
        report.setdefault('wallet_ocr', []).append(raw)
        return value
    def drain_choices():
        end = time.monotonic() + 90
        for _ in range(30):
            rows = lines(); terminal(rows)
            if not choice_pending(rows): return
            assert time.monotonic() < end, 'Choice queue exceeded90seconds'
            key('1')
        assert not choice_pending(lines()), 'Choice queue not drained after30selections'
    def inspect_offer(index, prefix, expected=None):
        left = TILE_LEFTS[index]
        call(['xdotool','mousemove','--window',win,str(left+60),'365'])
        def sample():
            image=shot(prefix+'.png')
            return (tile_name_text(image,left,env),
                    crop_text(image,f'39x23+{left+52}+384',env,True),
                    crop_text(image,PREVIEW_CROP,env))
        return poll_preview(sample,index,expected)
    def close_market_panel():
        key('b')
        end = time.monotonic() + 20
        while time.monotonic() < end:
            if market_closed(lines()): return
            time.sleep(.3)
        raise RuntimeError('Market did not close')
    def close_inventory():
        clicktext('Close')
        rows = lines()
        assert not inventory_open(rows) and not find_text(rows, 'Close'), 'Equipment did not close'
    def inspect_inventory(prefix):
        rows = lines(); count = inventory_count(rows)
        details = []
        end = time.monotonic() + 90
        for index, (x, y) in enumerate(inventory_icons(count)):
            assert time.monotonic() < end, 'Inventory search exceeded90seconds'
            click(x, y)
            image = shot(f'{prefix}-icon-{index}.png')
            text = crop_text(image, '840x25+220+445', env)
            details.append(selected_inventory_name(text))
        assert len(set(details)) == len(details), 'Duplicate selected inventory headings'
        report.setdefault('inventory_inspections', []).append(dict(prefix=prefix, count=count, names=details))
        return count, details
    def hud_wallet(dimmed=False):
        native=shot('hud-wallet-source.png')
        value,bounds,raw=read_hud_wallet(native,env,dimmed=dimmed)
        report.setdefault('hud_wallet_ocr',[]).append(dict(value=value,bounds=bounds,raw=raw))
        return value
    def read_speed():
        drain_choices()
        value = read_toolbar_speed(shot('speed-source.png'), env)
        report.setdefault('speed_ocr_values', []).append(value)
        return value
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
        call(['xdotool','windowactivate','--sync',win])
        geometry=dict(line.split('=',1) for line in call(['xdotool','getwindowgeometry','--shell',win]).splitlines())
        assert geometry['WIDTH']=='1280' and geometry['HEIGHT']=='720', 'Forge OCR route requires1280x720English'
        wait('EXPEDITIONS');shot('startup.png');click(289,294)
        wait('Click a basic commander to begin')
        names=[r for r in lines() if normalized(r[0])=='stormsavant' and 150<r[2]<550];assert names
        _,x,y,r,b=min(names,key=lambda r:r[2]);click((x+r)//2,(y+b)//2)
        worker=threading.Thread(target=operate,daemon=True)
        end=time.monotonic()+90
        while time.monotonic()<end:
            if any('24:' in t.replace(' ','') for t,*_ in lines()):break
            key('1')
        else:raise RuntimeError('Battle countdown never advanced')
        print('stage: battle started; resetting actual toolbar to1x',flush=True)
        end=time.monotonic()+90
        while time.monotonic()<end:
            speeds={read_speed()}
            if speeds=={1}:
                time.sleep(1)
                if read_speed()==1:break
                continue
            if len(speeds)==1:click(1045,44)
            else:raise RuntimeError('Cannot read actual toolbar speed')
        else:raise RuntimeError('1x toolbar not verified')
        report['actual_speed']=1;shot('speed1.png');worker.start()
        print('stage: earn kills and wait for real3min unlock',flush=True)
        unlock_deadline=time.monotonic()+900
        last_clock=None; last_progress=time.monotonic()
        while time.monotonic()<unlock_deadline:
            native=shot('unlock-source.png')
            try:
                clock=screenshot_battle_seconds(native,env)
            except AssertionError as error:
                report.setdefault('unreadable_unlock_clock',[]).append(str(error))
                terminal(lines());time.sleep(3);continue
            report.setdefault('unlock_timer_reads',[]).append(clock)
            print('stage: real unlock countdown '+str(clock),flush=True)
            now=time.monotonic()
            if last_clock is None or clock<last_clock:last_progress=now
            last_clock=clock
            report.setdefault('clock_progress',[]).append(dict(stage='unlock',remaining=clock,stall_seconds=now-last_progress,wall=now))
            assert now-last_progress<180,'Actual simulation clock stalled for180seconds'
            (OUT/'progress.json').write_text(json.dumps(dict(stage='unlock',remaining=clock,stall_seconds=now-last_progress)))
            if clock<22*60:break
            time.sleep(3)
        else:raise RuntimeError('Real3minute unlock not observed')
        shot('unlocked.png')
        forge_deadline=time.monotonic()+3600
        for attempt in range(120):
            assert time.monotonic()<forge_deadline, "Finite earned forge wall budget exhausted"
            stop.set();worker.join(5);assert not worker.is_alive()
            drain_choices()
            actual=read_speed()
            if actual != 1:
                click(1045,44)
                assert read_speed() == 1, 'Actual reset speed did not reach1x'
            key('b')
            rows=lines();terminal(rows)
            assert has_gear_rows(rows), ('Expected unlocked market panel', rows)
            before=wallet();shot(f'market-{attempt:02}.png')
            offers=[]
            visit_end=time.monotonic()+120
            for index,left in enumerate(TILE_LEFTS):
                assert time.monotonic()<visit_end, 'Natural market inspection exceeded120seconds'
                offers.append(inspect_offer(index,f'market-{attempt:02}-inspect-{index}'))
            report.setdefault('market_inspections',[]).append(dict(attempt=attempt,wallet=before,offers=offers))
            offer=natural_material(offers,before,report['acquired_materials'])
            if offer is not None:
                name,price=offer['name'],offer['price'];left=TILE_LEFTS[offer['index']]
                checked=inspect_offer(offer['index'],'before-purchase',offer)
                report['selected_material_ocr']=checked
                assert hud_wallet()==before==wallet(), 'HUD/header wallet disagreement'
                report['pending_purchase']=dict(name=name,price=price,wallet_before=before,offer_ocr=offer)
                click(1184,506)
                after=hud_wallet()
                assert wallet()==after, 'After-buy shared header/HUD wallet disagreement'
                verify_deduction(before,after,offer)
                report['purchases'].append(dict(name=name,price=price,wallet_before=before,wallet_after=after,offer_ocr=offer))
                shot('purchase.png');close_market_panel();drain_choices()
                opened=wait_inventory_open(shot,env,key,time.monotonic()+90,
                    report.setdefault('inventory_open_attempts',[]),
                    f'material-{len(report["purchases"])}')
                if opened is None:
                    terminal(lines())
                    raise AssertionError('Terminal inventory observation no longer readable; craft proof remains pending')
                count,details=inspect_inventory(f'material-{len(report["purchases"])}')
                expected=[normalized(item) for item in [*report['acquired_materials'],name]]
                assert count==len(expected) and set(details)==set(expected), ('Purchased materials not owned',count,details,expected)
                shot(f'inventory-material-{len(report["purchases"])}.png')
                report['acquired_materials'].append(name)
                if set(report['acquired_materials'])==MATERIALS:
                    clicktext('Forge')
                    # Six recipe icons: second is Astral Prism. Verify actual
                    # selected heading/materials/action before spending inputs.
                    click(330,184)
                    ready=shot('ready-materials.png')
                    title=crop_text(ready,'840x25+220+394',env)
                    materials=crop_text(ready,'840x24+220+420',env)
                    action=crop_text(ready,'250x28+515+552',env)
                    verify_recipe_selection(title,materials,action)
                    assert inventory_count(lines())==2
                    report['ready_material_ocr']=dict(title=title,materials=materials,action=action)
                    before_forge=hud_wallet(dimmed=True)
                    clicktext('Forge equipment');time.sleep(.5)
                    forged_rows=lines();shot('forged.png')
                    assert find_text(forged_rows,'Already forged'), 'Craft action not consumed'
                    assert hud_wallet(dimmed=True)==before_forge,'Forge charged unexpected kill credit'
                    clicktext('Gear');count,details=inspect_inventory('crafted')
                    verify_crafted_inventory(count,details,report['acquired_materials'])
                    report['crafted_inventory_ocr']=dict(count=count,names=details)
                    shot('crafted-inventory.png');close_inventory()
                    report['inventory_verified']=True
                    report['crafting']='Astral Prism crafted from two same-run naturally purchased materials'
                    break
                close_inventory()
            else:
                close_market_panel()
            stop.clear();worker=threading.Thread(target=operate,daemon=True);worker.start();time.sleep(12)
        assert report.get('inventory_verified'), 'No genuine recipe completed within finite public stock route'
        assert report['purchases'],'No naturally affordable equipment found within120 public market requests'
        before=timer_seconds();time.sleep(2);after=timer_seconds()
        report['resume_countdown_before']=before;report['resume_countdown_after']=after
        assert after<before,'Actual battlefield countdown did not resume'
        print('stage: acquired equipment, inspected inventory/recipes, actual countdown resumed',flush=True)
        click(640,30);wait('Settings');clicktext('Quit');time.sleep(1)
        if app.poll() is None:clicktext('Quit game')
        report['quit_exit']=app.wait(30);assert report['quit_exit']==0
        report['final_progress']=read_progress(state/'campaign.save');assert report['final_progress']==initial
        report['initial_save_sha256']=initial_save_hash
        report['final_save_sha256']=hashlib.sha256((state/'campaign.save').read_bytes()).hexdigest()
        assert report['final_save_sha256']==initial_save_hash, 'Campaign save bytes changed'
        report['completed']=True
    finally:
        stop.set()
        if worker and worker.ident is not None:worker.join(5)
        (OUT/'result.json').write_text(json.dumps(report,indent=2))
        for p in reversed(ps):
            if p.poll() is None:p.terminate()
        for f in files:f.close()
if __name__=='__main__':main()
