from campaign_equipment_runtime import screenshot_battle_seconds
from smoke_runtime import normalized
import shutil
import unittest
from contextlib import nullcontext
from unittest.mock import patch, Mock
from smoke_runtime import ocr_screenshot
from campaign_equipment_runtime import EquipmentPolicy, tile_name_crop, tile_name_text, public_panel_state, wait_panel_open, battle_level_pixels
from campaign_equipment_runtime import battle_seconds
from pathlib import Path
from campaign_equipment_runtime import (affordable_gear, market_wallet, inventory_count,
    inventory_icons, inventory_open, selected_inventory_name, choice_pending, PREVIEW_CROP, market_closed, inspected_offer, verify_deduction, crop_text,
    screenshot_wallet, TILE_LEFTS, tile_name_matches)


class EquipmentPolicyParsing(unittest.TestCase):
    def test_battle_countdown_requires_complete_valid_time(self):
        self.assertEqual(battle_seconds('24:59'), 1499)
        for text in ('24:99', '24:5', 'ROUND 0/5', '', '24:59 0/5'):
            with self.assertRaises(AssertionError):
                battle_seconds(text)

    def test_countdown_drains_recognized_recruitment_before_reading_hud(self):
        policy = EquipmentPolicy.__new__(EquipmentPolicy)
        keys = []
        policy.key = keys.append
        policy.visits = 4
        policy.ui = dict(shot=lambda name: Path(name), env=None, report={},
                         lines=lambda: [('Level 10 · Recruit a hero', 40, 80, 450, 110)])
        with patch('campaign_equipment_runtime.screenshot_battle_seconds',
                   side_effect=[AssertionError('blank'), 1155]), \
             patch('campaign_equipment_runtime.time.monotonic', return_value=0):
            self.assertEqual(policy.visible_countdown('resume', 10), 1155)
        self.assertEqual(keys, ['1'])
        self.assertEqual(len(policy.ui['report']['equipment_clock_choices']), 1)

    def test_unreadable_clock_without_choice_still_fails_without_keys(self):
        policy = EquipmentPolicy.__new__(EquipmentPolicy)
        keys = []
        policy.key = keys.append
        policy.ui = dict(shot=lambda name: Path(name), env=None, report={}, lines=lambda: [])
        with patch('campaign_equipment_runtime.screenshot_battle_seconds',
                   side_effect=AssertionError('blank')), \
             patch('campaign_equipment_runtime.time.monotonic', return_value=0), \
             self.assertRaises(AssertionError):
            policy.visible_countdown('resume', 10)
        self.assertEqual(keys, [])

    def test_replacement_bond_modal_is_an_in_scope_public_choice(self):
        self.assertTrue(choice_pending([('Choose a bond to replace', 40, 80, 600, 110)]))
        self.assertFalse(choice_pending([('Choose an encounter', 40, 80, 600, 110)]))

    def test_market_header_requires_its_actual_zero_padded_digit_count(self):
        with patch('campaign_equipment_runtime.wallet_header_reads', return_value=[(200,'LV2 KILLS 0111')]):
            self.assertEqual(screenshot_wallet(Path('public.png'), None)[0], 111)
        with patch('campaign_equipment_runtime.wallet_header_reads', return_value=[(200,'LV2 KILLS 111')]), \
             self.assertRaises(AssertionError):
            screenshot_wallet(Path('public.png'), None)

    def test_multiscale_header_rejects_conflicting_complete_values(self):
        for reads in ([(200,'LV1 KILLS 0089'),(300,'LV1 KILLS 0088')],
                      [(200,'LV1 KILLS 0089'),(300,'LV2 KILLS 0089')],
                      [(200,'LVI KILLS 0089 _'),(300,'LV1 KILLS')]):
            with patch('campaign_equipment_runtime.wallet_header_reads',return_value=reads), self.assertRaises(AssertionError):
                screenshot_wallet(Path('public.png'),None)
        with patch('campaign_equipment_runtime.wallet_header_reads',return_value=[
                (200,'LV1 KILLS 0089'),(300,'LV1 KILLS 0089'),(600,'LV1 KILLS')]):
            self.assertEqual(screenshot_wallet(Path('public.png'),None)[0],89)

    def inventory_opener(self, rows, clocks=()):
        policy=EquipmentPolicy.__new__(EquipmentPolicy)
        policy.visits=2
        policy.ui=dict(report={},env=None,shot=Mock(return_value=Path('public.png')))
        policy.key=Mock()
        return policy, patch('campaign_equipment_runtime.ocr_screenshot',side_effect=rows), \
            patch('campaign_equipment_runtime.screenshot_battle_seconds',side_effect=clocks)

    def test_battle_level_anchor_is_exact_spatial_and_blocked_by_optional_modal(self):
        level=('LEVEL1 12/60XP',490,657,600,680)
        self.assertEqual(public_panel_state([level]),'battle')
        for row in [('LEVEL1 12/60XP',100,300,300,320),
                    ('LEVEL1 12 XP',490,657,600,680),
                    ('LEVEL1 unknown/60XP',490,657,600,680),
                    ('Something LEVEL1 12/60XP',490,657,600,680)]:
            self.assertEqual(public_panel_state([row]),'unknown')
        encounter=('Choose an encounter',40,80,600,110)
        self.assertEqual(public_panel_state([level,encounter]),'unknown')
        self.assertEqual(public_panel_state([('LIVES3',23,87,70,98),encounter]),'unknown')

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_actual_packaged_battle_uses_level_anchor_without_correcting_lives(self):
        from smoke_runtime import read_toolbar_speed
        path=Path(__file__).resolve().parents[2]/'docs/verification/sword-package-2026-11-05/smoke/battle.png'
        rows=ocr_screenshot(path)
        self.assertEqual(public_panel_state(rows),'battle')
        self.assertEqual(screenshot_battle_seconds(path,None),24*60+46)
        self.assertEqual(read_toolbar_speed(path,None),1)

    def test_market_opener_uses_same_choice_clock_and_no_toggle_rules(self):
        battle=[('LIVES2',23,87,70,98)]
        market=[('BLACK MARKET',617,214,759,230)]
        policy,ocr,clock=self.inventory_opener([
            [('Level10 · Recruit a hero',40,80,500,110)],battle,battle,battle,market],[100,100,99])
        with ocr,clock,patch('campaign_equipment_runtime.time.monotonic',return_value=0), \
                patch('campaign_equipment_runtime.read_toolbar_speed',return_value=1), \
                patch('campaign_equipment_runtime.screenshot_wallet',return_value=(89,'LV1 KILLS0089')):
            self.assertEqual(wait_panel_open(policy.ui['shot'],None,policy.key,90,[],'test','market'),market)
        self.assertEqual([call.args[0] for call in policy.key.call_args_list],['1','b','b'])
        policy,ocr,clock=self.inventory_opener([market])
        with ocr,clock,patch('campaign_equipment_runtime.time.monotonic',return_value=0), \
                patch('campaign_equipment_runtime.screenshot_wallet',return_value=(89,'LV1 KILLS0089')):
            self.assertEqual(wait_panel_open(policy.ui['shot'],None,policy.key,90,[],'test','market'),market)
        policy.key.assert_not_called()

    def test_level_glyph_hint_requires_real_numeric_pixels_and_never_repairs_text(self):
        hint=[('LEVELS 20/132XP',487,667,592,680)]
        self.assertEqual(public_panel_state(hint),'unknown')
        with patch('campaign_equipment_runtime.battle_level_pixels',side_effect=AssertionError('not numeric')):
            with self.assertRaises(AssertionError):public_panel_state(hint,Path('screen.png'))
        with patch('campaign_equipment_runtime.battle_level_pixels') as pixels:
            self.assertEqual(public_panel_state(hint+[('Choose an encounter',40,80,600,110)],Path('screen.png')),'unknown')
            self.assertEqual(public_panel_state(hint+[('Level 10 · Recruit a hero',40,80,600,110)],Path('screen.png')),'choice')
            pixels.assert_not_called()
        with patch('campaign_equipment_runtime.subprocess.run'), \
                patch('campaign_equipment_runtime.subprocess.check_output',return_value='LEVELS20/132XP'):
            with self.assertRaises(AssertionError):battle_level_pixels(Path('screen.png'),None)

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_actual_two_level5_failures_use_numeric_pixel_signature(self):
        from smoke_runtime import read_toolbar_speed
        root=Path(__file__).resolve().parents[2]/'docs/verification/battle-level-ocr-2026-11-05'
        for name,clock,numeric in [('forge-level5.png',21*60+32,'LEVEL520/132XP'),
                                  ('chapter6-level5.png',21*60+23,'LEVEL59/132XP')]:
            with self.subTest(name=name):
                path=root/name;rows=ocr_screenshot(path)
                self.assertEqual(public_panel_state(rows),'unknown')
                self.assertEqual(public_panel_state(rows,path,None),'battle')
                self.assertEqual(battle_level_pixels(path,None),numeric)
                self.assertEqual(screenshot_battle_seconds(path,None),clock)
                self.assertEqual(read_toolbar_speed(path,None),1)

    def test_inventory_opener_never_toggles_an_open_panel(self):
        rows=[('EQUIPMENT · 2',213,108,400,130)]
        policy,ocr,clock=self.inventory_opener([rows])
        with ocr,clock,patch('campaign_equipment_runtime.time.monotonic',return_value=0):
            self.assertEqual(wait_panel_open(policy.ui['shot'],None,policy.key,90,policy.ui['report'].setdefault('equipment_open_attempts',[]),'test','inventory',2),rows)
        policy.key.assert_not_called()

    def test_inventory_opener_handles_choice_and_waits_for_clock_before_retry(self):
        battle=[('LIVES 2',23,87,70,98)]
        choice=[('Level 20 · Recruit a hero',40,80,500,110)]
        inventory=[('EQUIPMENT · 2',213,108,400,130)]
        policy,ocr,clock=self.inventory_opener([choice,battle,battle,battle,inventory],[1167,1167,1166])
        with ocr,clock,patch('campaign_equipment_runtime.time.monotonic',return_value=0), \
                patch('campaign_equipment_runtime.read_toolbar_speed',return_value=1):
            self.assertEqual(wait_panel_open(policy.ui['shot'],None,policy.key,90,policy.ui['report'].setdefault('equipment_open_attempts',[]),'test','inventory',2),inventory)
        self.assertEqual([call.args[0] for call in policy.key.call_args_list],['1','i','i'])
        records=policy.ui['report']['equipment_open_attempts']
        self.assertEqual([row.get('clock') for row in records],[None,1167,1167,1166,None])
        self.assertNotIn('input',records[2])

    def test_inventory_opener_unknown_or_deadline_never_sends_input(self):
        for rows in ([('Unknown overlay',200,200,500,230)],
                     [('Settings',300,100,500,130),('LIVES2',23,87,70,98)],
                     [('BLACK MARKET',617,214,759,230),('LIVES2',23,87,70,98)]):
            policy,ocr,clock=self.inventory_opener([rows])
            with ocr,clock,patch('campaign_equipment_runtime.time.monotonic',return_value=0), self.assertRaises(AssertionError):
                wait_panel_open(policy.ui['shot'],None,policy.key,90,policy.ui['report'].setdefault('equipment_open_attempts',[]),'test','inventory',2)
            policy.key.assert_not_called()
        policy,ocr,clock=self.inventory_opener([])
        with ocr,clock,patch('campaign_equipment_runtime.time.monotonic',return_value=90), self.assertRaises(AssertionError):
            wait_panel_open(policy.ui['shot'],None,policy.key,90,policy.ui['report'].setdefault('equipment_open_attempts',[]),'test','inventory',2)
        policy.key.assert_not_called()

    def test_inventory_opener_limits_requests_and_defers_real_terminal(self):
        self.assertEqual(public_panel_state([('CHAPTER CLEARED',400,200,800,250),
            ('EQUIPMENT · 2',213,108,400,130)]), 'terminal')
        battle=[('LIVES2',23,87,70,98)]
        policy,ocr,clock=self.inventory_opener([battle]*4,[10,9,8,7])
        with ocr,clock,patch('campaign_equipment_runtime.time.monotonic',return_value=0), \
                patch('campaign_equipment_runtime.read_toolbar_speed',return_value=1),self.assertRaises(AssertionError):
            wait_panel_open(policy.ui['shot'],None,policy.key,90,policy.ui['report'].setdefault('equipment_open_attempts',[]),'test','inventory',2)
        self.assertEqual([call.args[0] for call in policy.key.call_args_list],['i']*3)
        policy,ocr,clock=self.inventory_opener([[('NO LIVES REMAIN',400,300,800,350)]])
        with ocr,clock,patch('campaign_equipment_runtime.time.monotonic',return_value=0):
            self.assertIsNone(wait_panel_open(policy.ui['shot'],None,policy.key,90,policy.ui['report'].setdefault('equipment_open_attempts',[]),'test','inventory',2))
        policy.key.assert_not_called()
        self.assertEqual(policy.ui['report']['equipment_open_attempts'][-1]['state'],'terminal')
        self.assertNotIn('victory',policy.ui['report'])

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_actual_missed_inventory_open_is_live_battle_not_inventory_or_terminal(self):
        from smoke_runtime import read_toolbar_speed
        path=Path(__file__).resolve().parents[2]/'docs/verification/inventory-open-2026-11-05/missed-open-battle.png'
        self.assertEqual(public_panel_state(ocr_screenshot(path)), 'battle')
        self.assertEqual(screenshot_battle_seconds(path,None),19*60+27)
        self.assertEqual(read_toolbar_speed(path,None),1)

    def test_campaign_market_visit_skips_until_public_simulation_unlock(self):
        for clock in (22*60+1,22*60):
            policy=EquipmentPolicy.__new__(EquipmentPolicy)
            policy.next_visit=185;policy.visits=0;policy.key=Mock()
            policy.visible_countdown=Mock(return_value=clock)
            policy.ui=dict(input_lock=nullcontext(),report={},shot=Mock(return_value=Path('unavailable.png')))
            with patch('campaign_equipment_runtime.wait_panel_open') as opener:
                policy.observe([],185)
            opener.assert_not_called();policy.key.assert_not_called()
            self.assertEqual(policy.next_visit,245)
            self.assertEqual(policy.ui['report']['equipment_unlock_checks'],
                             [dict(visit=1,countdown=clock,unlocked=False)])
            policy.ui['shot'].assert_called_once_with('equipment-01-unavailable.png')
        policy=EquipmentPolicy.__new__(EquipmentPolicy)
        policy.next_visit=185;policy.visits=0;policy.key=Mock()
        policy.visible_countdown=Mock(return_value=22*60-1)
        policy.ui=dict(input_lock=nullcontext(),report={},shot=Mock(),env=None)
        with patch('campaign_equipment_runtime.wait_panel_open',side_effect=RuntimeError('unlocked route reached')) as opener, \
                self.assertRaisesRegex(RuntimeError,'unlocked route reached'):
            policy.observe([],185)
        opener.assert_called_once()
        self.assertEqual(opener.call_args.args[-2], 'market')
        self.assertTrue(policy.ui['report']['equipment_unlock_checks'][0]['unlocked'])
        policy.key.assert_not_called()

    def test_current_header_and_ambiguous_wallet(self):
        self.assertEqual(market_wallet([('LV1 KILLS 99880',1120,205,1209,214)]),99880)
        self.assertFalse(market_closed([('BLACK MARKET',617,214,759,230)]))
        self.assertTrue(market_closed([]))
        with self.assertRaises(AssertionError):
            market_wallet([('KILLS 10',1120,205,1209,214),('KILLS 11',1120,205,1209,214)])

    def test_fail_closed_on_unreadable_or_wrong_preview(self):
        for args in [('Arc Lens','?', 'Arc Lens * Equipment',0),
                     ('Arc Lens','180','Other Lens * Equipment',0),
                     ('Arc Lens','180','Arc Lens',0)]:
            with self.assertRaises(AssertionError):
                inspected_offer(*args)
        with self.assertRaises(AssertionError):
            affordable_gear([],1000)

    def test_wrapped_tile_name_is_exact_and_contiguous(self):
        self.assertTrue(tile_name_matches('Prismatic\nEngine','Prismatic Engine'))
        self.assertTrue(tile_name_matches('ae\nLegion\nStandard','Legion Standard'))
        for text in ('Prismatic Extra\nEngine', 'Prismatic\n180\nEngine',
                     'Prismatic\nEquipment\nEngine', 'Prismatic\n?\nEngine',
                     'Prismatic\nEngine Extra', 'Prismatic\nThree\nEngine'):
            self.assertFalse(tile_name_matches(text,'Prismatic Engine'), text)
        offer=inspected_offer('Prismatic\nEngine','300','Prismatic Engine + Equipment',0)
        self.assertEqual(offer['name'],'Prismatic Engine')

    def test_cheapest_equipment_requires_all_inspected_offers(self):
        # Transaction constraints; these artificial offers are not native evidence.
        offers=[dict(name=str(i),index=i,price=p,category=c) for i,(p,c) in enumerate(
            [(300,'Equipment'),(180,'Equipment'),(120,'Consumable'),(180,'Equipment'),(280,'Equipment')])]
        self.assertEqual([o['index'] for o in affordable_gear(offers,200)],[1,3])
        self.assertEqual(affordable_gear(offers,179),[])
        verify_deduction(200,20,offers[1])
        for before,after in [(200,80),(179,-1),(200,200)]:
            with self.assertRaises(AssertionError):
                verify_deduction(before,after,offers[1])

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_actual_padded_wallet0089_and_prior_market_headers(self):
        root=Path(__file__).resolve().parents[2]/'docs/verification'
        for relative,expected in [
                ('market-wallet-0089-2026-11-05/wallet0089.png',89),
                ('hud-wallet-2026-11-05/before.png',291),
                ('hud-wallet-2026-11-05/after.png',111),
                ('market-name-crop-2026-11-05/natural-stock.png',110),
                ('market-red-label-2026-11-05/red-label.png',969)]:
            with self.subTest(path=relative):
                self.assertEqual(screenshot_wallet(root/relative,None)[0],expected)

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_actual_two_item_inventory_header_survives_quote_and_plus_ocr(self):
        root = Path(__file__).resolve().parents[2]
        path = root / 'docs/verification/inventory-ocr-2026-11-05/two-items.png'
        self.assertEqual(inventory_count(ocr_screenshot(path)), 2)

    def test_inventory_header_and_visible_grid(self):
        self.assertEqual(inventory_count([('EQUIPMENT · 12',213,108,400,130)]),12)
        self.assertEqual(inventory_count([('EQUIPMENT + 2',215,108,367,127)]),2)
        self.assertEqual(inventory_count([('EQUIPMENT \" 2',215,108,367,127)]),2)
        self.assertEqual(inventory_icons(11)[10],(245,270))
        for count in (0,21):
            with self.assertRaises(AssertionError):inventory_icons(count)
        with self.assertRaises(AssertionError):inventory_count([])

    def test_queue_and_inventory_require_exact_evidence(self):
        self.assertTrue(choice_pending([('Arc Spark · Choose an evolution',300,200,800,240)]))
        self.assertTrue(inventory_open([('EQUIPMENT · 12',213,108,400,130)]))
        self.assertFalse(inventory_open([('Equipment acquired',500,400,800,420)]))
        self.assertEqual(selected_inventory_name('[R] Arc Lens'),'arclens')
        for text in ('Arc Lens','+10% Arc Lens damage','[R] Arc Lens\nOther'):
            with self.assertRaises(AssertionError):selected_inventory_name(text)
        self.assertEqual(len(inventory_icons(20)),20)

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_native_natural_stock_names_exclude_art_and_tile_borders(self):
        root = Path(__file__).resolve().parents[2]
        path = root / 'docs/verification/market-name-crop-2026-11-05/natural-stock.png'
        names = ['Wind Boots', 'Arc Lens', 'Gate Patch', 'Skill Tuner', 'Ember Heart']
        for left, name in zip(TILE_LEFTS, names):
            with self.subTest(name=name):
                self.assertEqual(tile_name_text(path, left, None), name)

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_native_red_ur_tile_contrast_without_fuzzy_matching(self):
        path = Path(__file__).resolve().parents[2] / 'docs/verification/market-red-label-2026-11-05/red-label.png'
        # This unchanged public screenshot previously produced "| Chaosie"
        # under luminance grayscale. Preserve red glyph contrast, then still
        # require the independently read exact tile/preview/price agreement.
        self.assertEqual(tile_name_text(path, TILE_LEFTS[1], None), 'Chaos Die')
        offer = inspected_offer(tile_name_text(path, TILE_LEFTS[1], None),
            crop_text(path, f'39x23+{TILE_LEFTS[1]+52}+384', None, True),
            crop_text(path, PREVIEW_CROP, None), 1)
        self.assertEqual(normalized(offer['name']), 'chaosdie')
        self.assertEqual(offer['category'], 'Equipment')
        self.assertEqual(offer['price'], 450)

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_real_committed_native_market_pixels(self):
        root=Path(__file__).resolve().parents[2]/'docs/verification/black-market-2026-11-04/en-1280x720'
        for frame,index,wallet,name,category,price in [
            (35,0,100000,'Guardian Seed','Equipment',300),
            (96,2,99880,'Skill Tuner','Consumable',140),
            (125,3,0,'Hardened Plate','Equipment',180),
            (145,4,0,'Ember Heart','Equipment',280)]:
            with self.subTest(frame=frame):
                path=root/f'frame{frame:05}.png'
                self.assertEqual(screenshot_wallet(path,None)[0],wallet)
                left=TILE_LEFTS[index]
                offer=inspected_offer(tile_name_text(path,left,None),
                    crop_text(path,f'39x23+{left+52}+384',None,True),
                    crop_text(path,PREVIEW_CROP,None),index)
                self.assertEqual((offer['name'],offer['category'],offer['price']),(name,category,price))
                self.assertEqual([int(crop_text(path,f'39x23+{x+52}+384',None,True))
                                  for x in TILE_LEFTS],[300,180,120 if frame==35 else 140,180,280])

        detail = root.parent.parent/'chapter6-equipment-interrupted-2026-10-16/equipment-04-inventory.png'
        self.assertEqual(selected_inventory_name(crop_text(detail,'840x25+220+445',None)), 'arclens')


if __name__=='__main__':unittest.main()
