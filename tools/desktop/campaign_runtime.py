#!/usr/bin/env python3
"""Real packaged first chapter via public keyboard/UI; never sets game state."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import threading
from smoke_runtime import find_text, normalized, ocr_screenshot

GAME, OUT = Path('/game'), Path('/verify')


def read_progress(path):
    lines = path.read_text().splitlines()
    assert len(lines) == 6 and lines[0] == 'RCAMPAIGN1'
    checksum = 0xcbf29ce484222325
    for byte in ('\n'.join(lines[:5]) + '\n').encode():
        checksum = ((checksum ^ byte) * 0x100000001b3) & ((1 << 64) - 1)
    assert int(lines[5], 16) == checksum
    return dict(unlocked=int(lines[1]), currency=int(lines[2]), cleared=lines[3],
                upgrades=[int(value) for value in lines[4].split(',')])


def main():
    assert not Path('/nix').exists() and not Path('/project').exists()
    assert not shutil.which('cargo') and not shutil.which('rustc')
    for item in json.loads((GAME / 'package-manifest.json').read_text())['files']:
        assert hashlib.sha256((GAME / item['file']).read_bytes()).hexdigest() == item['sha256']
    OUT.mkdir(exist_ok=True)
    state = OUT / 'state'; state.mkdir(exist_ok=False)
    progress_path = state / 'campaign.save'
    env = os.environ.copy()
    for key in ('BEVY_ASSET_ROOT', 'CARGO_MANIFEST_DIR', 'WAYLAND_DISPLAY', 'ROGUE_TUTORIAL_SAVE'):
        env.pop(key, None)
    env.update(DISPLAY=':93', WINIT_UNIX_BACKEND='x11', XDG_CONFIG_HOME=str(state / 'config'),
               XDG_RUNTIME_DIR=str(state), ROGUE_CAMPAIGN_SAVE=str(progress_path),
               ROGUE_COMMANDER_LANG='en', ROGUE_COMMANDER_SEED='0xA11CE',
               ROGUE_AUDIO_SAVE=str(state / 'audio'),
               VK_ICD_FILENAMES=str(next(Path('/usr/share/vulkan/icd.d').glob('lvp_icd*.json'))))
    (state / 'audio').write_text('1 0 0 1\n')
    processes, logs = [], []
    def start(args, name):
        log = (OUT / name).open('w'); logs.append(log)
        process = subprocess.Popen(args, env=env, cwd='/tmp', stdout=log, stderr=subprocess.STDOUT)
        processes.append(process); return process
    def call(args):
        return subprocess.check_output(args, env=env, text=True).strip()
    def shot(win, name):
        path = OUT / name; call(['import', '-window', win, str(path)]); return path
    def lines(win):
        return ocr_screenshot(shot(win, 'current-ui.png'), env)
    def wait_text(win, app, text, timeout=120):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            assert app.poll() is None, 'Game exited before expected UI'
            found = find_text(lines(win), text)
            if found: return found
            time.sleep(.3)
        raise RuntimeError(f'Timed out waiting for {text}')
    def click_text(win, app, text):
        _, left, top, right, bottom = wait_text(win, app, text)
        call(['xdotool', 'mousemove', '--window', win, str((left+right)//2), str((top+bottom)//2)])
        call(['xdotool', 'click', '1']); time.sleep(.3)
    def window(app):
        deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            try:
                win = call(['xdotool', 'search', '--onlyvisible', '--pid', str(app.pid)]).splitlines()[0]
                call(['xdotool', 'windowactivate', '--sync', win]); return win
            except (subprocess.CalledProcessError, IndexError): time.sleep(.3)
        raise RuntimeError('No game window')
    def quit_game(win, app):
        call(['xdotool', 'mousemove', '--window', win, '640', '30']); call(['xdotool', 'click', '1'])
        click_text(win, app, 'Quit')
        time.sleep(1)
        if app.poll() is None: click_text(win, app, 'Quit game')
        assert app.wait(timeout=30) == 0
    stop_input = threading.Event()
    input_thread = None
    report = dict(scope='Packaged Ubuntu/X11 software Vulkan, public UI/keys, stationary greedy policy',
                  seed='0xA11CE', speed=32, completed_run=False, victory=False)
    try:
        start(['Xvfb', ':93', '-screen', '0', '1600x900x24', '-nolisten', 'tcp', '-ac'], 'xvfb.log')
        time.sleep(1); start(['openbox'], 'wm.log'); time.sleep(1)
        app = start([str(GAME / 'rogue_commander')], 'game.log'); win = window(app)
        click_text(win, app, 'BEGIN EXPEDITION')
        wait_text(win, app, 'Skip'); call(['xdotool', 'key', 'Escape'])
        wait_text(win, app, 'Click a basic commander to begin')
        names = [line for line in lines(win) if normalized(line[0]) == 'stormsavant' and 150 < line[2] < 550]
        assert names, 'Actual portrait card caption, not the duplicate detail heading'
        _, left, top, right, bottom = min(names, key=lambda line: line[2])
        call(['xdotool', 'mousemove', '--window', win, str((left+right)//2), str((top+bottom)//2)])
        call(['xdotool', 'click', '1'])
        deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            if any('24:' in text.replace(' ', '') for text, *_ in lines(win)): break
            call(['xdotool', 'key', '1']); time.sleep(.3)
        else: raise RuntimeError('Actual battle countdown did not advance after hero card click')
        for _ in range(5): call(['xdotool', 'key', 't']); time.sleep(.25)
        def operate():
            while not stop_input.is_set() and app.poll() is None:
                call(['xdotool', 'key', '--delay', '30', '1', 'g', 'f'])
                stop_input.wait(.2)
        input_thread = threading.Thread(target=operate, daemon=True)
        input_thread.start()
        speed_lines = lines(win)
        assert any('32' in text and ('x' in normalized(text) or '×' in text) for text, *_ in speed_lines), 'Actual speed must be32'
        shot(win, 'speed32.png')
        started, deadline = time.monotonic(), time.monotonic() + 900
        next_sample, sample = 0., 0
        while time.monotonic() < deadline:
            assert app.poll() is None, 'Unexpected game exit'
            if time.monotonic() >= next_sample:
                sample += 1
                screen = shot(win, f'run-{sample:03}.png')
                text_lines = ocr_screenshot(screen, env)
                title = 'CHAPTER CLEARED' if find_text(text_lines, 'CHAPTER CLEARED') else 'NO LIVES REMAIN' if find_text(text_lines, 'NO LIVES REMAIN') else None
                print(json.dumps({'sample':sample,'wall_seconds':round(time.monotonic()-started,1),'terminal':title}), flush=True)
                if title:
                    report.update(completed_run=True, victory=title=='CHAPTER CLEARED', terminal=title,
                                  wall_seconds=time.monotonic()-started, samples=sample)
                    shutil.copy2(screen, OUT / 'result.png'); break
                next_sample = time.monotonic() + 3
            time.sleep(.2)
        stop_input.set(); input_thread.join(timeout=5)
        assert not input_thread.is_alive(), 'Public input worker did not stop'
        assert report['completed_run'], 'No real terminal UI within budget'
        deadline = time.monotonic()+30
        while not progress_path.exists() and time.monotonic()<deadline: time.sleep(.2)
        before = read_progress(progress_path); report['earned_progress'] = before
        assert before['currency'] > 0
        assert (before['cleared'][0]=='1') == report['victory']
        if before['currency'] >= 15:
            geometry=dict(line.split('=',1) for line in call(['xdotool','getwindowgeometry','--shell',win]).splitlines())
            assert geometry['WIDTH']=='1280' and geometry['HEIGHT']=='720'
            # The inspected report footer places the training action here; require
            # the real training title after the click rather than trusting OCR aliases.
            call(['xdotool','mousemove','--window',win,str(714 if report['victory'] else 655),'620'])
            call(['xdotool','click','1'])
            wait_text(win, app, 'LEGACY TRAINING')
            # Inspected1280x720 training layout: first Strength upgrade button.
            shot(win,'training-before.png')
            call(['xdotool','mousemove','--window',win,'290','231']);call(['xdotool','click','1']);time.sleep(1)
            after=read_progress(progress_path)
            assert after['currency']==before['currency']-15 and sum(after['upgrades'])==1
            report['earned_upgrade']=after;shot(win,'training.png')
        quit_game(win,app)
        app=start([str(GAME/'rogue_commander')],'restart.log');win=window(app)
        wait_text(win,app,'EXPEDITIONS');shot(win,'restart.png')
        assert read_progress(progress_path)==report.get('earned_upgrade',before)
        report['saved_progress_survives_restart']=True
        quit_game(win,app);report['normal_quit_twice']=True
        (OUT/'result.json').write_text(json.dumps(report,indent=2)+'\n')
        assert report['victory'], 'Actual run lost: retain failure evidence, do not count as chapter completion'
    finally:
        stop_input.set()
        if input_thread is not None: input_thread.join(timeout=5)
        (OUT/'latest-report.json').write_text(json.dumps(report,indent=2)+'\n')
        for process in reversed(processes):
            if process.poll() is None:
                process.terminate()
                try:process.wait(timeout=10)
                except subprocess.TimeoutExpired:process.kill();process.wait()
        for log in logs:log.close()

if __name__=='__main__': main()
