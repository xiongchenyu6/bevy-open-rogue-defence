from campaign_equipment_runtime import screenshot_battle_seconds, inventory_header_rows, purchase_market_offer
from smoke_runtime import normalized
import shutil
import unittest
from contextlib import nullcontext
from unittest.mock import patch, Mock
from smoke_runtime import ocr_screenshot
from campaign_equipment_runtime import EquipmentPolicy, tile_name_crop, tile_name_text, public_panel_state, wait_panel_open, battle_level_pixels, verify_owned_inventory, market_level_pixel_reads, wallet_header_reads, market_header_text, poll_market_offer
from campaign_equipment_runtime import battle_seconds
from pathlib import Path
from campaign_equipment_runtime import (affordable_gear, market_wallet, inventory_count,
    inventory_icons, inventory_open, selected_inventory_name, choice_pending, PREVIEW_CROP, market_closed, inspected_offer, verify_deduction, crop_text,
    screenshot_wallet, TILE_LEFTS, tile_name_matches)


class EquipmentPolicyParsing(unittest.TestCase):
    def test_purchase_passively_waits_for_exact_debit_with_one_input(self):
        offer=dict(name='Wind Boots',price=160,category='Equipment',index=0)
        click=Mock();shot=Mock(side_effect=[Path('early.png'),Path('settled.png')]);records=[]
        with patch('campaign_equipment_runtime.market_header_text',return_value='BLACK MARKET'), \
             patch('campaign_equipment_runtime.screenshot_wallet',side_effect=[(336,'0336'),(176,'0176')]), \
             patch('campaign_equipment_runtime.read_hud_wallet',side_effect=[(336,None,'336'),(176,None,'176')]), \
             patch('campaign_equipment_runtime.tile_name_text',return_value='Wind Boots'), \
             patch('campaign_equipment_runtime.crop_text',side_effect=['160','Wind Boots + Equipment']):
            result=purchase_market_offer(click,shot,None,offer,336,100,records,'visit',now=lambda:0,pause=lambda _:None)
        click.assert_called_once_with()
        self.assertEqual(result,(Path('settled.png'),176,'0176','176'))
        self.assertEqual([r['wallet'] for r in records if 'wallet' in r],[336,176])
        self.assertTrue(records[-1]['exact_debit'])

    def test_purchase_never_reclicks_on_wrong_debit_conflict_stock_or_unknown_panel(self):
        offer=dict(name='Wind Boots',price=160,category='Equipment',index=0)
        for header,wallet,hud,tile,preview in [
                ('BLACK MARKET',175,175,'Wind Boots','Wind Boots + Equipment'),
                ('BLACK MARKET',176,336,'Wind Boots','Wind Boots + Equipment'),
                ('BLACK MARKET',336,336,'Arc Lens','Arc Lens + Equipment'),
                (None,336,336,'Wind Boots','Wind Boots + Equipment')]:
            with self.subTest(header=header,wallet=wallet,hud=hud,tile=tile):
                click=Mock()
                with patch('campaign_equipment_runtime.market_header_text',side_effect=AssertionError('Unknown panel') if header is None else None,return_value=header), \
                     patch('campaign_equipment_runtime.screenshot_wallet',return_value=(wallet,'raw')), \
                     patch('campaign_equipment_runtime.read_hud_wallet',return_value=(hud,None,'raw')), \
                     patch('campaign_equipment_runtime.tile_name_text',return_value=tile), \
                     patch('campaign_equipment_runtime.crop_text',side_effect=['160',preview]), self.assertRaises(AssertionError):
                    purchase_market_offer(click,lambda _:Path('screen.png'),None,offer,336,100,[],'visit',now=lambda:0,pause=lambda _:None)
                click.assert_called_once_with()

    def test_purchase_unchanged_offer_exhausts_bound_without_another_click(self):
        offer=dict(name='Wind Boots',price=160,category='Equipment',index=0)
        click=Mock();records=[]
        with patch('campaign_equipment_runtime.market_header_text',return_value='BLACK MARKET'), \
             patch('campaign_equipment_runtime.screenshot_wallet',return_value=(336,'0336')), \
             patch('campaign_equipment_runtime.read_hud_wallet',return_value=(336,None,'336')), \
             patch('campaign_equipment_runtime.tile_name_text',return_value='Wind Boots'), \
             patch('campaign_equipment_runtime.crop_text',side_effect=['160','Wind Boots + Equipment']*20), self.assertRaises(AssertionError):
            purchase_market_offer(click,lambda _:Path('unchanged.png'),None,offer,336,100,records,'visit',now=lambda:0,pause=lambda _:None)
        click.assert_called_once_with()
        self.assertEqual(len([r for r in records if 'wallet' in r]),20)

    def test_purchase_deadline_bounds_all_observation_and_no_input_after_expiry(self):
        offer=dict(name='Wind Boots',price=160,category='Equipment',index=0)
        click=Mock()
        with self.assertRaises(AssertionError):
            purchase_market_offer(click,Mock(),None,offer,336,0,[],'visit',now=lambda:0)
        click.assert_not_called()
        clock=[0]
        def hud(*args):
            clock[0]=21
            return (176,None,'176')
        with patch('campaign_equipment_runtime.market_header_text',return_value='BLACK MARKET'), \
             patch('campaign_equipment_runtime.screenshot_wallet',return_value=(176,'0176')), \
             patch('campaign_equipment_runtime.read_hud_wallet',side_effect=hud), self.assertRaises(AssertionError):
            purchase_market_offer(click,lambda _:Path('late.png'),None,offer,336,100,[],'visit',now=lambda:clock[0])
        click.assert_called_once_with()

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_actual_one_click_probe_frames_keep_early_wallet_then_exact_debit(self):
        root=Path(__file__).resolve().parents[2]/'docs/verification/overlap-buy-probe-2026-11-06/chapter6'
        frames=iter([root/'buy-probe-ordinary-02.png',root/'buy-probe-ordinary-03.png'])
        click=Mock();records=[]
        result=purchase_market_offer(click,lambda _:next(frames),None,
            dict(name='Wind Boots',price=160,category='Equipment',index=0),336,100,records,'native',
            now=lambda:0,pause=lambda _:None)
        click.assert_called_once_with()
        self.assertEqual(result[1],176)
        self.assertEqual([r['wallet'] for r in records if 'wallet' in r],[336,176])

    def test_battle_countdown_requires_complete_valid_time(self):
        self.assertEqual(battle_seconds('24:59'), 1499)
        for text in ('24:99', '24:5', 'ROUND 0/5', '', '24:59 0/5'):
            with self.assertRaises(AssertionError):
                battle_seconds(text)

    def test_timer_requires_two_agreed_numeric_scales_without_guessing(self):
        with patch('campaign_equipment_runtime.battle_clock_reads',return_value=[
                (100,'23:39'),(200,'23:39'),(600,'')]):
            self.assertEqual(screenshot_battle_seconds(Path('screen.png'),None),23*60+39)
        for reads in ([(100,'23:39'),(200,'23:38'),(600,'')],
                      [(100,'23:39'),(200,''),(600,'oe')],
                      [(100,'23:39'),(100,'23:39')],
                      [(100,'23:99'),(200,'23:99')],
                      [(100,'339'),(200,'oe'),(600,'')]):
            with patch('campaign_equipment_runtime.battle_clock_reads',return_value=reads), self.assertRaises(AssertionError):
                screenshot_battle_seconds(Path('screen.png'),None)

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_actual2339_timer_and_covered_modal_pixel_negatives(self):
        root=Path(__file__).resolve().parents[2]/'docs/verification'
        actual=root/'battle-timer-ocr-2026-11-05/clock2339.png'
        self.assertEqual(screenshot_battle_seconds(actual,None),23*60+39)
        for relative in ('earned-forge-2026-11-05/level20-recruitment.png',
                         'hud-recruitment-2026-11-05/level10-over-speed.png'):
            with self.subTest(path=relative),self.assertRaises(AssertionError):
                screenshot_battle_seconds(root/relative,None)

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
            self.assertEqual(wait_panel_open(policy.ui['shot'],None,policy.key,90,[],'test','market')[1],market)
        self.assertEqual([call.args[0] for call in policy.key.call_args_list],['1','b'])
        policy,ocr,clock=self.inventory_opener([market])
        with ocr,clock,patch('campaign_equipment_runtime.time.monotonic',return_value=0), \
                patch('campaign_equipment_runtime.screenshot_wallet',return_value=(89,'LV1 KILLS0089')):
            self.assertEqual(wait_panel_open(policy.ui['shot'],None,policy.key,90,[],'test','market')[1],market)
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

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_actual_delayed_market_sequence_sends_one_toggle_and_returns_observed_pixels(self):
        root=Path(__file__).resolve().parents[2]/'docs/verification/panel-toggle-2026-11-05'
        paths=[root/name for name in ('06-battle.png','07-battle.png','08-market.png')]
        rows={path.name:ocr_screenshot(path) for path in paths}
        images=iter(paths);send=Mock();records=[]
        with patch('campaign_equipment_runtime.ocr_screenshot',side_effect=lambda image,env:rows[image.name]):
            image,observed=wait_panel_open(lambda name:next(images),None,send,
                __import__('time').monotonic()+90,records,'test','market')
        send.assert_called_once_with('b')
        self.assertEqual(image,paths[2])
        self.assertEqual(observed,rows[paths[2].name])
        self.assertEqual(screenshot_wallet(image,None)[0],262)
        self.assertEqual(public_panel_state(ocr_screenshot(root/'09-fresh-battle.png')),'battle')
        self.assertNotIn('input',records[1])

    def test_compressed_level_candidate_is_not_positive_without_exact_pixels(self):
        rows=[('LEVEL7 281168XP',487,667,592,680)]
        self.assertEqual(public_panel_state(rows),'unknown')
        with patch('campaign_equipment_runtime.battle_level_pixels',side_effect=AssertionError('invalidpixels')):
            with self.assertRaises(AssertionError):public_panel_state(rows,Path('screen.png'))
        with patch('campaign_equipment_runtime.battle_level_pixels') as pixels:
            self.assertEqual(public_panel_state(rows+[('Choose an encounter',40,80,600,110)],Path('screen.png')),'unknown')
            pixels.assert_not_called()

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_actual_speed_source_missing_level_separator_uses_same_pixels(self):
        from smoke_runtime import read_toolbar_speed
        root=Path(__file__).resolve().parents[2]/'docs/verification/battle-level-separator-2026-11-05'
        failed=root/'speed-level7.png';control=root/'unlock-level4.png'
        rows=ocr_screenshot(failed)
        self.assertEqual(public_panel_state(rows),'unknown')
        self.assertEqual(public_panel_state(rows,failed,None),'battle')
        self.assertEqual(battle_level_pixels(failed,None),'LEVEL728/168XP')
        self.assertEqual(screenshot_battle_seconds(failed,None),20*60+13)
        self.assertEqual(read_toolbar_speed(failed,None),1)
        self.assertEqual(public_panel_state(ocr_screenshot(control)),'battle')
        self.assertEqual(screenshot_battle_seconds(control,None),21*60+51)

    def test_duplicate_x_hint_only_invokes_numeric_pixels_after_modal_guards(self):
        hint=[('LEVEL 11 1/240XxP',487,667,592,679)]
        self.assertEqual(public_panel_state(hint),'unknown')
        with patch('campaign_equipment_runtime.battle_level_pixels',side_effect=AssertionError('not numeric')):
            with self.assertRaises(AssertionError):public_panel_state(hint,Path('screen.png'))
        with patch('campaign_equipment_runtime.battle_level_pixels') as pixels:
            self.assertEqual(public_panel_state(hint+[('Choose an encounter',40,80,600,110)],Path('screen.png')),'unknown')
            self.assertEqual(public_panel_state(hint+[('Level20 · Recruit a hero',40,80,600,110)],Path('screen.png')),'choice')
            pixels.assert_not_called()

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_actual_duplicate_x_level11_caption_requires_exact_numeric_pixels(self):
        from smoke_runtime import read_toolbar_speed
        path=Path(__file__).resolve().parents[2]/'docs/verification/battle-level-xp-2026-11-05/level11-battle.png'
        rows=ocr_screenshot(path)
        self.assertEqual(public_panel_state(rows),'unknown')
        self.assertEqual(public_panel_state(rows,path,None),'battle')
        self.assertEqual(battle_level_pixels(path,None),'LEVEL111/240XP')
        self.assertEqual(screenshot_battle_seconds(path,None),17*60+44)
        self.assertEqual(read_toolbar_speed(path,None),1)

    def test_shared_preview_poll_uses_current_header_and_reobserves_stale_selection(self):
        observations=[]
        expected=dict(name='Arc Lens',price=180,category='Equipment',index=1)
        with patch('campaign_equipment_runtime.market_header_text',return_value='BLACK MARKET'), \
                patch('campaign_equipment_runtime.tile_name_text',return_value='Arc Lens'), \
                patch('campaign_equipment_runtime.crop_text',side_effect=['180','Wind Boots + Equipment','180','ArcLens + Equipment']), \
                patch('campaign_equipment_runtime.ocr_screenshot',side_effect=AssertionError('expensive scene OCR forbidden')):
            offer=poll_market_offer(lambda name:Path(name),None,1,'test',90,observations,expected,
                                    now=lambda:0,pause=lambda _:None)
        self.assertEqual(offer['price'],180)
        self.assertEqual(len(observations),2)
        self.assertIn('error',observations[0]);self.assertNotIn('error',observations[1])
        self.assertNotEqual(observations[0]['screenshot'],observations[1]['screenshot'])

    def test_shared_preview_rejects_unknown_header_wrong_price_and_deadline(self):
        with patch('campaign_equipment_runtime.market_header_text',side_effect=AssertionError('battle')), \
                patch('campaign_equipment_runtime.tile_name_text') as tile,self.assertRaises(AssertionError):
            poll_market_offer(lambda name:Path(name),None,1,'test',90,[],now=lambda:0)
        tile.assert_not_called()
        expected=dict(name='Arc Lens',price=180,category='Equipment',index=1)
        with patch('campaign_equipment_runtime.market_header_text',return_value='BLACK MARKET'), \
                patch('campaign_equipment_runtime.tile_name_text',return_value='Arc Lens'), \
                patch('campaign_equipment_runtime.crop_text',side_effect=lambda path,crop,env,digits=False:'181' if digits else 'Arc Lens + Equipment'), \
                self.assertRaises(AssertionError):
            poll_market_offer(lambda name:Path(name),None,1,'test',90,[],expected,now=lambda:0,pause=lambda _:None)
        with self.assertRaises(AssertionError):
            poll_market_offer(Mock(),None,1,'test',0,[],now=lambda:0)

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_actual_stale_preview_title_pixels_and_battle_modal_negatives(self):
        root=Path(__file__).resolve().parents[2]/'docs/verification'
        stale=root/'market-preview-ocr-2026-11-05/stale-arc-lens.png'
        initial=root/'market-preview-ocr-2026-11-05/initial-market.png'
        for path in (stale,initial):self.assertEqual(market_header_text(path,None),'BLACK MARKET')
        self.assertEqual(tile_name_text(stale,TILE_LEFTS[1],None),'Arc Lens')
        self.assertEqual(crop_text(stale,'39x23+794+384',None,True),'180')
        self.assertEqual(crop_text(stale,PREVIEW_CROP,None),'Wind Boots + Equipment')
        for path in (root/'panel-toggle-2026-11-05/09-fresh-battle.png',
                     root/'hud-recruitment-2026-11-05/level10-over-speed.png'):
            with self.assertRaises(AssertionError):market_header_text(path,None)

    def test_merged_footer_hint_needs_anchored_xp_boundary_and_exact_pixels(self):
        good=[('LEVEL 24 84/474 xP ey io | Se shift @:',483,656,1237,691)]
        self.assertEqual(public_panel_state(good),'unknown')
        with patch('campaign_equipment_runtime.battle_level_pixels',return_value='LEVEL2484/474XP') as pixels:
            self.assertEqual(public_panel_state(good,Path('screen.png')),'battle')
            pixels.assert_called_once()
        for text in ('LEVEL 24 84/474 noXP', 'status LEVEL24 84/474XP',
                     'LEVEL24 84/474XPP buttons','LEVEL24 84/474XPjunk'):
            with patch('campaign_equipment_runtime.battle_level_pixels') as pixels:
                self.assertEqual(public_panel_state([(text,483,656,1237,691)],Path('screen.png')),'unknown')
                pixels.assert_not_called()
        with patch('campaign_equipment_runtime.battle_level_pixels') as pixels:
            self.assertEqual(public_panel_state(good+[('Choose an encounter',40,80,600,110)],Path('screen.png')),'unknown')
            self.assertEqual(public_panel_state(good+[('Level20 · Recruit a hero',40,80,600,110)],Path('screen.png')),'choice')
            pixels.assert_not_called()
        with patch('campaign_equipment_runtime.battle_level_pixels',side_effect=AssertionError('invalid numeric pixels')):
            with self.assertRaises(AssertionError):public_panel_state(good,Path('screen.png'))

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_actual_level24_merged_footer_requires_fresh_numeric_pixels(self):
        from smoke_runtime import read_toolbar_speed
        path=Path(__file__).resolve().parents[2]/'docs/verification/battle-level-footer-2026-11-05/level24-battle.png'
        rows=ocr_screenshot(path)
        self.assertEqual(public_panel_state(rows),'unknown')
        self.assertEqual(public_panel_state(rows,path,None),'battle')
        self.assertEqual(battle_level_pixels(path,None),'LEVEL2484/474XP')
        self.assertEqual(screenshot_battle_seconds(path,None),9*60+50)
        self.assertEqual(read_toolbar_speed(path,None),1)

    def test_owned_route_budget_scales_finitely_and_total_covers_all_visits(self):
        self.assertEqual(EquipmentPolicy.visit_base_budget_seconds,160)
        self.assertEqual(EquipmentPolicy.pause_budget_seconds,3480)
        self.assertEqual(1800+EquipmentPolicy.pause_budget_seconds,5280)
        self.assertLess(5280,6000)
        budgets=[EquipmentPolicy.visit_budget_for_owned(count) for count in range(1,EquipmentPolicy.visit_limit+1)]
        self.assertTrue(all(a < b for a,b in zip(budgets,budgets[1:])))
        self.assertEqual(sum(budgets),EquipmentPolicy.pause_budget_seconds)
        self.assertGreater(budgets[0],EquipmentPolicy.visit_base_budget_seconds)
        for count in (-1,EquipmentPolicy.visit_limit+1):
            with self.assertRaises(AssertionError):EquipmentPolicy.visit_budget_for_owned(count)

    def test_owned_proof_requires_every_prior_item_not_only_new_purchase(self):
        purchases=[dict(name='Wind Boots'),dict(name='Spirit Drum'),dict(name='Serrated Disc')]
        verify_owned_inventory(3,['serrateddisc','spiritdrum','windboots'],purchases)
        for count,names in [(1,['serrateddisc']), (3,['serrateddisc','spiritdrum','spiritdrum']),
                            (3,['serrateddisc','spiritdrum','arclens']),
                            (3,['serrateddisc','spiritdrum'])]:
            with self.assertRaises(AssertionError):verify_owned_inventory(count,names,purchases)

    def test_visit_reserves_deadline_for_all_known_owned_items_plus_next_purchase(self):
        policy=EquipmentPolicy.__new__(EquipmentPolicy)
        policy.next_visit=0;policy.visits=4;policy.key=Mock()
        policy.purchases=[dict(name='Wind Boots'),dict(name='Spirit Drum')]
        policy.visible_countdown=Mock(return_value=21*60)
        policy.ui=dict(input_lock=nullcontext(),report={},shot=Mock(),env=None)
        with patch('campaign_equipment_runtime.time.monotonic',return_value=1000), \
                patch('campaign_equipment_runtime.wait_panel_open',side_effect=RuntimeError('reserved deadline')) as opener, \
                self.assertRaisesRegex(RuntimeError,'reserved deadline'):
            policy.observe([],1000)
        self.assertEqual(opener.call_args.args[3],1000+EquipmentPolicy.visit_budget_for_owned(3))
        self.assertEqual(policy.ui['report']['equipment_visit_budgets'],
            [dict(visit=5,owned_item_bound=3,seconds=EquipmentPolicy.visit_budget_for_owned(3))])
        policy.key.assert_not_called()

    def test_inventory_opener_never_toggles_an_open_panel(self):
        rows=[('EQUIPMENT · 2',213,108,400,130)]
        policy,ocr,clock=self.inventory_opener([rows])
        with ocr,clock,patch('campaign_equipment_runtime.time.monotonic',return_value=0):
            self.assertEqual(wait_panel_open(policy.ui['shot'],None,policy.key,90,policy.ui['report'].setdefault('equipment_open_attempts',[]),'test','inventory',2)[1],rows)
        policy.key.assert_not_called()

    def test_inventory_opener_handles_choice_and_waits_for_clock_before_retry(self):
        battle=[('LIVES 2',23,87,70,98)]
        choice=[('Level 20 · Recruit a hero',40,80,500,110)]
        inventory=[('EQUIPMENT · 2',213,108,400,130)]
        policy,ocr,clock=self.inventory_opener([choice,battle,battle,battle,inventory],[1167,1167,1166])
        with ocr,clock,patch('campaign_equipment_runtime.time.monotonic',return_value=0), \
                patch('campaign_equipment_runtime.read_toolbar_speed',return_value=1):
            self.assertEqual(wait_panel_open(policy.ui['shot'],None,policy.key,90,policy.ui['report'].setdefault('equipment_open_attempts',[]),'test','inventory',2)[1],inventory)
        self.assertEqual([call.args[0] for call in policy.key.call_args_list],['1','i'])
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
        choice=[('Level20 · Recruit a hero',40,80,500,110)]
        policy,ocr,clock=self.inventory_opener([battle,choice,battle,choice,battle,choice,battle],[10,9,8,7])
        with ocr,clock,patch('campaign_equipment_runtime.time.monotonic',return_value=0), \
                patch('campaign_equipment_runtime.read_toolbar_speed',return_value=1),self.assertRaises(AssertionError):
            wait_panel_open(policy.ui['shot'],None,policy.key,90,policy.ui['report'].setdefault('equipment_open_attempts',[]),'test','inventory',2)
        self.assertEqual([call.args[0] for call in policy.key.call_args_list],['i','1','i','1','i','1'])
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
            policy.next_visit=185;policy.visits=0;policy.key=Mock();policy.purchases=[]
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
        policy.next_visit=185;policy.visits=0;policy.key=Mock();policy.purchases=[]
        policy.visible_countdown=Mock(return_value=22*60-1)
        policy.ui=dict(input_lock=nullcontext(),report={},shot=Mock(),env=None)
        with patch('campaign_equipment_runtime.wait_panel_open',side_effect=RuntimeError('unlocked route reached')) as opener, \
                self.assertRaisesRegex(RuntimeError,'unlocked route reached'):
            policy.observe([],185)
        opener.assert_called_once()
        self.assertEqual(opener.call_args.args[-2], 'market')
        self.assertTrue(policy.ui['report']['equipment_unlock_checks'][0]['unlocked'])
        policy.key.assert_not_called()

    def test_lvs_needs_actual_numeric_tag_and_all_header_identities_must_agree(self):
        pixels=[(200,'LV5'),(300,'LV5'),(600,'LV5')]
        with patch('campaign_equipment_runtime.wallet_header_reads',return_value=[(200,'LVS KILLS 1757'),(300,'LVS KILLS 1757')]), \
                patch('campaign_equipment_runtime.market_level_pixel_reads',return_value=pixels):
            self.assertEqual(screenshot_wallet(Path('public.png'),None)[0],1757)
        cases=[([(200,'LVS KILLS1757'),(300,'LVS KILLS1758')],pixels),
               ([(200,'LV4 KILLS1757'),(300,'LVS KILLS1757')],pixels),
               ([(200,'LVS KILLS1757')],[(200,'LV5'),(300,'LV6')]),
               ([(200,'LVS KILLS1757')],[(200,'LV5'),(300,'')]),
               ([(200,'LV5 KILLS1757'),(300,'LVS KILLS1757')],[(200,'LVS'),(300,'LV')])]
        for headers,levels in cases:
            with patch('campaign_equipment_runtime.wallet_header_reads',return_value=headers), \
                    patch('campaign_equipment_runtime.market_level_pixel_reads',return_value=levels),self.assertRaises(AssertionError):
                screenshot_wallet(Path('public.png'),None)
        # The tag number comes from numeric pixels, never from the S character.
        with patch('campaign_equipment_runtime.wallet_header_reads',return_value=[(200,'LVS KILLS0042')]), \
                patch('campaign_equipment_runtime.market_level_pixel_reads',return_value=[(200,'LV3'),(300,'LV3')]):
            self.assertEqual(screenshot_wallet(Path('public.png'),None)[0],42)
        with patch('campaign_equipment_runtime.wallet_header_reads',return_value=[(200,'BAD KILLS1757')]), \
                patch('campaign_equipment_runtime.market_level_pixel_reads') as pixels_reader,self.assertRaises(AssertionError):
            screenshot_wallet(Path('public.png'),None)
        pixels_reader.assert_not_called()

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_actual_market_lv5_tag_pixels_retain_independent_wallet_proof(self):
        from smoke_runtime import read_hud_wallet
        root=Path(__file__).resolve().parents[2]/'docs/verification'
        path=root/'market-header-level5-2026-11-05/market-lv5.png'
        self.assertEqual(wallet_header_reads(path,None),[(200,'LVS KILLS 1757'),(300,'LVS KILLS 1757'),(600,'LVS KILLS 1757')])
        self.assertEqual(market_level_pixel_reads(path,None),[(200,'LV5'),(300,'LV5'),(600,'LV5')])
        self.assertEqual(screenshot_wallet(path,None)[0],1757)
        self.assertEqual(read_hud_wallet(path,None)[0],1757)
        for relative,wallet in [('black-market-2026-11-04/en-1280x720/frame00035.png',100000),
                                ('hud-wallet-2026-11-05/before.png',291),
                                ('market-red-label-2026-11-05/red-label.png',969)]:
            self.assertEqual(screenshot_wallet(root/relative,None)[0],wallet)

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

    @unittest.skipUnless(shutil.which('tesseract') and shutil.which('convert'),
                         'Native screenshot OCR requires Tesseract and ImageMagick')
    def test_isolated_native_inventory_titles_and_noninventory_controls(self):
        root = Path(__file__).resolve().parents[2] / 'docs/verification'
        for relative, expected in [
                ('vortex-chapter6-r2-2026-11-06/chapter6/equipment-08-item-0-before.png', 7),
                ('forge-restart-86cb084-2026-11-06/restart-empty-inventory.png', 0),
                ('inventory-ocr-2026-11-05/two-items.png', 2),
                ('inventory-split-2026-11-05/after/equipment-en-desktop/frame00230.png', 5)]:
            with self.subTest(path=relative):
                self.assertEqual(inventory_count(inventory_header_rows(root/relative)), expected)
        for relative in ('inventory-open-2026-11-05/missed-open-battle.png',
                         'hud-recruitment-2026-11-05/level10-over-speed.png',
                         'vortex-chapter6-r2-2026-11-06/chapter6/equipment-08-market.png'):
            with self.subTest(noninventory=relative), self.assertRaises(AssertionError):
                inventory_count(inventory_header_rows(root/relative))

    def test_inventory_count_rejects_conflicting_or_non_numeric_title_reads(self):
        for rows in ([('EQUIPMENT · 7',210,100,410,132), ('EQUIPMENT + 1',215,108,367,127)],
                     [('EQUIPMENT "了',215,108,367,127)],
                     [('EQUIPMENT · 7 extra',210,100,410,132)],
                     [('EQUIPMENT · 7',210,400,410,432)]):
            with self.subTest(rows=rows), self.assertRaises(AssertionError):
                inventory_count(rows)

    def test_inventory_header_and_visible_grid(self):
        self.assertEqual(inventory_count([('EQUIPMENT · 12',213,108,400,130)]),12)
        self.assertEqual(inventory_count([('EQUIPMENT + 2',215,108,367,127)]),2)
        self.assertEqual(inventory_count([('EQUIPMENT \" 2',215,108,367,127)]),2)
        self.assertEqual(inventory_icons(11)[10],(365,505))
        for count in (0,13):
            with self.assertRaises(AssertionError):inventory_icons(count)
        with self.assertRaises(AssertionError):inventory_count([])

    def test_queue_and_inventory_require_exact_evidence(self):
        self.assertTrue(choice_pending([('Arc Spark · Choose an evolution',300,200,800,240)]))
        self.assertTrue(inventory_open([('EQUIPMENT · 12',213,108,400,130)]))
        self.assertFalse(inventory_open([('Equipment acquired',500,400,800,420)]))
        self.assertEqual(selected_inventory_name('[R] Arc Lens'),'arclens')
        for text in ('Arc Lens','+10% Arc Lens damage','[R] Arc Lens\nOther'):
            with self.assertRaises(AssertionError):selected_inventory_name(text)
        self.assertEqual(len(inventory_icons(12)),12)

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
