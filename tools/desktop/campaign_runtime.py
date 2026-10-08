#!/usr/bin/env python3
"""Real packaged selected chapter via public keyboard/UI; never sets game state."""
import hashlib
import json
import os
import re
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


def strength_from_lines(text_lines):
    """Only accept the Strength row's first-column value, never other stats."""
    values, evidence = set(), []
    for label in text_lines:
        text, left, top, right, bottom = label
        if not re.match(r'^Strength\b', text, re.I):
            continue
        inline = re.match(r'^Strength\s+(\d+(?:\.\d+)?)(?:\s|$)', text, re.I)
        if inline:
            values.add(float(inline[1])); evidence.append(label)
        elif normalized(text) == 'strength':
            for candidate in text_lines:
                value, x, y, _, lower = candidate
                if (re.fullmatch(r'\d+(?:\.\d+)?', value.strip())
                        and right < x < left + 285
                        and abs((y + lower) / 2 - (top + bottom) / 2) <= 8):
                    values.add(float(value)); evidence.extend((label, candidate))
    assert len(values) == 1, f'Cannot identify a unique Strength-row value: {evidence}'
    return values.pop(), list(dict.fromkeys(evidence))


def seed_from_text(text):
    match = re.fullmatch(r'SEED(0x[0-9a-f]{16})', ''.join(text.split()), re.I)
    assert match, f'Expected complete16-digit result seed, got {text!r}'
    return match[1].lower()


def result_seed_crop(text_lines):
    seed_lines = [line for line in text_lines if re.match(r'^SEED\b', line[0], re.I)]
    assert seed_lines, 'Result must expose the SEED row for crop verification'
    seed_top = max(0, min(line[2] for line in seed_lines) - 6)
    seed_height = max(line[4] for line in seed_lines) - seed_top + 6
    return f'480x{seed_height}+400+{seed_top}'


def strength_purchase(progress):
    """Expected public Strength purchase under campaign_progress.rs's pricing."""
    level = progress['upgrades'][0]
    assert level < 10, 'Strength already at maximum upgrade level'
    cost = 15 + 10 * level + 5 * level * level
    expected = dict(progress, currency=progress['currency'] - cost,
                    upgrades=[level + 1, *progress['upgrades'][1:]])
    return cost, expected


