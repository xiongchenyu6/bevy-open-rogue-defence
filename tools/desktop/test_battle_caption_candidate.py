"""Real ambiguous LEVEL caption must request, never replace, numeric pixel proof."""
from pathlib import Path
import ast
import unittest
from unittest.mock import patch
from campaign_equipment_runtime import public_panel_state,battle_level_pixels
from smoke_runtime import ocr_screenshot
ROOT=Path(__file__).resolve().parents[2]/'docs/verification'
FAIL=ROOT/'overlap-earned-forge-failure-2026-11-06'

class BattleCaptionCandidateTests(unittest.TestCase):
    def test_actual_lvest_levelt_before_unknown_then_strict_numeric_pixels(self):
        rows=ast.literal_eval((FAIL/'runtime.log').read_text().split(
            'Unexpected public screen while opening market: ',1)[1].strip())
        self.assertTrue(any(r[0]=='LVES3'for r in rows))
        self.assertTrue(any(r[0].startswith('LEVELT ')for r in rows))
        self.assertEqual(public_panel_state(rows),'unknown')
        self.assertEqual(battle_level_pixels(FAIL/'market-02-open-04.png',None),'LEVEL76/168XP')
        self.assertEqual(public_panel_state(rows,FAIL/'market-02-open-04.png',None),'battle')
        # The merged numeric payload is not treated as a level value.
        self.assertEqual(public_panel_state(ocr_screenshot(FAIL/'market-02-open-04.png'),
                                          FAIL/'market-02-open-04.png',None),'battle')
    def test_actual_native_nonbattle_holdouts_keep_priority_without_pixel_read(self):
        cases=[('market-preview-ocr-2026-11-05/initial-market.png','market'),
               ('forge-restart-86cb084-2026-11-06/restart-empty-inventory.png','inventory'),
               ('hud-recruitment-2026-11-05/level10-over-speed.png','choice'),
               ('attribute-settings-return-2026-10-14/en-1280x720/settings.png','unknown'),
               ('packaged-chapter6-2026-10-09/run-117.png','terminal')]
        for name,expected in cases:
            with self.subTest(name=name):
                image=ROOT/name;rows=ocr_screenshot(image)
                with patch('campaign_equipment_runtime.battle_level_pixels',side_effect=AssertionError('No battle read for modal')) as read:
                    self.assertEqual(public_panel_state(rows,image,None),expected)
                    read.assert_not_called()
    def test_ambiguous_candidate_crop_failure_rejects_without_numeric_guess(self):
        hint=[('LEVELT 6/168 XP',490,667,589,679)]
        with patch('campaign_equipment_runtime.battle_level_pixels',side_effect=AssertionError('Unreadable numeric level/XP pixels')) as read:
            with self.assertRaisesRegex(AssertionError,'Unreadable numeric'):
                public_panel_state(hint,Path('disputed.png'))
            read.assert_called_once_with(Path('disputed.png'),None)
    def test_spatial_and_xp_boundaries_remain_required(self):
        for row in [('LEVELT 6/168 XP',100,300,300,320),
                    ('status LEVELT 6/168 XP',490,667,589,679),
                    ('LEVELT 6/168 XPjunk',490,667,589,679),
                    ('LEVELT 6/168 XPP buttons',490,667,589,679),
                    ('LEVELT 6/168 noXP',490,667,589,679)]:
            with self.subTest(row=row),patch('campaign_equipment_runtime.battle_level_pixels') as read:
                self.assertEqual(public_panel_state([row],Path('unknown.png')),'unknown')
                read.assert_not_called()
