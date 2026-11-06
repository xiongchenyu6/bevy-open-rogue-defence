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
    for row in csv.DictReader(io.StringIO(tsv), delimiter='\t', quoting=csv.QUOTE_NONE):
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

def ocr_screenshot(path, env=None):
    """Read both contrast polarities without moving any screenshot pixels.

    The guide's light labels on dark buttons need inverted grayscale, while
    other widget labels can be clearer in the original RGB screenshot.
    """
    path = Path(path)
    with tempfile.TemporaryDirectory(prefix='rogue-smoke-ocr-') as directory:
        inverted = Path(directory) / 'inverted.png'
        subprocess.run(['convert', str(path), '-colorspace', 'Gray', '-negate', str(inverted)],
                       env=env, check=True, capture_output=True)
        lines = []
        for image in (path, inverted):
            # Sparse layout reads separate buttons; block layout preserves the
            # Chinese chapter heading that sparse segmentation splits apart.
            for mode in ('11', '6'):
                tsv = subprocess.check_output(
                    ['tesseract', str(image), 'stdout', '-l', 'eng+chi_sim', '--psm', mode, 'tsv'],
                    env=env, text=True, stderr=subprocess.PIPE)
                lines.extend(ocr_lines(tsv))
    # Matching results from both passes need only one click rectangle.
    return list(dict.fromkeys(lines))

def wallet_text(results):
    nonempty = {text.strip() for text in results if text.strip()}
    if len(nonempty) != 1 or not all(re.fullmatch(r'\d+', text) for text in nonempty):
        raise ValueError(f'Unreadable or conflicting HUD wallet OCR: {results!r}')
    return nonempty.pop()


def read_hud_wallet(path, env=None, dimmed=False):
    """Locate the last gold-colored resource number in the 1280x720 HUD.

    Earlier resource widths change with gold/wood totals; a fixed x crop can
    silently lose the leading digit. Fail if no number is found.
    """
    raw = subprocess.check_output(
        ['convert', str(path), '-crop', '300x14+20+166', '-depth', '8', 'rgb:-'], env=env)
    if len(raw) != 300 * 14 * 3:
        raise ValueError('Unexpected HUD image geometry')
    groups = []
    # Equipment covers x>=200; read only the exposed HUD, allowing its dimmer.
    for x in range(180 if dimmed else 300):
        count = 0
        for y in range(14):
            r, g, b = raw[(y * 300 + x) * 3:(y * 300 + x) * 3 + 3]
            if dimmed:
                count += r > 80 and g > 70 and r - g > 4 and g - b > 12 and b < 195
            else:
                count += r > 175 and g > 150 and r - g > 10 and g - b > 25 and b < 195
        if count >= 2:
            if not groups or x - groups[-1][-1] > 7:
                groups.append([])
            groups[-1].append(x)
    if not groups:
        raise ValueError('No resource digits found in HUD')
    left, right = groups[-1][0] + 20, groups[-1][-1] + 20
    with tempfile.TemporaryDirectory(prefix='rogue-wallet-ocr-') as directory:
        crop = Path(directory) / 'digits.png'
        subprocess.run(['convert', str(path), '-crop', f'{right-left+5}x18+{left-2}+163',
                        '-resize', '300%', '-colorspace', 'Gray', '-negate',
                        '-threshold', '70%' if dimmed else '35%',
                        '-bordercolor', 'white', '-border', '2', str(crop)],
                       env=env, check=True, capture_output=True)
        # Measured 300% glyphs and 2px padding preserve 17, 111 and 0 in both
        # segmentation modes. Never choose between conflicting numeric reads.
        results = [subprocess.check_output(
            ['tesseract', str(crop), 'stdout', '-l', 'eng', '--psm', mode,
             '-c', 'tessedit_char_whitelist=0123456789'], env=env, text=True,
            stderr=subprocess.PIPE).strip() for mode in ('7', '13')]
    text = wallet_text(results)
    return int(text), (left, right), text


def toolbar_speed(text):
    match = re.fullmatch(r'(1|2|4|8|16|32)\s*[x×]', text.strip(), re.I)
    assert match, f'Unreadable toolbar speed: {text!r}'
    return int(match[1])


def read_toolbar_speed(path, env=None):
    # Verified 1280x720 toolbar bounds; whole-screen OCR can merge this
    # small label with +/- and the timer even when the actual pixels are clear.
    with tempfile.TemporaryDirectory(prefix='rogue-speed-') as directory:
        crop = Path(directory) / 'speed.png'
        subprocess.run(['convert', str(path), '-crop', '38x28+1029+32',
                        '-resize', '600%', '-colorspace', 'Gray', '-negate',
                        '-bordercolor', 'white', '-border', '20', str(crop)],
                       env=env, check=True, capture_output=True)
        raw = subprocess.check_output(['tesseract', str(crop), 'stdout', '-l', 'eng',
                                       '--psm', '7'], env=env, text=True, stderr=subprocess.PIPE)
        return toolbar_speed(raw)



def strict_battle_clock(reads):
    """Require both independently scaled timer crops to agree on exact MM:SS."""
    seconds = []
    for _, text in reads:
        match = re.fullmatch(r'([0-9]{2}):([0-9]{2})', text.strip())
        if not match or int(match[2]) >= 60:
            raise ValueError(f'Unreadable battle countdown pixels: {reads!r}')
        seconds.append(int(match[1]) * 60 + int(match[2]))
    if len({scale for scale, _ in reads}) != 2 or len(seconds) != 2 or seconds[0] != seconds[1]:
        raise ValueError(f'Conflicting battle countdown pixels: {reads!r}')
    return seconds[0]


