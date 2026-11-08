"""Reject unrelated or ambiguous numbers when verifying an earned stat bonus."""
import unittest
import shutil
from pathlib import Path
from smoke_runtime import toolbar_speed, read_toolbar_speed
from campaign_runtime import strength_from_lines, seed_from_text, strength_purchase, battle_clock_from_lines


class ToolbarSpeedTests(unittest.TestCase):
    def test_strict_toolbar_speed_values(self):
        for text, value in [('1×', 1), ('32x', 32), ('2 x', 2)]:
            self.assertEqual(toolbar_speed(text), value)
        for text in ('1', '12x', 'ROUND 1/5', '1x 24:50', ''):
            with self.assertRaises(AssertionError):
                toolbar_speed(text)

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_native_startup_failure_pixels_still_prove_actual_1x(self):
        root = Path(__file__).resolve().parents[2]
        self.assertEqual(read_toolbar_speed(root / 'docs/verification/chapter6-startup-speed-2026-11-05/actual-1x.png'), 1)


class StrengthEvidenceTests(unittest.TestCase):
    def test_separate_value_stays_in_strength_row_and_first_column(self):
        lines = [('Strength', 203, 235, 258, 248),
                 ('22.0', 456, 235, 484, 248),
                 ('Agility', 203, 265, 250, 278),
                 ('18.0', 456, 265, 484, 278),
                 ('50.0', 745, 235, 780, 248)]
        value, evidence = strength_from_lines(lines)
        self.assertEqual(value, 22.0)
        self.assertEqual(evidence, lines[:2])

    def test_inline_value_does_not_take_later_offense_number(self):
        value, _ = strength_from_lines([('Strength 20.0 Base attack 48.0', 203, 235, 780, 248)])
        self.assertEqual(value, 20.0)

    def test_missing_or_conflicting_strength_is_rejected(self):
        for lines in [[('Agility 22.0', 203, 235, 484, 248)],
                      [('Strength 20.0', 203, 235, 484, 248),
                       ('Strength 22.0', 203, 235, 484, 248)]]:
            with self.assertRaises(AssertionError):
                strength_from_lines(lines)


class ContinuedPurchaseTests(unittest.TestCase):
    def test_actual_earned_level_one_progress_buys_next_level_for_thirty(self):
        progress = dict(unlocked=2, currency=165, cleared='100000', upgrades=[1,0,0,0,0,0])
        cost, expected = strength_purchase(progress)
        self.assertEqual(cost, 30)
        self.assertEqual(expected, dict(unlocked=2, currency=135, cleared='100000', upgrades=[2,0,0,0,0,0]))
        self.assertEqual(progress['currency'], 165)
        self.assertEqual(progress['upgrades'][0], 1)
        # An actual row with the old +2 bonus must fail the new +4 expectation.
        actual, _ = strength_from_lines([('Strength 24.0', 203,235,484,248)])
        self.assertEqual(actual, 20 + 2 * expected['upgrades'][0])

    def test_pricing_increases_and_max_level_cannot_be_purchased(self):
        cost, expected = strength_purchase(dict(currency=100, upgrades=[2,1,0,0,0,0]))
        self.assertEqual(cost, 55)
        self.assertEqual(expected['upgrades'], [3,1,0,0,0,0])
        with self.assertRaisesRegex(AssertionError, 'maximum'):
            strength_purchase(dict(currency=1000, upgrades=[10,0,0,0,0,0]))


class SeedEvidenceTests(unittest.TestCase):
    def test_complete_seed_accepts_ocr_word_spacing(self):
        self.assertEqual(seed_from_text('SEED 0x00000000000A11 CF'), '0x00000000000a11cf')

    def test_truncated_and_overlong_seeds_are_rejected(self):
        for text in ['SEED 0x00000000000A11', 'SEED 0x000000000000A11CF']:
            with self.assertRaises(AssertionError):
                seed_from_text(text)


if __name__ == '__main__':
    unittest.main()


class BattleClockTests(unittest.TestCase):
    def test_late_start_and_valid_toolbar_clock(self):
        self.assertEqual(battle_clock_from_lines([('23:08', 1200, 25, 1260, 45)]), 1388)
        self.assertEqual(battle_clock_from_lines([('24:59', 1200, 25, 1260, 45)]), 1499)

    def test_unrelated_invalid_and_conflicting_clocks(self):
        for lines in [[('23:08', 20, 25, 80, 45)],
                      [('23:08', 1200, 200, 1260, 220)],
                      [('23:60', 1200, 25, 1260, 45)],
                      [('3:30', 1200, 25, 1260, 45)],
                      [('25:01', 1200, 25, 1260, 45)],
                      [('23:08', 1200, 25, 1260, 45), ('23:07', 1200, 25, 1260, 45)]]:
            self.assertIsNone(battle_clock_from_lines(lines))