def main():
    assert not Path('/nix').exists() and not Path('/project').exists()
    assert not shutil.which('cargo') and not shutil.which('rustc')
    for item in json.loads((GAME / 'package-manifest.json').read_text())['files']:
        assert hashlib.sha256((GAME / item['file']).read_bytes()).hexdigest() == item['sha256']
    OUT.mkdir(exist_ok=True)
    stage = int(os.environ.get('CAMPAIGN_STAGE', '0'))
    assert 0 <= stage < 6, 'CAMPAIGN_STAGE must be 0..5'
    continuation = os.environ.get('CAMPAIGN_CONTINUE', '0')
    assert continuation in ('0', '1'), 'CAMPAIGN_CONTINUE must be 0 or 1'
    state = OUT / 'state'
    progress_path = state / 'campaign.save'
    if continuation == '1':
        assert state.is_dir() and progress_path.is_file(), 'Continue requires actual existing state'
        initial = read_progress(progress_path)
        assert stage < initial['unlocked'] and all(value == '1' for value in initial['cleared'][:stage])
    else:
        assert stage == 0, 'Fresh campaign can only begin at stage 0'
        state.mkdir(exist_ok=False)
        initial = dict(unlocked=1, currency=0, cleared='000000', upgrades=[0]*6)
    speed = int(os.environ.get('CAMPAIGN_SPEED', '32'))
    assert speed in (1, 2, 4, 8, 16, 32), 'CAMPAIGN_SPEED must be 1,2,4,8,16,32'
    requested_seed = os.environ.get('CAMPAIGN_SEED', '0xA11CE')
    assert 0 <= int(requested_seed, 0) < (1 << 64), 'CAMPAIGN_SEED must fit u64'
    env = os.environ.copy()
    for key in ('BEVY_ASSET_ROOT', 'CARGO_MANIFEST_DIR', 'WAYLAND_DISPLAY', 'ROGUE_TUTORIAL_SAVE'):
        env.pop(key, None)
    env.update(DISPLAY=':93', WINIT_UNIX_BACKEND='x11', XDG_CONFIG_HOME=str(state / 'config'),
               XDG_RUNTIME_DIR=str(state), ROGUE_CAMPAIGN_SAVE=str(progress_path),
               ROGUE_COMMANDER_LANG='en', ROGUE_COMMANDER_SEED=requested_seed,
               ROGUE_AUDIO_SAVE=str(state / 'audio'),
               VK_ICD_FILENAMES=str(next(Path('/usr/share/vulkan/icd.d').glob('lvp_icd*.json'))))
    if continuation == '0': (state / 'audio').write_text('1 0 0 1\n')
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
    def start_chapter(win):
        geometry = dict(line.split('=', 1) for line in call(
            ['xdotool', 'getwindowgeometry', '--shell', win]).splitlines())
        assert geometry['WIDTH'] == '1280' and geometry['HEIGHT'] == '720'
        # Multiple cards share BEGIN EXPEDITION; use the inspected chapter grid.
        call(['xdotool', 'mousemove', '--window', win, str((289, 640, 990)[stage % 3]), str(294 if stage < 3 else 509)])
        call(['xdotool', 'click', '1'])
    def select_storm(win, app):
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
    def record_strength(win, app, name, expected):
        geometry = dict(line.split('=', 1) for line in call(
            ['xdotool', 'getwindowgeometry', '--shell', win]).splitlines())
        assert geometry['WIDTH'] == '1280' and geometry['HEIGHT'] == '720'
        assert any(re.fullmatch(r'1\s*[x×]', text.strip(), re.I)
                   for text, *_ in lines(win)), 'Sample attributes only after confirming actual 1x'
        # Screenshot-confirmed public portrait rectangle: (156,22,44,44).
        call(['xdotool', 'mousemove', '--window', win, '178', '44'])
        call(['xdotool', 'click', '1'])
        wait_text(win, app, 'ATTRIBUTES')
        screen = shot(win, name)
        text_lines = ocr_screenshot(screen, env)
        assert any(re.search(r'Storm\s*Savant.*Level\s+1\b', text, re.I)
                   for text, *_ in text_lines), 'Attributes must show StormSavant at Level 1'
        actual, evidence = strength_from_lines(text_lines)
        record = dict(hero='StormSavant', level=1, speed=1, strength=actual,
                      expected=expected, screenshot=name, strength_ocr=evidence)
        assert actual == expected, f'Actual Strength {actual}, expected {expected}'
        call(['xdotool', 'key', 'Escape'])
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            if find_text(lines(win), 'ATTRIBUTES') is None:
                break
            time.sleep(.3)
        else:
            raise RuntimeError('Attributes did not close after Escape')
        return record
    stop_input = threading.Event()
    input_thread = None
    report = dict(scope='Packaged Ubuntu/X11 software Vulkan, public UI/keys, stationary greedy policy',
                  stage=stage, continued=continuation == '1', initial_progress=initial,
                  requested_seed=requested_seed, requested_speed=speed,
                  strategy='Stationary greedy public keys: 1, g, f', completed_run=False, victory=False)
    try:
        start(['Xvfb', ':93', '-screen', '0', '1600x900x24', '-nolisten', 'tcp', '-ac'], 'xvfb.log')
        time.sleep(1); start(['openbox'], 'wm.log'); time.sleep(1)
        app = start([str(GAME / 'rogue_commander')], 'game.log'); win = window(app)
        wait_text(win, app, 'EXPEDITIONS')
        if continuation == '1': assert read_progress(progress_path) == initial
        start_chapter(win)
        if continuation == '0':
            wait_text(win, app, 'Skip'); call(['xdotool', 'key', 'Escape'])
        wait_text(win, app, 'Click a basic commander to begin')
        select_storm(win, app)
        report['base_attributes'] = record_strength(win, app, 'base-strength.png', 20.0 + 2 * initial['upgrades'][0])
        for _ in range(speed.bit_length() - 1): call(['xdotool', 'key', 't']); time.sleep(.25)
        def operate():
            while not stop_input.is_set() and app.poll() is None:
                call(['xdotool', 'key', '--delay', '30', '1', 'g', 'f'])
                stop_input.wait(.2)
        input_thread = threading.Thread(target=operate, daemon=True)
        input_thread.start()
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            speed_lines = lines(win)
            visible = {int(match[1]) for text, x, y, right, bottom in speed_lines
                       if 1000 <= x <= 1080 and y < 90
                       and (match := re.fullmatch(r'(1|2|4|8|16|32)\s*[x×]', text.strip(), re.I))}
            assert len(visible) <= 1, f'Ambiguous toolbar speed: {visible}'
            if visible == {speed}:
                break
            if visible:
                current = visible.pop()
                # Correct a missed input through the real +/- toolbar button.
                call(['xdotool', 'mousemove', '--window', win, str(995 if current < speed else 943), '44'])
                call(['xdotool', 'mousedown', '1']); time.sleep(.15)
                call(['xdotool', 'mouseup', '1']); time.sleep(.3)
        else:
            raise RuntimeError(f'Actual toolbar speed did not reach {speed}x')
        report['actual_speed'] = speed
        shot(win, f'speed{speed}.png')
        timeout = max(900, 1500 / speed + 300)
        report['battle_timeout_seconds'] = timeout
        started, deadline = time.monotonic(), time.monotonic() + timeout
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
                    seed_row = OUT / 'seed-row.png'
                    call(['convert', str(screen), '-crop', result_seed_crop(text_lines), '-resize', '400%', str(seed_row)])
                    seed_text = call(['tesseract', str(seed_row), 'stdout', '-l', 'eng', '--psm', '7'])
                    report['actual_run_seed'] = seed_from_text(seed_text)
                    report['actual_seed_ocr'] = seed_text
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
        assert (before['cleared'][stage] == '1') == (report['victory'] or initial['cleared'][stage] == '1')
        cost, expected_purchase = strength_purchase(before)
        report['strength_upgrade_cost'] = cost
        if before['currency'] >= cost:
            geometry=dict(line.split('=',1) for line in call(['xdotool','getwindowgeometry','--shell',win]).splitlines())
            assert geometry['WIDTH']=='1280' and geometry['HEIGHT']=='720'
            # The inspected report footer places the training action here; require
            # the real training title after the click rather than trusting OCR aliases.
            call(['xdotool','mousemove','--window',win,str(714 if report['victory'] and stage < 5 else 655),'620'])
            call(['xdotool','click','1'])
            wait_text(win, app, 'LEGACY TRAINING')
            # Inspected1280x720 training layout: first Strength upgrade button.
            shot(win,'training-before.png')
            call(['xdotool','mousemove','--window',win,'290','231']);call(['xdotool','click','1']);time.sleep(1)
            after=read_progress(progress_path)
            assert after == expected_purchase, 'Actual saved purchase differs from expected cost/level'
            report['earned_upgrade']=after;shot(win,'training.png')
        quit_game(win,app)
        app=start([str(GAME/'rogue_commander')],'restart.log');win=window(app)
        wait_text(win,app,'EXPEDITIONS');shot(win,'restart.png')
        assert read_progress(progress_path)==report.get('earned_upgrade',before)
        report['saved_progress_survives_restart']=True
        assert 'earned_upgrade' in report, 'Earned shards insufficient to purchase Strength; effect remains unverified'
        # Reopen the configured chapter after restarting with its real purchased upgrade.
        start_chapter(win)
        wait_text(win, app, 'Click a basic commander to begin')
        select_storm(win, app)
        report['upgraded_attributes'] = record_strength(win, app, 'earned-strength-restart.png', 20.0 + 2 * report['earned_upgrade']['upgrades'][0])
        report['earned_strength_applies_after_restart'] = True
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
