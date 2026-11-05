#!/usr/bin/env python3
"""Restart the exact earned forge profile through public UI; never copy or grant state."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import time
from campaign_runtime import read_progress, strength_from_lines
from campaign_equipment_runtime import inventory_count, wait_panel_open, screenshot_battle_seconds
from forge_runtime import visible_hud_value
from smoke_runtime import find_text, normalized, ocr_screenshot, read_toolbar_speed

GAME, OUT, STATE, PREVIOUS = map(Path, ('/game', '/verify', '/earned-state', '/previous-proof'))
EXPECTED_COMMIT = '040ce5c1084634f3e8d886b65d1f5c2c467997af'
EXPECTED_SAVE = 'da5bf0a9ca3f4032a265425f9989bda292d8c0c2622848157df6d2c0223ba825'
EXPECTED_PROGRESS = dict(unlocked=5, currency=575, cleared='111100', upgrades=[5, 0, 0, 0, 0, 0])


def main():
    deadline = time.monotonic() + 180
    assert not Path('/nix').exists() and not Path('/project').exists()
    assert not shutil.which('cargo') and not shutil.which('rustc')
    OUT.mkdir(exist_ok=True)
    assert STATE.is_dir() and OUT.resolve() != STATE.resolve()
    version = json.loads((GAME/'version.json').read_text())
    assert version['git_commit'] == EXPECTED_COMMIT == os.environ['PACKAGE_COMMIT']
    for item in json.loads((GAME/'package-manifest.json').read_text())['files']:
        assert hashlib.sha256((GAME/item['file']).read_bytes()).hexdigest() == item['sha256']
    assert hashlib.sha256((GAME/'rogue_commander').read_bytes()).hexdigest() == version['binary_sha256']
    previous = json.loads((PREVIOUS/'result.json').read_text())
    assert previous['completed'] is True and previous['quit_exit'] == 0
    assert previous['package_version'] == version
    assert previous['crafted_inventory_ocr'] == dict(count=1, names=['astralprism'])
    assert previous['initial_progress'] == previous['final_progress'] == EXPECTED_PROGRESS
    assert previous['initial_save_sha256'] == previous['final_save_sha256'] == EXPECTED_SAVE
    progress = STATE/'campaign.save'
    before = progress.read_bytes()
    assert hashlib.sha256(before).hexdigest() == EXPECTED_SAVE
    assert read_progress(progress) == EXPECTED_PROGRESS
    assert (STATE/'audio').is_file() and (STATE/'config').is_dir()
    env = os.environ.copy()
    for key in ('BEVY_ASSET_ROOT', 'CARGO_MANIFEST_DIR', 'WAYLAND_DISPLAY', 'ROGUE_TUTORIAL_SAVE'):
        env.pop(key, None)
    env.update(DISPLAY=':96', WINIT_UNIX_BACKEND='x11', XDG_CONFIG_HOME=str(STATE/'config'),
        XDG_RUNTIME_DIR=str(STATE), ROGUE_CAMPAIGN_SAVE=str(progress), ROGUE_AUDIO_SAVE=str(STATE/'audio'),
        ROGUE_COMMANDER_LANG='en', ROGUE_COMMANDER_SEED='0xA11CE',
        VK_ICD_FILENAMES=str(next(Path('/usr/share/vulkan/icd.d').glob('lvp_icd*.json'))))
    processes, logs = [], []
    report = dict(scope='Same earned profile restart; no purchase, training, save copy or resource grant',
        package_version=version, source_profile=str(STATE), previous_proof=str(PREVIOUS/'result.json'),
        progress_before=EXPECTED_PROGRESS, save_before_sha256=EXPECTED_SAVE,
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
        text('EXPEDITIONS');report['menu_shards_ocr']=text('575 LEGACY SHARDS')
        shot('restart-menu.png');assert progress.read_bytes()==before
        click(289,294);text('Click a basic commander to begin')
        captions=wait(lambda:[row for row in rows() if normalized(row[0])=='stormsavant'
                              and 150<row[2]<550],'actual Storm Savant portrait caption')
        caption=min(captions,key=lambda row:row[2]);report['selected_caption_ocr']=caption
        _,x,y,right,bottom=caption;shot('restart-commander.png');click((x+right)//2,(y+bottom)//2)
        clock('restart-battle');assert read_toolbar_speed(shot('restart-speed.png'),env)==1
        click(178,44);text('ATTRIBUTES')
        image=shot('restart-strength.png');panel=ocr_screenshot(image,env)
        assert any(re.search(r'Storm\s*Savant.*Level\s+1\b',row[0],re.I) for row in panel)
        strength,evidence=strength_from_lines(panel)
        assert strength==30.0, ('Expected actual baseline20+earned Strength5×2',strength)
        report['attributes']=dict(hero='StormSavant',level=1,speed=1,strength=strength,strength_ocr=evidence)
        key('Escape');wait(lambda:find_text(rows(),'ATTRIBUTES') is None,'attributes closed')
        opened=wait_panel_open(shot,env,key,deadline,report.setdefault('panel_open_attempts',[]),
                               'restart-inventory','inventory')
        assert opened is not None, 'Actual terminal before empty inventory proof'
        image,inventory=opened
        assert inventory_count(inventory)==0, ('Per-run Astral Prism persisted unexpectedly',inventory)
        report['equipment_count']=0;report['empty_inventory_ocr']=inventory
        shutil.copy2(image,OUT/'restart-empty-inventory.png')
        assert any('noequipmentyet' in normalized(row[0]) for row in inventory)
        click_text('Close')
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
        print(json.dumps(dict(result='PASS',equipment_count=0,strength=30,quit_exit=0,
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
