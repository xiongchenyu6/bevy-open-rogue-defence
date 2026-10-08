"""Reject unrelated or ambiguous numbers when verifying an earned stat bonus."""
import unittest
from campaign_runtime import strength_from_lines, seed_from_text


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


class SeedEvidenceTests(unittest.TestCase):
    def test_complete_seed_accepts_ocr_word_spacing(self):
        self.assertEqual(seed_from_text('SEED 0x00000000000A11 CF'), '0x00000000000a11cf')

    def test_truncated_and_overlong_seeds_are_rejected(self):
        for text in ['SEED 0x00000000000A11', 'SEED 0x000000000000A11CF']:
            with self.assertRaises(AssertionError):
                seed_from_text(text)


if __name__ == '__main__':
    unittest.main()
