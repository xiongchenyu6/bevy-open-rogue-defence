"""Public pixel parser checks; no test here claims a new earned live craft."""
import shutil
import unittest
from unittest.mock import Mock
from pathlib import Path
from smoke_runtime import normalized
from smoke_runtime import ocr_screenshot
from forge_runtime import natural_material, verify_recipe_selection, verify_crafted_inventory, poll_preview, offer_identity, visible_timer
from campaign_equipment_runtime import (crop_text, screenshot_wallet, inspected_offer,
    tile_name_crop, PREVIEW_CROP, TILE_LEFTS, selected_inventory_name, choice_pending, screenshot_battle_seconds)


class ForgePolicyTests(unittest.TestCase):
    def test_natural_same_run_material_selection(self):
        # Synthetic policy cases, separate from the real screenshot checks below.
        offers=[dict(index=i,name=name,category=category,price=price) for i,(name,category,price) in enumerate([
            ('Arc Lens','Equipment',180),('Echo Core','Equipment',300),
            ('Gate Patch','Consumable',120),('Other','Equipment',100),('Arc Lens','Consumable',10)])]
        self.assertEqual(natural_material(offers,300,[])['name'],'Arc Lens')
        self.assertEqual(natural_material(offers,300,['Arc Lens'])['name'],'Echo Core')
        self.assertIsNone(natural_material(offers,179,[]))
        self.assertIsNone(natural_material(offers,300,['Arc Lens','Echo Core']))
        with self.assertRaises(AssertionError):natural_material(offers[:4],1000,[])

    def test_spacing_identity_and_stale_preview_polling(self):
        offers=[dict(index=i,name=name,category='Equipment',price=180) for i,name in enumerate(
            ['ArcLens','EchoCore','Other','Other Two','Other Three'])]
        self.assertEqual(natural_material(offers,180,[])['name'],'Arc Lens')
        self.assertEqual(natural_material(offers,180,['Arc Lens'])['name'],'Echo Core')
        self.assertIsNone(natural_material(offers,180,['ArcLens','Echo Core']))
        samples=iter([('Arc Lens','180','Echo Core + Equipment'),
                      ('ArcLens','180','ArcLens * Equipment')])
        expected=dict(name='Arc Lens',price=180,category='Equipment',index=0,tile_ocr='old noise')
        result=poll_preview(lambda:next(samples),0,expected,pause=lambda _:None)
        self.assertEqual(offer_identity(result),offer_identity(expected))
        with self.assertRaises(AssertionError):
            poll_preview(lambda:('ArcLens','180','Echo Core + Equipment'),0,expected,pause=lambda _:None)
        with self.assertRaises(AssertionError):
            poll_preview(lambda:('ArcLens','181','ArcLens + Equipment'),0,expected,pause=lambda _:None)
        verify_recipe_selection('[SSR] AstralPrism','✓ ArcLens + ✓ EchoCore','Forge equipment')
        for bad in ('ArcLens + Echo Core + Other','EchoCore + ArcLens','ArcLens Extra + EchoCore'):
            with self.assertRaises(AssertionError):
                verify_recipe_selection('[SSR] AstralPrism',bad,'Forge equipment')

    def test_recipe_requires_exact_selected_recipe_and_ready_action(self):
        verify_recipe_selection('[SSR] Astral Prism','/ Arc Lens + ¥ Echo Core','Forge equipment')
        for title,materials,action in [('[SSR] Suncoil','Arc Lens + Echo Core','Forge equipment'),
                ('[SSR] Astral Prism','Arc Lens + Other Echo Core','Forge equipment'),
                ('[SSR] Astral Prism','Arc Lens + Echo Core','Missing materials')]:
            with self.assertRaises(AssertionError):verify_recipe_selection(title,materials,action)

    def test_consumption_requires_only_output_and_both_earned_materials(self):
        verify_crafted_inventory(1,['astralprism'],['Echo Core','Arc Lens'])
        for count,details,acquired in [(2,['astralprism','arclens'],['Arc Lens','Echo Core']),
                (1,['arclens'],['Arc Lens','Echo Core']),
                (1,['astralprism'],['Arc Lens']),
                (1,['astralprism'],['Arc Lens','Arc Lens','Echo Core'])]:
            with self.assertRaises(AssertionError):verify_crafted_inventory(count,details,acquired)

    def test_visible_timer_recovers_late_recruitment_before_each_read(self):
        # A modal appears after an earlier drain returned. Each timer read
        # independently observes its own screenshot and recognized heading.
        shot=Mock(side_effect=['recruit20','hud1','recruit40','hud2'])
        timer=Mock(side_effect=[AssertionError('covered'),100,AssertionError('covered'),97])
        rows=Mock(return_value=[('Level 20 · Recruit a hero',300,200,800,230)])
        choose=Mock()
        self.assertEqual(visible_timer(shot,timer,rows,choose,90,now=lambda:0),100)
        self.assertEqual(visible_timer(shot,timer,rows,choose,90,now=lambda:0),97)
        self.assertEqual(choose.call_args_list,[unittest.mock.call('1'),unittest.mock.call('1')])
        self.assertEqual(rows.call_args_list,[unittest.mock.call('recruit20'),unittest.mock.call('recruit40')])

    def test_visible_timer_unknown_hud_fails_without_input(self):
        choose=Mock()
        with self.assertRaisesRegex(AssertionError,'unreadable'):
            visible_timer(lambda:'unknown',Mock(side_effect=AssertionError('unreadable')),
                lambda image:[('GATE 1000/1000',20,20,150,40)],choose,90,now=lambda:0)
        choose.assert_not_called()

    def test_visible_timer_deadline_bounds_covered_and_success_reads(self):
        for timer_result in (AssertionError('covered'),100):
            choose=Mock();ticks=iter([89,90])
            timer=Mock(side_effect=timer_result) if isinstance(timer_result,Exception) else Mock(return_value=timer_result)
            with self.assertRaises(AssertionError):
                visible_timer(lambda:'screen',timer,
                    lambda image:[('Level 20 · Recruit a hero',300,200,800,230)],
                    choose,90,now=lambda:next(ticks))
            choose.assert_not_called()
        shot=Mock()
        with self.assertRaises(AssertionError):
            visible_timer(shot,Mock(),Mock(),Mock(),90,now=lambda:90)
        shot.assert_not_called()

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),'Screenshot OCR tools unavailable')
    def test_actual_level20_recruitment_covers_timer_and_permits_only_ordinal(self):
        path=Path(__file__).resolve().parents[2]/'docs/verification/earned-forge-2026-11-05/level20-recruitment.png'
        rows=ocr_screenshot(path)
        self.assertTrue(choice_pending(rows))
        self.assertTrue(any(normalized(row[0]) == 'level20recruitahero' for row in rows))
        with self.assertRaises(AssertionError):screenshot_battle_seconds(path,None)
        # The real screenshot is the race input; subsequent exposed timer is
        # mocked. This proves recognition/input gating, not live dismissal.
        shot=Mock(side_effect=[path,'exposed'])
        def timer(image):
            return 100 if image=='exposed' else screenshot_battle_seconds(image,None)
        choose=Mock()
        self.assertEqual(visible_timer(shot,timer,lambda image:rows,choose,90,now=lambda:0),100)
        choose.assert_called_once_with('1')

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),'Screenshot OCR tools unavailable')
    def test_actual_committed_market_and_forge_screenshots(self):
        root=Path(__file__).resolve().parents[2]/'docs/verification'
        market=root/'black-market-2026-11-04/en-1280x720/frame00035.png'
        self.assertEqual(screenshot_wallet(market,None)[0],100000)
        left=TILE_LEFTS[0]
        offer=inspected_offer(crop_text(market,tile_name_crop(left),None),
            crop_text(market,f'39x23+{left+52}+384',None,True),crop_text(market,PREVIEW_CROP,None),0)
        self.assertEqual((offer['name'],offer['category'],offer['price']),('Guardian Seed','Equipment',300))
        ready=root/'earned-forge-2026-10-14/ready-materials.png'
        verify_recipe_selection(crop_text(ready,'840x25+220+394',None),
            crop_text(ready,'840x24+220+420',None),crop_text(ready,'250x28+515+552',None))
        owned=root/'earned-forge-2026-10-14/crafted-inventory.png'
        self.assertEqual(selected_inventory_name(crop_text(owned,'840x25+220+445',None)),'astralprism')


if __name__=='__main__':unittest.main()
