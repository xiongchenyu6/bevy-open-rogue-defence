"""Reject unrelated or ambiguous numbers when verifying an earned stat bonus."""
import unittest
from campaign_runtime import strength_from_lines, seed_from_text, strength_purchase


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
