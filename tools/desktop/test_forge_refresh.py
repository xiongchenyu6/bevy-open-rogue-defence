"""Refresh policy/parser/observer tests; no live game or successful craft claim."""
import unittest
from pathlib import Path
from unittest.mock import Mock, patch
import forge_refresh as refresh
from forge_runtime import player_cycle, post_refresh_phase_deadline, inspect_refreshed_materials, natural_material

ROOT=Path(__file__).resolve().parents[2]
ACTUAL=ROOT/'docs/verification/earned-forge-loss-2026-11-06/forge/market-18.png'

class RefreshTests(unittest.TestCase):
    def test_actual_f5_marker_and_fee_pixels(self):
        self.assertEqual(refresh.refresh_control(ACTUAL,None),dict(fee=60,center=(910,615),fee_ocr='60',marker_differences=0))

    def test_nonmarket_pixels_cannot_be_refresh_input(self):
        with self.assertRaises(AssertionError):refresh.refresh_control(ACTUAL.with_name('terminal.png'),None)

    def test_visible_marker_required_even_if_fee_is_numeric(self):
        with patch.object(refresh,'market_header_text'),patch.object(refresh.subprocess,'check_output',return_value=bytes(576)),patch.object(refresh,'crop_text',return_value='60'):
            with self.assertRaises(AssertionError):refresh.refresh_control(Path('unknown.png'),None)

    def test_unreadable_or_zero_fee_refused(self):
        pixels=bytes(255 if c=='1' else 0 for c in refresh.MARKER)
        for fee in ('','O0','0','-60','60 80'):
            with patch.object(refresh,'market_header_text'),patch.object(refresh.subprocess,'check_output',return_value=pixels),patch.object(refresh,'crop_text',return_value=fee):
                with self.assertRaises(AssertionError):refresh.refresh_control(Path('x'),None)

    def test_reserve_and_all_budget_boundaries(self):
        self.assertTrue(refresh.refresh_allowed(360,60,set(),1,0,0))
        for args in [(359,60,set(),1,0,0),(1000,60,{1},1,0,0),(1000,60,set(),1,6,0),(1000,60,set(),1,1,301),(1000,0,set(),1,0,0),(1000,None,set(),1,0,0)]:
            self.assertFalse(refresh.refresh_allowed(*args))
        self.assertTrue(refresh.refresh_allowed(420,120,set(),3,3,240))

    def observe(self,records):
        click=Mock();events=[];read=Mock(side_effect=records)
        result=refresh.observe_one_refresh(click,lambda name:Path(name),read,500,60,100,events,'proof',now=lambda:0,pause=lambda _:None)
        click.assert_called_once();return result,events

    def test_one_click_waits_passively_for_exact_debit_and_changed_stock(self):
        result,events=self.observe([dict(wallet=500,hud=500),dict(wallet=440,hud=440,exact_debit=True,stock_changed=True)])
        self.assertEqual(result[1],440);self.assertEqual(len(events),3)

    def test_notice_can_prove_refresh_even_if_random_stock_repeats(self):
        result,_=self.observe([dict(wallet=440,hud=440,exact_debit=True,stock_refreshed_notice=True,stock_changed=False)])
        self.assertEqual(result[1],440)

    def test_wrong_debit_or_hud_mismatch_rejects_without_reclick(self):
        for record in (dict(wallet=430,hud=430,exact_debit=True,stock_changed=True),dict(wallet=440,hud=441,exact_debit=True,stock_changed=True)):
            click=Mock()
            with self.assertRaises(AssertionError):refresh.observe_one_refresh(click,lambda n:Path(n),lambda _:record,500,60,100,[],'x',now=lambda:0,pause=lambda _:None)
            click.assert_called_once()

    def test_debit_without_notice_or_changed_stock_never_qualifies(self):
        click=Mock()
        with self.assertRaises(AssertionError):refresh.observe_one_refresh(click,lambda n:Path(n),lambda _:dict(wallet=440,hud=440,exact_debit=True),500,60,100,[],'x',now=lambda:0,pause=lambda _:None)
        click.assert_called_once()

    def test_expired_deadline_sends_nothing(self):
        click=Mock()
        with self.assertRaises(AssertionError):refresh.observe_one_refresh(click,Mock(),Mock(),500,60,0,[],'x',now=lambda:0)
        click.assert_not_called()

    def test_worker_includes_actual_bond_key_and_stops_before_next_input(self):
        send=Mock();player_cycle(send,lambda:False)
        self.assertEqual([c.args[0] for c in send.call_args_list],['1','g','f'])
        send=Mock();player_cycle(send,lambda:send.call_count==1)
        send.assert_called_once_with('1')

    def test_stock_proof_does_not_claim_material_purchase(self):
        with patch.object(refresh,'market_header_text'),patch.object(refresh,'screenshot_wallet',return_value=(440,'LV4 KILLS0440')),patch.object(refresh,'read_hud_wallet',return_value=(440,(),'440')),patch.object(refresh,'tile_name_text',side_effect=['Wind Boots','Hardened Plate','Revival Seed','Skill Tuner','Spirit Drum']):
            result=refresh.refresh_observation(Path('proof.png'),None,500,60,('a','b','c','d','e'))
        self.assertTrue(result['stock_changed']);self.assertNotIn('acquired_materials',result)


