"""Pixel ACK means selected UI state only; exact owned identity checks remain separate."""
from pathlib import Path
import unittest
from unittest.mock import Mock, patch
from campaign_equipment_runtime import inventory_item_selected, select_visible_inventory_item

ROOT=Path(__file__).resolve().parents[2]
FAILED=ROOT/'docs/verification/inventory-chapter6-2026-11-06/chapter6/equipment-05-inventory-icon-0.png'
OPEN=[('EQUIPMENT · 3',213,108,367,130),('Close',1019,107,1055,125)]

class InventorySelectionAckTests(unittest.TestCase):
    def test_actual_failed_owned_pixels_ack_only_middle_not_clicked_first_or_third(self):
        for index,expected in [(0,False),(1,True),(2,False)]:
            self.assertEqual(inventory_item_selected(FAILED,index,3),expected)
        for index in (-1,3):
            with self.assertRaises(AssertionError):inventory_item_selected(FAILED,index,3)
        with self.assertRaises(AssertionError):inventory_item_selected(FAILED,0,13)
    def test_native_selected_first_and_last_and_closed_screen_rejection(self):
        native=ROOT/'docs/verification/equipment-mouse-2026-11-06/en-desktop'
        self.assertTrue(inventory_item_selected(native/'frame00222.png',0,5))
        self.assertFalse(inventory_item_selected(native/'frame00322.png',0,5))
        self.assertTrue(inventory_item_selected(native/'frame00322.png',4,5))
        with self.assertRaises(AssertionError):
            inventory_item_selected(native/'frame00355.png',0,5)
    def test_one_down_waits_visual_ack_then_releases(self):
        call=Mock();records=[]
        with patch('campaign_equipment_runtime.inventory_header_rows',return_value=OPEN), \
             patch('campaign_equipment_runtime.inventory_item_selected',side_effect=[False,False,True]):
            select_visible_inventory_item(lambda _:Path('actual.png'),call,'42',None,0,10,records,
                                          now=lambda:0,pause=lambda _:None)
        commands=[args[0] for args,_ in call.call_args_list]
        self.assertEqual(commands,[['xdotool','mousemove','--window','42','261','193'],
                                   ['xdotool','mousedown','1'],['xdotool','mouseup','1']])
        self.assertEqual([r['selected'] for r in records if 'selected'in r],[False,True])
    def test_unknown_header_or_bad_index_never_sends_input(self):
        for rows,index in [([],0),(OPEN,3),(OPEN,-1)]:
            call=Mock()
            with patch('campaign_equipment_runtime.inventory_header_rows',return_value=rows),self.assertRaises(AssertionError):
                select_visible_inventory_item(lambda _:Path('actual.png'),call,'42',None,index,10,[],now=lambda:0)
            call.assert_not_called()
    def test_timeout_and_unknown_pixel_release_without_retry(self):
        for pixel in [Mock(return_value=False),Mock(side_effect=[False,AssertionError('unknown pixel')])]:
            call=Mock();clock=iter([0,0,0,0,11])
            with patch('campaign_equipment_runtime.inventory_header_rows',return_value=OPEN), \
                 patch('campaign_equipment_runtime.inventory_item_selected',pixel),self.assertRaises(AssertionError):
                select_visible_inventory_item(lambda _:Path('actual.png'),call,'42',None,0,10,[],
                                              now=lambda:next(clock,11),pause=lambda _:None)
            commands=[args[0] for args,_ in call.call_args_list]
            self.assertEqual(commands.count(['xdotool','mousedown','1']),1)
            self.assertEqual(commands.count(['xdotool','mouseup','1']),1)