def read_smoke_battle_clock(path, env=None):
    # Measured on the actual failed 1280x720 smoke screenshot (23:00).
    # These same narrow timer bounds also read the independent 24:52 battle.
    reads = []
    with tempfile.TemporaryDirectory(prefix='rogue-smoke-clock-') as directory:
        for scale in (300, 400):
            crop = Path(directory) / f'clock-{scale}.png'
            subprocess.run(['convert', str(path), '-crop', '74x24+1194+22',
                            '-resize', f'{scale}%', '-colorspace', 'Gray', '-negate',
                            '-bordercolor', 'white', '-border', '20', str(crop)],
                           env=env, check=True, capture_output=True)
            raw = subprocess.check_output(
                ['tesseract', str(crop), 'stdout', '-l', 'eng', '--psm', '7',
                 '-c', 'tessedit_char_whitelist=0123456789:'],
                env=env, text=True, stderr=subprocess.PIPE).strip()
            reads.append((scale, raw))
    return strict_battle_clock(reads), reads


def wait_battle_progress(shot, env, process, deadline, records,
                        now=time.monotonic, pause=time.sleep):
    """Two decreasing public timer readings, with visible guide/chooser guards."""
    previous = None
    blockers = (
        'MOVE & DODGE', 'THREE LIVES', 'KEEP GROWING', 'YOUR LEGACY',
        '移动与闪避', '三次生命', '持续成长', '永久成长',
        'Click a basic commander to begin', '点击基础指挥官开始远征',
    )
    while now() < deadline:
        if process.poll() is not None:
            raise RuntimeError('Game exited before the public battle clock advanced')
        image = shot(f'battle-progress-{len(records):02}.png')
        observed = now()
        rows = ocr_screenshot(image, env)
        blocked = [line for line in rows if any(
            normalized(line[0]) == normalized(label)
            or re.fullmatch(r'[1-4]4' + re.escape(normalized(label)), normalized(line[0]))
            for label in blockers
        )]
        record = dict(screenshot=image.name, observed_monotonic=observed,
                      blockers=blocked, clock_seconds=None, clock_reads=[])
        records.append(record)
        if blocked:
            previous = None
        else:
            try:
                seconds, reads = read_smoke_battle_clock(image, env)
                record.update(clock_seconds=seconds, clock_reads=reads)
            except (ValueError, subprocess.CalledProcessError) as error:
                record['clock_error'] = str(error)
                previous = None
            else:
                if previous is not None and observed > previous['observed_monotonic'] and seconds < previous['clock_seconds']:
                    record['completed_monotonic'] = now()
                    assert record['completed_monotonic'] < deadline, 'Battle observation exceeded deadline'
                    return previous, record
                previous = record
        record['completed_monotonic'] = now()
        pause(.2)
    raise AssertionError('Two readable decreasing public battle countdowns were not observed')


def find_text(lines, text):
    """Exact line match; choose the bottom action if title and button repeat."""
    matches = [line for line in lines if normalized(line[0]) == normalized(text)]
    return max(matches, key=lambda line: line[2]) if matches else None

def wait_basic_commander(shot, env, send, deadline, records, now=time.monotonic, pause=time.sleep):
    """Dismiss only the observed first real guide, then require recruitment."""
    skip_sent = False
    while now() < deadline:
        image = shot(f'startup-observation-{len(records):02}.png')
        rows = ocr_screenshot(image, env)
        guide = find_text(rows, '1 / 4 · MOVE & DODGE')
        skip = find_text(rows, 'Skip')
        commander = find_text(rows, 'Click a basic commander to begin')
        record = dict(screenshot=image.name, guide=guide, skip=skip, commander=commander)
        records.append(record)
        assert now() < deadline, 'Public startup observation exceeded deadline'
        if guide and skip and not skip_sent:
            send('Escape')
            skip_sent = True
            record['input'] = 'Escape'
        elif commander and not guide:
            return rows
        pause(.2)
    raise AssertionError('Actual basic commander recruitment did not appear within startup budget')


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
    processes, handles = [], []
    battle_observations, battle_proofs = {}, {}
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
        return ocr_screenshot(OUT / 'current-ui.png', env)
    def wait_text(win, process, text):
        return wait(lambda: find_text(visible_lines(win), text), process)
    def click_text(win, process, text):
        _, left, top, right, bottom = wait_text(win, process, text)
        click(win, (left + right) // 2, (top + bottom) // 2)
    def battle_running(win, process, phase):
        records = []
        battle_observations[phase] = records
        def shot(name):
            name = phase + '-' + name
            screenshot(win, name)
            return OUT / name
        first, second = wait_battle_progress(
            shot, env, process, time.monotonic() + 120, records)
        battle_proofs[phase] = dict(first=first, second=second)
    def quit_game(win, process, name):
        # The battle's 12px launcher is not reliably OCR-readable in Chinese.
        # Use its actual centered toolbar position, then require the visible
        # large Settings title before locating the quit action through OCR.
        click(win, int(geometry(win)['WIDTH']) // 2, 30)
        wait_text(win, process, '设置')
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
        battle_running(win, app, 'initial')
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
        battle_running(win, app, 'restart')
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
                  'battle_clock_proofs': battle_proofs,
                  'battle_clock_observations': battle_observations,
                  'scope': 'Ubuntu 24.04 / X11 / software Vulkan; no Steam client or hardware performance claim'}
        (OUT / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print('DESKTOP_RUNTIME_PASS ' + json.dumps(result), flush=True)
    finally:
        (OUT / 'battle-progress.json').write_text(
            json.dumps(dict(proofs=battle_proofs, observations=battle_observations), indent=2) + '\n')
        for process in reversed(processes):
            if process.poll() is None:
                process.terminate()
                try: process.wait(timeout=10)
                except subprocess.TimeoutExpired: process.kill(); process.wait()
        for file in handles: file.close()

if __name__ == '__main__':
    main()