class RefreshPerformanceTests(unittest.TestCase):
    def test_changed_names_skip_notice_ocr(self):
        with patch.object(refresh,'market_header_text'),patch.object(refresh,'screenshot_wallet',return_value=(440,'header')),patch.object(refresh,'read_hud_wallet',return_value=(440,(),'hud')),patch.object(refresh,'tile_name_text',side_effect=['a','b','c','d','e']),patch.object(refresh,'crop_text') as notice:
            result=refresh.refresh_observation(Path('frame.png'),None,500,60,('v','w','x','y','z'))
        notice.assert_not_called();self.assertTrue(result['stock_changed'])

    def test_unchanged_names_require_exact_cropped_feedback(self):
        for text,expected in [('Stock refreshed',True),('Restock 60 cost rises',False),('',False)]:
            with patch.object(refresh,'market_header_text'),patch.object(refresh,'screenshot_wallet',return_value=(440,'header')),patch.object(refresh,'read_hud_wallet',return_value=(440,(),'hud')),patch.object(refresh,'tile_name_text',side_effect=['a','b','c','d','e']),patch.object(refresh,'crop_text',return_value=text) as notice:
                result=refresh.refresh_observation(Path('frame.png'),None,500,60,('a','b','c','d','e'))
            notice.assert_called_once_with(Path('frame.png'),'630x24+614+562',None)
            self.assertEqual(result['stock_refreshed_notice'],expected)

    def test_fresh_inspection_phase_capped_by_original_overall_deadline(self):
        self.assertEqual(post_refresh_phase_deadline(1000,now=lambda:200),(200,320))
        self.assertEqual(post_refresh_phase_deadline(250,now=lambda:200),(200,250))
        with self.assertRaises(AssertionError):post_refresh_phase_deadline(200,now=lambda:200)

    def test_only_missing_material_receives_full_preview(self):
        inspect=Mock(return_value=dict(name='Echo Core',category='Equipment',price=300,index=2))
        records=[]
        offers=inspect_refreshed_materials(('windboots','arclens','echocore','skilltuner','spiritdrum'),['Arc Lens'],inspect,200,records,now=lambda:100)
        inspect.assert_called_once_with(2,200)
        self.assertEqual([r['inspection'] for r in records],['names_only_not_hovered','names_only_not_hovered','full_tile_price_category_preview','names_only_not_hovered','names_only_not_hovered'])
        self.assertEqual(natural_material(offers,300,['Arc Lens'],complete_stock=False)['name'],'Echo Core')
        with self.assertRaises(AssertionError):natural_material(offers,300,['Arc Lens'])

    def test_no_target_names_do_not_hover_or_invent_material(self):
        inspect=Mock();records=[]
        self.assertEqual(inspect_refreshed_materials(('a','b','c','d','e'),[],inspect,200,records,now=lambda:100),[])
        inspect.assert_not_called();self.assertEqual(len(records),5)

    def test_target_preview_still_rejects_expired_phase_and_wrong_material(self):
        for callback,clock in [(Mock(),lambda:200),(Mock(return_value=dict(name='Wind Boots')),lambda:100)]:
            with self.assertRaises(AssertionError):inspect_refreshed_materials(('echocore','b','c','d','e'),[],callback,200,[],now=clock)
