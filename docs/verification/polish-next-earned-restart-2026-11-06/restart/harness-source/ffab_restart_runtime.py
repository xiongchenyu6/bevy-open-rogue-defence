#!/usr/bin/env python3
"""Read genuine completed C6 progress in a fresh profile through public UI; never grant or buy state."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import time
from campaign_runtime import read_progress, strength_from_lines
from campaign_equipment_runtime import inventory_count, wait_panel_open, screenshot_battle_seconds, close_visible_inventory, inventory_header_rows, inventory_open, public_panel_state
from forge_runtime import visible_hud_value
from smoke_runtime import find_text, normalized, ocr_screenshot, read_toolbar_speed, wait_basic_commander

GAME, OUT, STATE, PREVIOUS = map(Path, ('/game', '/verify', '/earned-state', '/previous-proof'))
EXPECTED_SAVE = 'ffab0a0d890e9ddd012de5516a7ef6de6a444a7b48108a1ae0fcecd43433cfbb'
EXPECTED_PROGRESS = dict(unlocked=6, currency=630, cleared='111111', upgrades=[7, 0, 0, 0, 0, 0])


def main():
    deadline = time.monotonic() + 180
    assert not Path('/nix').exists() and not Path('/project').exists()
    assert not shutil.which('cargo') and not shutil.which('rustc')
    OUT.mkdir(exist_ok=True)
    assert STATE.is_dir() and OUT.resolve() != STATE.resolve()
    version = json.loads((GAME/'version.json').read_text())
    assert version['git_commit'] == os.environ['PACKAGE_COMMIT']
    for item in json.loads((GAME/'package-manifest.json').read_text())['files']:
        assert hashlib.sha256((GAME/item['file']).read_bytes()).hexdigest() == item['sha256']
    assert hashlib.sha256((GAME/'rogue_commander').read_bytes()).hexdigest() == version['binary_sha256']
    previous = json.loads((PREVIOUS/'chapter6/result.json').read_text())
    assert previous['completed_run'] is True and previous['victory'] is True
    assert previous['normal_quit_twice'] is True
    assert previous['saved_progress_survives_restart'] is True
    assert previous['earned_upgrade'] == EXPECTED_PROGRESS
    assert previous['upgraded_attributes']['strength'] == 34.0
    assert previous['package_version']['git_commit'] == 'd4540c44df479041fe4ff4fdb818f8fb2f280f99'
    assert hashlib.sha256((PREVIOUS/'campaign-final.save').read_bytes()).hexdigest() == EXPECTED_SAVE
    assert sorted(item.name for item in STATE.iterdir()) == ['campaign.save'], 'Fresh profile must initially contain only exact earned save bytes'
    progress = STATE/'campaign.save'
    before = progress.read_bytes()
    assert hashlib.sha256(before).hexdigest() == EXPECTED_SAVE
    assert read_progress(progress) == EXPECTED_PROGRESS
    env = os.environ.copy()
    for key in ('BEVY_ASSET_ROOT', 'CARGO_MANIFEST_DIR', 'WAYLAND_DISPLAY', 'ROGUE_TUTORIAL_SAVE'):
        env.pop(key, None)
    env.update(DISPLAY=':96', WINIT_UNIX_BACKEND='x11', XDG_CONFIG_HOME=str(STATE/'config'),
        XDG_RUNTIME_DIR=str(STATE), ROGUE_CAMPAIGN_SAVE=str(progress), ROGUE_AUDIO_SAVE=str(STATE/'audio'),
        ROGUE_COMMANDER_LANG='en', ROGUE_COMMANDER_SEED='0xA11CE',
        VK_ICD_FILENAMES=str(next(Path('/usr/share/vulkan/icd.d').glob('lvp_icd*.json'))))
    processes, logs = [], []
    report = dict(scope='New-package compatibility with exact genuinely earned final C6 bytes; no purchase/training/resource grants',
        package_version=version, source_profile=str(STATE), previous_proof=str(PREVIOUS/'chapter6/result.json'),
        progress_before=EXPECTED_PROGRESS, save_before_sha256=EXPECTED_SAVE, previous_package_commit=previous['package_version']['git_commit'], requested_seed='0xA11CE',
        operation_budget_seconds=180, completed=False, result='INCOMPLETE')
    def call(args):
        remaining = deadline-time.monotonic()
        assert remaining > 0, 'Restart proof exceeded180second budget'
        return subprocess.check_output(args, env=env, text=True, timeout=remaining).strip()
    def start(args, name):
        log=(OUT/name).open('w'); logs.append(log)
        process=subprocess.Popen(args, env=env, cwd='/tmp', stdout=log, stderr=subprocess.STDOUT)
        processes.append(process); return process
    def shot(name):
        path=OUT/name; call(['import', '-window', win, str(path)]); return path
    def rows():return ocr_screenshot(shot('current-ui.png'), env)
    def wait(predicate, description):
        while time.monotonic() < deadline:
            assert app.poll() is None, 'Game exited before public restart proof'
            value=predicate()
            if value:return value
            time.sleep(.2)
        raise AssertionError('Restart proof timed out: '+description)
    def text(label):return wait(lambda:find_text(rows(), label), label)
    def click(x,y):
        call(['xdotool','mousemove','--window',win,str(x),str(y)])
        call(['xdotool','click','1']);time.sleep(.3)
    def click_text(label):
        _,x,y,right,bottom=text(label);click((x+right)//2,(y+bottom)//2)
    def key(value):
        assert value in ('Escape','i'), 'Unexpected choice in fresh restart proof; no additional grant selected'
        call(['xdotool','keydown',value]);time.sleep(.15)
        call(['xdotool','keyup',value]);time.sleep(.35)
    def clock(prefix):
        value=visible_hud_value(lambda:shot(prefix+'.png'),
            lambda image:screenshot_battle_seconds(image,env),lambda image:ocr_screenshot(image,env),
            key,deadline,env=env)
        assert value is not None, 'Actual terminal appeared before restart proof completed'
        return value
    try:
        start(['Xvfb',':96','-screen','0','1600x900x24','-nolisten','tcp','-ac'],'xvfb.log')
        time.sleep(1);start(['openbox'],'wm.log');time.sleep(1)
        app=start([str(GAME/'rogue_commander')],'restart-game.log')
        def visible():
            try:return call(['xdotool','search','--onlyvisible','--pid',str(app.pid)]).splitlines()[0]
            except (subprocess.CalledProcessError,IndexError):return None
        win=wait(visible,'window');call(['xdotool','windowactivate','--sync',win])
        geometry=dict(line.split('=',1) for line in call(['xdotool','getwindowgeometry','--shell',win]).splitlines())
        assert geometry['WIDTH']=='1280' and geometry['HEIGHT']=='720'
        text('EXPEDITIONS');report['menu_shards_ocr']=wait(lambda:next((row for row in rows() if normalized(row[0])=='630legacyshards'),None),'actual630legacyshards')
        menu=shot('restart-menu.png');assert progress.read_bytes()==before
        report['six_cleared_public_ocr']=[]
        # Native1280 menu statuses: x289/639/989, y294/509, from actual committed menu.
        for index,(x,y) in enumerate(( (x,y) for y in (294,509) for x in (289,639,989) ),1):
            crop=OUT/f'restart-cleared-{index:02}.png'
            call(['convert',str(menu),'-crop',f'140x32+{x-70}+{y-16}','+repage','-resize','300%',str(crop)])
            evidence=ocr_screenshot(crop,env)
            assert any(normalized(row[0])=='cleared' for row in evidence), ('Actual chapter status not CLEARED',index,evidence)
            report['six_cleared_public_ocr'].append(dict(chapter=index,screenshot=crop.name,rows=evidence))
        click(289,294)
        wait_basic_commander(shot,env,key,deadline,report.setdefault('startup_observations',[]))
        captions=wait(lambda:[row for row in rows() if normalized(row[0])=='stormsavant'
                              and 150<row[2]<550],'actual Storm Savant portrait caption')
        caption=min(captions,key=lambda row:row[2]);report['selected_caption_ocr']=caption
        _,x,y,right,bottom=caption;shot('restart-commander.png')
        target=((x+right)//2,(y+bottom)//2)
        call(['xdotool','mousemove','--window',win,str(target[0]),str(target[1])])
        request=dict(target=target,caption_ocr=caption,down_requested_monotonic=time.monotonic(),down_requested_wall=time.time())
        report['commander_click']=request
        try:
            call(['xdotool','mousedown','1'])
            request['down_returned_monotonic']=time.monotonic()
            time.sleep(.15)
        finally:
            request['up_requested_monotonic']=time.monotonic()
            subprocess.run(['xdotool','mouseup','1'],env=env,capture_output=True,check=True,timeout=5)
            request['up_returned_monotonic']=time.monotonic()
            request['up_returned_wall']=time.time()
            request['held_after_down_return_seconds']=request['up_returned_monotonic']-request.get('down_returned_monotonic',request['down_requested_monotonic'])
        # One request only. Observe actual chooser/loading settlement; no click
        # retry, portrait substitution, keyboard fallback or guessed clock.
        settlement=report.setdefault('commander_settlement',[])
        while time.monotonic()<deadline:
            image=shot(f'restart-settle-{len(settlement):02}.png')
            observed=ocr_screenshot(image,env)
            chooser=find_text(observed,'Click a basic commander to begin')
            loading=find_text(observed,'Preparing expedition')
            state=public_panel_state(observed,image,env)
            settlement.append(dict(screenshot=image.name,monotonic=time.monotonic(),state=state,chooser=chooser,loading=loading,rows=observed))
            assert state!='terminal', 'Actual terminal before basic battle settlement'
            if not chooser and not loading and state=='battle':break
            assert state!='choice' or chooser, 'Unexpected extra selection; no reward chosen'
            time.sleep(.2)
        else:raise AssertionError('Single observed Storm footer click did not settle into battle within original deadline')
        clock('restart-battle');assert read_toolbar_speed(shot('restart-speed.png'),env)==1
        click(178,44);text('ATTRIBUTES')
        image=shot('restart-strength.png');panel=ocr_screenshot(image,env)
        assert any(re.search(r'Storm\s*Savant.*Level\s+1\b',row[0],re.I) for row in panel)
        strength,evidence=strength_from_lines(panel)
        assert strength==34.0, ('Expected actual baseline20+earned Strength7×2',strength)
        report['attributes']=dict(hero='StormSavant',level=1,speed=1,strength=strength,strength_ocr=evidence)
        key('Escape');wait(lambda:find_text(rows(),'ATTRIBUTES') is None,'attributes closed')
        opened=wait_panel_open(shot,env,key,deadline,report.setdefault('panel_open_attempts',[]),
                               'restart-inventory','inventory')
        assert opened is not None, 'Actual terminal before empty inventory proof'
        image,inventory=opened
        assert inventory_count(inventory)==0, ('Per-run equipment should be empty in the fresh run',inventory)
        report['equipment_count']=0;report['empty_inventory_ocr']=inventory
        shutil.copy2(image,OUT/'restart-empty-inventory.png')
        assert any('noequipmentyet' in normalized(row[0]) for row in inventory)
        close_observations=report.setdefault('inventory_close_observations',[])
        def close_rows():
            image=shot(f'restart-close-{len(close_observations):03}.png')
            result=inventory_header_rows(image,env)
            close_observations.append(dict(screenshot=image.name,rows=result))
            return result
        def close_down(x,y):
            call(['xdotool','mousemove','--window',win,str(x),str(y)])
            call(['xdotool','mousedown','1'])
        close_visible_inventory(close_rows,close_down,
            lambda:call(['xdotool','mouseup','1']),min(deadline,time.monotonic()+20),close_observations)
        closed=rows()
        assert not inventory_open(closed) and not find_text(closed,'Close'),'Equipment did not close'
        first=clock('restart-resume-before');time.sleep(2);second=clock('restart-resume-after')
        assert second<first, ('Actual battle countdown failed to resume',first,second)
        report['resume_countdown']=dict(before=first,after=second)
        click(640,30);click_text('Quit');time.sleep(.5)
        if app.poll() is None:click_text('Quit game')
        report['quit_exit']=app.wait(timeout=max(.1,min(30,deadline-time.monotonic())))
        assert report['quit_exit']==0
        assert progress.read_bytes()==before and read_progress(progress)==EXPECTED_PROGRESS
        report['save_after_sha256']=hashlib.sha256(progress.read_bytes()).hexdigest()
        assert report['save_after_sha256']==EXPECTED_SAVE
        report['completed']=True
        report['result']='PASS'
        print(json.dumps(dict(result='PASS',equipment_count=0,strength=34,quit_exit=0,
                              unchanged_campaign_sha256=EXPECTED_SAVE)),flush=True)
    finally:
        (OUT/'result.json').write_text(json.dumps(report,indent=2)+'\n')
        for process in reversed(processes):
            if process.poll() is None:
                process.terminate()
                try:process.wait(timeout=10)
                except subprocess.TimeoutExpired:process.kill();process.wait()
        for log in logs:log.close()


if __name__=='__main__':main()
