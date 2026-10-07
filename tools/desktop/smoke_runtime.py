#!/usr/bin/env python3
"""Exercise a packaged game inside the isolated runtime test image."""
import csv
import io
import re
import unicodedata
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

GAME = Path('/game')
OUT = Path('/verify')


def normalized(text):
    return ''.join(char.lower() for char in unicodedata.normalize('NFKC', text) if char.isalnum())

def ocr_lines(tsv):
    """Group real OCR words into visible lines, keeping their click bounds."""
    groups = {}
    for row in csv.DictReader(io.StringIO(tsv), delimiter='\t'):
        if row['level'] != '5' or not row['text'].strip():
            continue
        key = tuple(row[name] for name in ('page_num', 'block_num', 'par_num', 'line_num'))
        x, y, width, height = (int(row[name]) for name in ('left', 'top', 'width', 'height'))
        groups.setdefault(key, []).append((row['text'], x, y, x + width, y + height))
    result = []
    for words in groups.values():
        result.append((' '.join(word[0] for word in words),
                       min(word[1] for word in words), min(word[2] for word in words),
                       max(word[3] for word in words), max(word[4] for word in words)))
    return result

def find_text(lines, text):
    """Exact line match; choose the bottom action if title and button repeat."""
    matches = [line for line in lines if normalized(line[0]) == normalized(text)]
    return max(matches, key=lambda line: line[2]) if matches else None

