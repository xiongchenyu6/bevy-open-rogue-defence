"""OCR click-target regression; no game, display, containers or preferences."""
import importlib.util
from pathlib import Path
import unittest
import shutil
from unittest.mock import patch

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

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_real17_hud_modes_agree_without_using_header_or_expected_price(self):
        from campaign_equipment_runtime import screenshot_wallet, crop_text, verify_deduction
        root=Path(__file__).resolve().parents[2]/'docs/verification/hud-wallet-17-2026-11-05'
        with patch.object(smoke,'wallet_text',wraps=smoke.wallet_text) as consensus:
            after,_,_=smoke.read_hud_wallet(root/'after17.png')
        self.assertEqual(consensus.call_args.args[0],['17','17'])
        self.assertEqual(after,17)
        before,_,_=smoke.read_hud_wallet(root/'before177.png')
        self.assertEqual(before,177)
        self.assertEqual(screenshot_wallet(root/'before177.png',None)[0],before)
        self.assertEqual(screenshot_wallet(root/'after17.png',None)[0],after)
        price=int(crop_text(root/'before177.png','39x23+667+384',None,True))
        self.assertEqual(price,160)
        verify_deduction(before,after,dict(price=price))

    def test_conflicting_numeric_reads_never_choose_an_expected_balance(self):
        self.assertEqual(smoke.wallet_text(['111', '']), '111')
        self.assertEqual(smoke.wallet_text(['0', '0']), '0')
        for results in (['111', '11'], ['17', '7'], ['', ''], ['0', 'O']):
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

    def test_literal_ocr_quote_cannot_consume_later_tsv_lines(self):
        tsv = ('level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n'
               '5\t1\t1\t1\t1\t1\t215\t108\t120\t20\t95\tEQUIPMENT\n'
               '5\t1\t1\t1\t1\t2\t340\t108\t5\t20\t95\t"\n'
               '5\t1\t1\t1\t1\t3\t355\t108\t12\t20\t95\t2\n'
               '5\t1\t2\t1\t1\t1\t904\t110\t29\t11\t95\tGear\n'
               '5\t1\t3\t1\t1\t1\t1019\t109\t36\t13\t95\tClose\n')
        lines = smoke.ocr_lines(tsv)
        self.assertEqual([row[0] for row in lines], ['EQUIPMENT " 2', 'Gear', 'Close'])
        self.assertEqual(smoke.find_text(lines, 'Close')[1:], (1019,109,1055,122))
        self.assertFalse(any('\n' in row[0] or '\t' in row[0] for row in lines))

    def test_readiness_background_is_not_a_guide_or_recruitment_confirmation(self):
        lines = [('Preparing expedition', 100, 100, 400, 120), ('8 / 14', 100, 125, 150, 145)]
        self.assertIsNone(smoke.find_text(lines, '1 / 4 · MOVE & DODGE'))
        self.assertIsNone(smoke.find_text(lines, 'Click a basic commander to begin'))
        self.assertIsNone(smoke.find_text(lines, 'Skip'))


if __name__ == '__main__':
    unittest.main()
