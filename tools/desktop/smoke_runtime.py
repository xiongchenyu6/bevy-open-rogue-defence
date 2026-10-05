#!/usr/bin/env python3
"""Exercise a packaged game inside the isolated runtime test image."""
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
    for key in ['BEVY_ASSET_ROOT', 'CARGO_MANIFEST_DIR', 'WAYLAND_DISPLAY', 'ROGUE_COMMANDER_LANG']:
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
        call(['xdotool', 'windowactivate', '--sync', result]); time.sleep(20)
        return result
    def click(win, x, y, delay=2):
        call(['xdotool', 'mousemove', '--window', win, str(x), str(y)]); call(['xdotool', 'click', '1']); time.sleep(delay)
    def screenshot(win, name):
        call(['import', '-window', win, str(OUT / name)])
    def geometry(win):
        return dict(line.split('=', 1) for line in call(['xdotool', 'getwindowgeometry', '--shell', win]).splitlines())
    try:
        start(['Xvfb', ':91', '-screen', '0', '1600x900x24', '-nolisten', 'tcp', '-ac'], 'xvfb.log'); time.sleep(1)
        start(['openbox'], 'wm.log'); time.sleep(1)
        app = start([str(GAME / 'rogue_commander')], 'game.log'); win = window(app)
        screenshot(win, 'startup.png')
        click(win, 290, 230, 3); screenshot(win, 'recruit.png')
        click(win, 230, 340, 10); screenshot(win, 'battle.png')
        click(win, 640, 30); click(win, 780, 300)
        assert (state / 'config/rogue-commander/language').read_text().strip() == 'zh-CN'
        call(['xdotool', 'key', 'F11'])
        wait(lambda: geometry(win)['WIDTH'] == '1600' and geometry(win)['HEIGHT'] == '900', app)
        time.sleep(2); screenshot(win, 'fullscreen-chinese.png')
        call(['xdotool', 'key', 'F11'])
        wait(lambda: geometry(win)['WIDTH'] == '1280' and geometry(win)['HEIGHT'] == '720', app)
        time.sleep(2); screenshot(win, 'settings-chinese.png')
        click(win, 725, 538); screenshot(win, 'quit-chinese.png'); click(win, 760, 400)
        assert app.wait(timeout=30) == 0
        app = start([str(GAME / 'rogue_commander')], 'restart.log'); win = window(app)
        screenshot(win, 'restart-chinese.png')
        assert (state / 'config/rogue-commander/language').read_text().strip() == 'zh-CN'
        click(win, 640, 30); click(win, 725, 538); click(win, 760, 400)
        assert app.wait(timeout=30) == 0
        for log in ['game.log', 'restart.log']:
            text = (OUT / log).read_text()
            for error in ['Path not found', 'ICU4X data error', 'panicked', 'Caught rendering error']:
                assert error not in text, (log, error)
        result = {'package_hashes_verified': True, 'no_source': True, 'no_rust': True, 'no_nix': True,
                  'working_directory': '/tmp', 'chapter_recruit_battle': True, 'fullscreen_restored': True,
                  'language_saved_and_reloaded': True, 'normal_quit_twice': True,
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
