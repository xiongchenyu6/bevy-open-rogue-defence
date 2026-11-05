"""OCR click-target regression; no game, display, containers or preferences."""
import importlib.util
from pathlib import Path
import unittest
import shutil

spec = importlib.util.spec_from_file_location('smoke_runtime', Path(__file__).with_name('smoke_runtime.py'))
smoke = importlib.util.module_from_spec(spec)
spec.loader.exec_module(smoke)


class VisibleActionTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_actual_wallet_digits_do_not_collapse_repeated_ones(self):
        root = Path(__file__).resolve().parents[2] / 'docs/verification'
        cases = [('hud-wallet-2026-11-05/before.png', 291),
                 ('hud-wallet-2026-11-05/after.png', 111),
                 ('earned-market-2026-11-05/before.png', 307),
                 ('earned-market-2026-11-05/after.png', 147),
                 ('black-market-2026-11-04/en-1280x720/frame00035.png', 100000),
                 ('black-market-2026-11-04/en-1280x720/frame00096.png', 99880),
                 ('black-market-2026-11-04/en-1280x720/frame00125.png', 0)]
        for name, expected in cases:
            with self.subTest(frame=name):
                self.assertEqual(smoke.read_hud_wallet(root / name)[0], expected)
        for name, expected in [('earned-market-2026-11-05/inventory.png', 169),
                               ('hud-wallet-2026-11-05/inventory.png', 54)]:
            with self.subTest(dimmed_frame=name):
                self.assertEqual(smoke.read_hud_wallet(root / name, dimmed=True)[0], expected)

    def test_conflicting_numeric_reads_never_choose_an_expected_balance(self):
        self.assertEqual(smoke.wallet_text(['111', '']), '111')
        self.assertEqual(smoke.wallet_text(['0', '0']), '0')
        for results in (['111', '11'], ['', ''], ['0', 'O']):
            with self.assertRaises(ValueError):
                smoke.wallet_text(results)

    def test_multilingual_words_keep_line_bounds_and_choose_button_below_title(self):
        # Tesseract TSV uses per-word rectangles; a confirmation title must
        # never receive the click intended for the identically named action.
        tsv = ('level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n'
               '5\t1\t1\t1\t1\t1\t390\t260\t90\t24\t95\t退出\n'
               '5\t1\t1\t1\t1\t2\t482\t260\t80\t24\t95\t游戏？\n'
               '5\t1\t2\t1\t1\t1\t710\t400\t100\t16\t96\t退出游戏\n')
        lines = smoke.ocr_lines(tsv)
        self.assertEqual(smoke.find_text(lines, '退出游戏')[1:], (710, 400, 810, 416))
        self.assertIsNone(smoke.find_text(lines, '退出'))

    def test_readiness_background_is_not_a_guide_or_recruitment_confirmation(self):
        lines = [('Preparing expedition', 100, 100, 400, 120), ('8 / 14', 100, 125, 150, 145)]
        self.assertIsNone(smoke.find_text(lines, '1 / 4 · MOVE & DODGE'))
        self.assertIsNone(smoke.find_text(lines, 'Click a basic commander to begin'))
        self.assertIsNone(smoke.find_text(lines, 'Skip'))


if __name__ == '__main__':
    unittest.main()