def main():
    assert not Path('/nix').exists() and not Path('/project').exists()
    assert shutil.which('rustc') is None and shutil.which('cargo') is None
    for item in json.loads((GAME / 'package-manifest.json').read_text())['files']:
        path = GAME / item['file']
        assert path.is_file() and hashlib.sha256(path.read_bytes()).hexdigest() == item['sha256']
    OUT.mkdir(exist_ok=True)
    state = Path(tempfile.mkdtemp(prefix='rogue-package-'))
    env = os.environ.copy()
    env.update(DISPLAY=':91', WINIT_UNIX_BACKEND='x11', XDG_CONFIG_HOME=str(state / 'config'),
               XDG_RUNTIME_DIR=str(state), ROGUE_AUDIO_SAVE=str(state / 'audio'),
               VK_ICD_FILENAMES=str(next(Path('/usr/share/vulkan/icd.d').glob('lvp_icd*.json'))))
    for key in ['BEVY_ASSET_ROOT', 'CARGO_MANIFEST_DIR', 'WAYLAND_DISPLAY', 'ROGUE_COMMANDER_LANG', 'ROGUE_TUTORIAL_SAVE', 'ROGUE_CAMPAIGN_SAVE', 'ROGUE_COMMANDER_SEED']:
        env.pop(key, None)
    (state / 'audio').write_text('1 0 0 1\n')
    processes, handles = [], []
    def start(args, log):
        file = (OUT / log).open('w'); handles.append(file)
        process = subprocess.Popen(args, env=env, cwd='/tmp', stdout=file, stderr=subprocess.STDOUT)
        processes.append(process); return process
    def call(args):
        return subprocess.check_output(args, env=env, text=True).strip()
    def wait(predicate, process, seconds=120):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            if process.poll() is not None:
                raise RuntimeError('Game exited before the expected UI state; inspect game.log')
            try:
                result = predicate()
                if result: return result
            except subprocess.CalledProcessError:
                pass
            time.sleep(.5)
        raise RuntimeError('Timed out waiting for runtime UI')
    def window(process):
        result = wait(lambda: call(['xdotool', 'search', '--onlyvisible', '--pid', str(process.pid)]).splitlines()[0], process)
        call(['xdotool', 'windowactivate', '--sync', result])
        return result
    def click(win, x, y, delay=2):
        call(['xdotool', 'mousemove', '--window', win, str(x), str(y)]); call(['xdotool', 'click', '1']); time.sleep(delay)
    def screenshot(win, name):
        call(['import', '-window', win, str(OUT / name)])
    def visible_lines(win):
        screenshot(win, 'current-ui.png')
        return ocr_lines(call(['tesseract', str(OUT / 'current-ui.png'), 'stdout', '-l', 'eng+chi_sim', '--psm', '11', 'tsv']))
    def wait_text(win, process, text):
        return wait(lambda: find_text(visible_lines(win), text), process)
    def click_text(win, process, text):
        _, left, top, right, bottom = wait_text(win, process, text)
        click(win, (left + right) // 2, (top + bottom) // 2)
    def battle_running(win, process):
        # The production countdown must advance: a battlefield screenshot alone
        # could still be the paused guide/recruitment background.
        wait(lambda: any(re.search(r'24\s*:\s*\d{2}', line[0]) for line in visible_lines(win))
             and find_text(visible_lines(win), '1 / 4 · MOVE & DODGE') is None, process)
    def quit_game(win, process, name):
        click_text(win, process, '设置')
        click_text(win, process, '退出')
        wait_text(win, process, '退出游戏？')
        screenshot(win, name)
        click_text(win, process, '退出游戏')
        assert process.wait(timeout=30) == 0
    def geometry(win):
        return dict(line.split('=', 1) for line in call(['xdotool', 'getwindowgeometry', '--shell', win]).splitlines())
    try:
        start(['Xvfb', ':91', '-screen', '0', '1600x900x24', '-nolisten', 'tcp', '-ac'], 'xvfb.log'); time.sleep(1)
        start(['openbox'], 'wm.log'); time.sleep(1)
        app = start([str(GAME / 'rogue_commander')], 'game.log'); win = window(app)
        wait_text(win, app, 'EXPEDITIONS')
        screenshot(win, 'startup.png')
        click(win, 290, 230)
        wait_text(win, app, '1 / 4 · MOVE & DODGE')
        wait_text(win, app, 'Skip')
        screenshot(win, 'guide.png')
        call(['xdotool', 'key', 'Escape'])
        tutorial = state / 'config/rogue-commander/tutorial-dismissed'
        wait(lambda: tutorial.is_file() and tutorial.read_text().strip() == '1', app)
        wait_text(win, app, 'Click a basic Commander to begin')
        screenshot(win, 'recruit.png')
        click(win, 230, 340)
        battle_running(win, app)
        screenshot(win, 'battle.png')
        click_text(win, app, 'Settings')
        wait_text(win, app, 'INTERFACE LANGUAGE')
        click(win, 780, 300)
        assert (state / 'config/rogue-commander/language').read_text().strip() == 'zh-CN'
        call(['xdotool', 'key', 'F11'])
        wait(lambda: geometry(win)['WIDTH'] == '1600' and geometry(win)['HEIGHT'] == '900', app)
        time.sleep(2); screenshot(win, 'fullscreen-chinese.png')
        call(['xdotool', 'key', 'F11'])
        wait(lambda: geometry(win)['WIDTH'] == '1280' and geometry(win)['HEIGHT'] == '720', app)
        time.sleep(2); screenshot(win, 'settings-chinese.png')
        # Already inside settings. Derive quit/confirm centers from the actual
        # visible action labels, including the added guide row in the footer.
        click_text(win, app, '退出')
        wait_text(win, app, '退出游戏？')
        screenshot(win, 'quit-chinese.png')
        click_text(win, app, '退出游戏')
        assert app.wait(timeout=30) == 0
        app = start([str(GAME / 'rogue_commander')], 'restart.log'); win = window(app)
        wait_text(win, app, '章节远征')
        screenshot(win, 'restart-chinese.png')
        assert (state / 'config/rogue-commander/language').read_text().strip() == 'zh-CN'
        click(win, 290, 230)
        wait_text(win, app, '点击基础指挥官开始远征')
        assert find_text(visible_lines(win), '1 / 4 · 移动与闪避') is None
        assert tutorial.read_text().strip() == '1'
        screenshot(win, 'restart-recruit-chinese.png')
        click(win, 230, 340)
        battle_running(win, app)
        screenshot(win, 'restart-battle-chinese.png')
        quit_game(win, app, 'restart-quit-chinese.png')
        for log in ['game.log', 'restart.log']:
            text = (OUT / log).read_text()
            for error in ['Path not found', 'ICU4X data error', 'panicked', 'Caught rendering error']:
                assert error not in text, (log, error)
        result = {'package_hashes_verified': True, 'no_source': True, 'no_rust': True, 'no_nix': True,
                  'working_directory': '/tmp', 'chapter_recruit_battle': True, 'fullscreen_restored': True,
                  'language_saved_and_reloaded': True, 'normal_quit_twice': True,
                  'real_guide_after_readiness': True, 'tutorial_dismissal_saved': True,
                  'restart_skips_automatic_guide_and_runs_battle': True,
                  'scope': 'Ubuntu 24.04 / X11 / software Vulkan; no Steam client or hardware performance claim'}
        (OUT / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print('DESKTOP_RUNTIME_PASS ' + json.dumps(result), flush=True)
    finally:
        for process in reversed(processes):
            if process.poll() is None:
                process.terminate()
                try: process.wait(timeout=10)
                except subprocess.TimeoutExpired: process.kill(); process.wait()
        for file in handles: file.close()

if __name__ == '__main__':
    main()
