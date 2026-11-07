"""Public pixel parser checks; no test here claims a new earned live craft."""
import shutil
import forge_runtime
import unittest
from unittest.mock import Mock, patch
from pathlib import Path
from smoke_runtime import normalized
from smoke_runtime import ocr_screenshot
from forge_runtime import natural_material, verify_recipe_selection, verify_crafted_inventory, poll_market_offer, offer_identity, visible_hud_value, wait_basic_commander
from campaign_equipment_runtime import (crop_text, screenshot_wallet, inspected_offer,
    tile_name_crop, PREVIEW_CROP, TILE_LEFTS, selected_inventory_name, choice_pending, screenshot_battle_seconds)


class ForgePolicyTests(unittest.TestCase):
    def test_magic_penetration_requires_its_own_unambiguous_percentage(self):
        label=('Magic penetration',501,475,617,490)
        value=('20.0%',748,475,780,487)
        unrelated=('Critical chance 95.0%',500,300,780,315)
        self.assertEqual(forge_runtime.magic_penetration_from_rows([label,value,unrelated])[0],20.0)
        self.assertEqual(forge_runtime.magic_penetration_from_rows(
            [('Magic penetration 10.0%',501,475,780,490)])[0],10.0)
        for rows in ([unrelated], [label,('20.0',748,475,780,487)],
                     [label,('20.0%',748,445,780,457)],
                     [label,value,('Magic penetration 10.0%',501,475,780,490)],
                     [('Magic penetration 120.0%',501,475,780,490)]):
            with self.assertRaises(AssertionError):
                forge_runtime.magic_penetration_from_rows(rows)

    def test_forge_uses_the_same_single_click_purchase_observer(self):
        from campaign_equipment_runtime import purchase_market_offer
        self.assertIs(forge_runtime.purchase_market_offer,purchase_market_offer)

    def test_startup_only_skips_real_guide_with_visible_skip_and_never_repeats_escape(self):
        guide=[('1/4 + MOVE & DODGE',520,326,762,360),('Skip',476,446,507,463)]
        commander=[('Click a basic commander to begin',400,100,800,130)]
        send=Mock();records=[]
        with patch('smoke_runtime.ocr_screenshot',side_effect=[guide,guide,commander]):
            rows=wait_basic_commander(lambda name:Path(name),None,send,90,records,now=lambda:0,pause=lambda _:None)
        self.assertEqual(rows,commander)
        send.assert_called_once_with('Escape')
        self.assertEqual(len(records),3)

    def test_startup_commander_needs_no_skip_and_unknown_screens_never_receive_input(self):
        commander=[('Click a basic commander to begin',400,100,800,130)]
        send=Mock()
        with patch('smoke_runtime.ocr_screenshot',return_value=commander):
            self.assertEqual(wait_basic_commander(lambda name:Path(name),None,send,90,[],now=lambda:0),commander)
        send.assert_not_called()
        for rows in ([('1/4 + MOVE & DODGE',520,326,762,360)],
                     [('Skip',476,446,507,463)],
                     [('Preparing expedition',400,100,800,130),('Skip',476,446,507,463)]):
            send=Mock();ticks=iter([0,0,90])
            with patch('smoke_runtime.ocr_screenshot',return_value=rows),self.assertRaises(AssertionError):
                wait_basic_commander(lambda name:Path(name),None,send,90,[],now=lambda:next(ticks),pause=lambda _:None)
            send.assert_not_called()

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),'Screenshot OCR tools unavailable')
    def test_actual_first_guide_is_skipped_before_recruitment_without_preference_injection(self):
        path=Path(__file__).resolve().parents[2]/'docs/verification/forge-startup-guide-2026-11-05/first-guide.png'
        actual=ocr_screenshot(path)
        self.assertTrue(forge_runtime.find_text(actual,'1 / 4 · MOVE & DODGE'))
        self.assertTrue(forge_runtime.find_text(actual,'Skip'))
        commander=[('Click a basic commander to begin',400,100,800,130)]
        send=Mock()
        with patch('smoke_runtime.ocr_screenshot',side_effect=[actual,commander]):
            result=wait_basic_commander(lambda name:Path(name),None,send,90,[],now=lambda:0,pause=lambda _:None)
        self.assertEqual(result,commander)
        send.assert_called_once_with('Escape')

    def test_campaign_and_forge_share_observed_startup_contract_without_profile_special_cases(self):
        import campaign_runtime
        import smoke_runtime
        self.assertIs(campaign_runtime.wait_basic_commander,smoke_runtime.wait_basic_commander)
        self.assertIs(forge_runtime.wait_basic_commander,smoke_runtime.wait_basic_commander)
        guide=[('1/4 + MOVE & DODGE',520,326,762,360),('Skip',476,446,507,463)]
        commander=[('Click a basic commander to begin',400,100,800,130)]
        for driver in (campaign_runtime,forge_runtime):
            send=Mock()
            with patch('smoke_runtime.ocr_screenshot',side_effect=[guide,commander]):
                self.assertEqual(driver.wait_basic_commander(lambda name:Path(name),None,send,
                    90,[],now=lambda:0,pause=lambda _:None),commander)
            send.assert_called_once_with('Escape')

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
        expected=dict(name='Arc Lens',price=180,category='Equipment',index=0)
        with patch('campaign_equipment_runtime.market_header_text',return_value='BLACK MARKET'), \
                patch('campaign_equipment_runtime.tile_name_text',return_value='ArcLens'), \
                patch('campaign_equipment_runtime.crop_text',side_effect=['180','Echo Core + Equipment','180','ArcLens * Equipment']):
            observations=[]
            result=forge_runtime.poll_market_offer(lambda name:Path(name),None,0,'forge',90,
                observations,expected,now=lambda:0,pause=lambda _:None)
        self.assertEqual(offer_identity(result),offer_identity(expected))
        self.assertEqual(len(observations),2)
    def test_recipe_requires_exact_owned_cards_selected_recipe_and_ready_action(self):
        materials = [dict(name='Arc Lens', status='✓ Owned'), dict(name='Echo Core', status='Owned')]
        verify_recipe_selection('[SSR] Astral Prism', materials, 'Forge equipment')
        for tick in ('v Owned', 'v¥ Owned', '¥ Owned', 'Owned'):
            verify_recipe_selection('[SSR] Astral Prism', [dict(name='Arc Lens', status=tick), materials[1]], 'Forge equipment')
        for status in ('Not Owned', 'Unowned', 'Owned Extra', 'Missing'):
            with self.assertRaises(AssertionError):
                verify_recipe_selection('[SSR] Astral Prism', [dict(name='Arc Lens', status=status), materials[1]], 'Forge equipment')
        invalid = [
            ('[SSR] Suncoil', materials, 'Forge equipment'),
            ('[SSR] Astral Prism', list(reversed(materials)), 'Forge equipment'),
            ('[SSR] Astral Prism', materials + [dict(name='Other', status='Owned')], 'Forge equipment'),
            ('[SSR] Astral Prism', [materials[0], dict(name='Echo Core', status='Missing')], 'Forge equipment'),
            ('[SSR] Astral Prism', [materials[0], dict(name='Other Echo Core', status='Owned')], 'Forge equipment'),
            ('[SSR] Astral Prism', materials, 'Missing materials'),
            ('[SSR] Astral Prism', '✓ Arc Lens + ✓ Echo Core', 'Forge equipment'),
        ]
        for title, cards, action in invalid:
            with self.subTest(title=title, materials=cards, action=action), self.assertRaises(AssertionError):
                verify_recipe_selection(title, cards, action)

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'), 'Screenshot OCR tools unavailable')
    def test_actual_startup_failure_already_has_running_23_minute_clock(self):
        image = Path(__file__).resolve().parents[2] / 'docs/verification/forge-startup-clock-2026-11-08/failure-running.png'
        self.assertEqual(screenshot_battle_seconds(image, None), 23 * 60 + 38)

    def test_unrecognized_hud_is_reobserved_without_sending_input(self):
        choose = Mock()
        observations = []
        with patch('forge_runtime.public_panel_state', side_effect=['unknown', 'battle']):
            value = visible_hud_value(Mock(side_effect=['unclear', 'clear']), Mock(return_value=1410),
                Mock(return_value=[]), choose, 90, now=lambda: 0, pause=Mock(), observations=observations)
        self.assertEqual(value, 1410)
        choose.assert_not_called()
        self.assertEqual([row['state'] for row in observations], ['unknown', 'battle'])

    def test_unreadable_classification_and_metric_are_retained_before_valid_read(self):
        observations = []
        choose = Mock()
        with patch('forge_runtime.public_panel_state', side_effect=[AssertionError('unclear level'), 'battle', 'battle']):
            value = visible_hud_value(Mock(side_effect=['level', 'digits', 'clear']),
                Mock(side_effect=[ValueError('unclear timer'), 1405]), Mock(return_value=[]), choose,
                90, now=lambda: 0, pause=Mock(), observations=observations)
        self.assertEqual(value, 1405)
        choose.assert_not_called()
        self.assertEqual(observations[0]['classification_error'], 'unclear level')
        self.assertEqual(observations[1]['metric_error'], 'unclear timer')
        self.assertEqual(observations[2]['value'], 1405)

    def test_consumption_requires_only_output_and_both_earned_materials(self):
        verify_crafted_inventory(1,['astralprism'],['Echo Core','Arc Lens'])
        for count,details,acquired in [(2,['astralprism','arclens'],['Arc Lens','Echo Core']),
                (1,['arclens'],['Arc Lens','Echo Core']),
                (1,['astralprism'],['Arc Lens']),
                (1,['astralprism'],['Arc Lens','Arc Lens','Echo Core'])]:
            with self.assertRaises(AssertionError):verify_crafted_inventory(count,details,acquired)

    def test_forge_shared_inventory_opener_settles_missed_input_without_toggling_open_panel(self):
        battle=[('LIVES2',23,87,70,98)]
        owned=[('EQUIPMENT · 1',213,108,400,130)]
        send=Mock();records=[]
        with patch('campaign_equipment_runtime.ocr_screenshot',side_effect=[battle,battle,battle,owned]), \
                patch('campaign_equipment_runtime.screenshot_battle_seconds',side_effect=[1167,1167,1166]), \
                patch('campaign_equipment_runtime.read_toolbar_speed',return_value=1), \
                patch('campaign_equipment_runtime.time.monotonic',return_value=0):
            result=forge_runtime.wait_panel_open(lambda name:Path(name),None,send,90,records,'forge-owned','inventory')
        self.assertEqual(result[1],owned)
        self.assertEqual([call.args[0] for call in send.call_args_list],['i'])
        self.assertEqual([record['state'] for record in records],['battle','battle','battle','inventory'])
        self.assertNotIn('input',records[-1])

    def test_visible_hud_value_recovers_late_recruitment_before_each_read(self):
        # A modal appears after an earlier drain returned. Each timer read
        # independently observes its own screenshot and recognized heading.
        shot=Mock(side_effect=['recruit20','hud1','recruit40','hud2'])
        timer=Mock(side_effect=[100,97])
        rows=Mock(side_effect=[[('Level 20 · Recruit a hero',300,200,800,230)],[('LIVES2',23,87,70,98)],[('Level 40 · Recruit a hero',300,200,800,230)],[('LIVES2',23,87,70,98)]])
        choose=Mock()
        self.assertEqual(visible_hud_value(shot,timer,rows,choose,90,now=lambda:0),100)
        self.assertEqual(visible_hud_value(shot,timer,rows,choose,90,now=lambda:0),97)
        self.assertEqual(choose.call_args_list,[unittest.mock.call('1'),unittest.mock.call('1')])
        self.assertEqual(rows.call_args_list,[unittest.mock.call('recruit20'),unittest.mock.call('hud1'),unittest.mock.call('recruit40'),unittest.mock.call('hud2')])

    def test_visible_hud_value_unknown_hud_waits_until_deadline_without_input(self):
        choose, metric = Mock(), Mock()
        with self.assertRaisesRegex(AssertionError, 'deadline'):
            visible_hud_value(lambda: 'unknown', metric,
                lambda image: [('GATE 1000/1000', 20, 20, 150, 40)], choose, 90,
                now=Mock(side_effect=[0, 0, 90]), pause=Mock())
        choose.assert_not_called()
        metric.assert_not_called()

    def test_visible_hud_value_terminal_returns_pending_without_metric_or_keys(self):
        metric=Mock(return_value=1);send=Mock()
        result=visible_hud_value(lambda:'terminal',metric,
            lambda image:[('NO LIVES REMAIN',400,300,800,340)],send,90,now=lambda:0)
        self.assertIsNone(result)
        metric.assert_not_called()
        send.assert_not_called()

    def test_visible_hud_value_deadline_bounds_covered_and_success_reads(self):
        for timer_result in (AssertionError('covered'),100):
            choose=Mock();ticks=iter([89,90])
            timer=Mock(side_effect=timer_result) if isinstance(timer_result,Exception) else Mock(return_value=timer_result)
            with self.assertRaises(AssertionError):
                visible_hud_value(lambda:'screen',timer,
                    lambda image:[('Level 20 · Recruit a hero',300,200,800,230)],
                    choose,90,now=lambda:next(ticks))
            choose.assert_not_called()
        shot=Mock()
        with self.assertRaises(AssertionError):
            visible_hud_value(shot,Mock(),Mock(),Mock(),90,now=lambda:90)
        shot.assert_not_called()

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),'Screenshot OCR tools unavailable')
    def test_actual_level10_masks_speed_before_any_metric_is_accepted(self):
        path=Path(__file__).resolve().parents[2]/'docs/verification/hud-recruitment-2026-11-05/level10-over-speed.png'
        rows=ocr_screenshot(path)
        self.assertEqual(forge_runtime.public_panel_state(rows),'choice')
        for value in (1,1167):
            metric=Mock(return_value=value);send=Mock()
            result=visible_hud_value(Mock(side_effect=[path,'battle']),metric,
                lambda image:rows if image==path else [('LIVES2',23,87,70,98)],send,90,now=lambda:0)
            self.assertEqual(result,value)
            metric.assert_called_once_with('battle')
            send.assert_called_once_with('1')

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
        self.assertEqual(visible_hud_value(shot,timer,lambda image:rows if image==path else [('LIVES2',23,87,70,98)],choose,90,now=lambda:0),100)
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
        # Historical pixels retain their own layout observations.
        self.assertEqual(selected_inventory_name(crop_text(ready,'840x25+220+394',None)), 'astralprism')
        legacy_materials = normalized(crop_text(ready,'840x24+220+420',None))
        self.assertIn('arclens', legacy_materials)
        self.assertIn('echocore', legacy_materials)
        owned=root/'earned-forge-2026-10-14/crafted-inventory.png'
        self.assertEqual(selected_inventory_name(crop_text(owned,'840x25+220+445',None)),'astralprism')


if __name__=='__main__':unittest.main()
