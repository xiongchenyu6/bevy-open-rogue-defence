from smoke_runtime import normalized
import shutil
import unittest
from unittest.mock import patch
from smoke_runtime import ocr_screenshot
from campaign_equipment_runtime import EquipmentPolicy, tile_name_crop, tile_name_text
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
        with patch('campaign_equipment_runtime.crop_text', return_value='LV2 KILLS 0111'):
            self.assertEqual(screenshot_wallet(Path('public.png'), None)[0], 111)
        with patch('campaign_equipment_runtime.crop_text', return_value='LV2 KILLS 111'), \
             self.assertRaises(AssertionError):
            screenshot_wallet(Path('public.png'), None)

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
