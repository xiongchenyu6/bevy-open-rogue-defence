import shutil
import unittest
from pathlib import Path
from campaign_equipment_runtime import (affordable_gear, market_wallet, inventory_count,
    last_owned_icon, market_closed, inspected_offer, verify_deduction, crop_text,
    screenshot_wallet, TILE_LEFTS)


class EquipmentPolicyParsing(unittest.TestCase):
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

    def test_inventory_header_and_visible_grid(self):
        self.assertEqual(inventory_count([('EQUIPMENT · 12',213,108,400,130)]),12)
        self.assertEqual(last_owned_icon(11),(245,270))
        for count in (0,21):
            with self.assertRaises(AssertionError):last_owned_icon(count)
        with self.assertRaises(AssertionError):inventory_count([])

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
                offer=inspected_offer(crop_text(path,f'120x40+{left}+343',None),
                    crop_text(path,f'39x23+{left+52}+384',None,True),
                    crop_text(path,'190x30+695+480',None),index)
                self.assertEqual((offer['name'],offer['category'],offer['price']),(name,category,price))
                self.assertEqual([int(crop_text(path,f'39x23+{x+52}+384',None,True))
                                  for x in TILE_LEFTS],[300,180,120 if frame==35 else 140,180,280])


if __name__=='__main__':unittest.main()
