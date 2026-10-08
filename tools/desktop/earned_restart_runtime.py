#!/usr/bin/env python3
"""Continue real earned campaign progress; purchase and inspect via public UI."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import time
from campaign_runtime import read_progress, strength_from_lines
from smoke_runtime import find_text, normalized, ocr_screenshot

GAME, OUT, STATE = Path('/game'), Path('/verify'), Path('/verify/state')


def main():
    assert not Path('/nix').exists() and not Path('/project').exists()
    assert not shutil.which('cargo') and not shutil.which('rustc')
    for item in json.loads((GAME / 'package-manifest.json').read_text())['files']:
        assert hashlib.sha256((GAME / item['file']).read_bytes()).hexdigest() == item['sha256']
    progress_path = STATE / 'campaign.save'
    assert STATE.is_dir() and progress_path.is_file(), 'Requires actual pre-existing earned state'
    before = read_progress(progress_path)
    assert before == dict(unlocked=2, currency=180, cleared='100000', upgrades=[0]*6)
    env = os.environ.copy()
    for key in ('BEVY_ASSET_ROOT', 'CARGO_MANIFEST_DIR', 'WAYLAND_DISPLAY', 'ROGUE_TUTORIAL_SAVE'):
        env.pop(key, None)
    env.update(DISPLAY=':94', WINIT_UNIX_BACKEND='x11', XDG_CONFIG_HOME=str(STATE/'config'),
               XDG_RUNTIME_DIR=str(STATE), ROGUE_CAMPAIGN_SAVE=str(progress_path),
               ROGUE_COMMANDER_LANG='en', ROGUE_COMMANDER_SEED='0xA11CE',
               ROGUE_AUDIO_SAVE=str(STATE/'audio'),
               VK_ICD_FILENAMES=str(next(Path('/usr/share/vulkan/icd.d').glob('lvp_icd*.json'))))
    processes, handles = [], []
    report = dict(scope='Continue actual 1x chapter-clear reward through public UI',
                  before=before, all_steps_verified=False)
    def start(args, name):
        handle = (OUT/name).open('w'); handles.append(handle)
        process = subprocess.Popen(args, env=env, cwd='/tmp', stdout=handle, stderr=subprocess.STDOUT)
        processes.append(process); return process
    def call(args):
        return subprocess.check_output(args, env=env, text=True).strip()
    def shot(win, name):
        path = OUT/name; call(['import', '-window', win, str(path)]); return path
    def lines(win):
        return ocr_screenshot(shot(win, 'current-ui.png'), env)
    def wait(app, predicate, description, timeout=120):
        deadline = time.monotonic()+timeout
        while time.monotonic()<deadline:
            assert app.poll() is None, 'Unexpected game exit'
            result = predicate()
            if result: return result
            time.sleep(.3)
        raise RuntimeError(f'Timed out: {description}')
    def text(win, app, label):
        return wait(app, lambda: find_text(lines(win), label), label)
    def click(win, x, y):
        call(['xdotool', 'mousemove', '--window', win, str(x), str(y)])
        call(['xdotool', 'click', '1']); time.sleep(.3)
    def click_text(win, app, label):
        _, left, top, right, bottom = text(win, app, label)
        click(win, (left+right)//2, (top+bottom)//2)
    def window(app):
        def visible():
            try: return call(['xdotool', 'search', '--onlyvisible', '--pid', str(app.pid)]).splitlines()[0]
            except (subprocess.CalledProcessError, IndexError): return None
        win = wait(app, visible, 'game window')
        call(['xdotool', 'windowactivate', '--sync', win]); return win
    def geometry(win):
        values = dict(line.split('=',1) for line in call(['xdotool','getwindowgeometry','--shell',win]).splitlines())
        assert values['WIDTH']=='1280' and values['HEIGHT']=='720'
    def quit_game(win, app):
        click(win, 640, 30); click_text(win, app, 'Quit')
        text(win, app, 'Quit game'); click_text(win, app, 'Quit game')
        assert app.wait(timeout=30)==0
    try:
        start(['Xvfb', ':94', '-screen', '0', '1600x900x24', '-nolisten', 'tcp', '-ac'], 'xvfb.log')
        time.sleep(1); start(['openbox'], 'wm.log'); time.sleep(1)
        app = start([str(GAME/'rogue_commander')], 'purchase.log'); win = window(app)
        text(win, app, 'EXPEDITIONS'); text(win, app, '180 LEGACY SHARDS')
        geometry(win); shot(win, 'earned-startup.png'); report['earned_startup_verified']=True
        click_text(win, app, 'Training'); text(win, app, 'LEGACY TRAINING')
        shot(win, 'training-before.png'); click(win, 290, 231)
        expected = dict(unlocked=2, currency=165, cleared='100000', upgrades=[1,0,0,0,0,0])
        wait(app, lambda: read_progress(progress_path)==expected, 'saved Strength purchase')
        text(win, app, '165 LEGACY SHARDS'); shot(win, 'training-purchased.png')
        report['after']=read_progress(progress_path); report['purchase_verified']=True
        quit_game(win, app); report['purchase_quit_exit_code']=0
        app = start([str(GAME/'rogue_commander')], 'restart.log'); win = window(app)
        text(win, app, 'EXPEDITIONS'); text(win, app, '165 LEGACY SHARDS')
        wait(app, lambda: any('embermarch' in normalized(line[0]) for line in lines(win)), 'Ember March menu')
        assert read_progress(progress_path)==expected
        geometry(win); shot(win, 'restart-earned-menu.png'); report['restart_verified']=True
        click(win, 289, 294); text(win, app, 'Click a basic commander to begin')
        captions = [line for line in lines(win) if normalized(line[0])=='stormsavant' and 150<line[2]<550]
        assert captions, 'Require actual portrait card caption'
        _, left, top, right, bottom = min(captions, key=lambda line: line[2])
        click(win, (left+right)//2, (top+bottom)//2)
        wait(app, lambda: any(re.search(r'24\s*:\s*\d{2}', line[0]) for line in lines(win)), 'actual battle countdown')
        assert any(re.fullmatch(r'1\s*[x×]', line[0].strip(), re.I) for line in lines(win)), 'Actual battle speed must be 1x'
        geometry(win); click(win, 178, 44); text(win, app, 'ATTRIBUTES')
        screen = shot(win, 'earned-strength-restart.png'); panel = ocr_screenshot(screen, env)
        assert any(re.search(r'Storm\s*Savant.*Level\s+1\b', line[0], re.I) for line in panel)
        actual, evidence = strength_from_lines(panel)
        report['attributes']=dict(hero='StormSavant', level=1, speed=1, strength=actual,
                                  expected_strength=22, strength_ocr=evidence, screenshot=screen.name)
        assert actual==22, f'Actual Strength {actual}, expected 20+2=22'
        report['earned_strength_applies_after_restart']=True
        call(['xdotool','key','Escape'])
        wait(app, lambda: find_text(lines(win),'ATTRIBUTES') is None, 'attributes closed', 30)
        report['attributes_closed_verified']=True
        quit_game(win, app); report['restart_quit_exit_code']=0
        assert read_progress(progress_path)==expected
        report['all_steps_verified']=True
        (OUT/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    finally:
        (OUT/'latest-report.json').write_text(json.dumps(report,indent=2)+'\n')
        for process in reversed(processes):
            if process.poll() is None:
                process.terminate()
                try: process.wait(timeout=10)
                except subprocess.TimeoutExpired: process.kill(); process.wait()
        for handle in handles: handle.close()

if __name__=='__main__': main()
