"""OCR click-target regression; no game, display, containers or preferences."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('smoke_runtime', Path(__file__).with_name('smoke_runtime.py'))
smoke = importlib.util.module_from_spec(spec)
spec.loader.exec_module(smoke)


class VisibleActionTests(unittest.TestCase):
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
