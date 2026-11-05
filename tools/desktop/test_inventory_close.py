"""Held visible Close contracts; public OS evidence is separate."""
import unittest
from unittest.mock import Mock
from forge_runtime import close_visible_inventory

OPEN = [('EQUIPMENT · 0',213,108,367,130),('Close',1019,107,1055,125)]
BATTLE = [('LIVES3',23,87,70,98)]

class InventoryCloseTests(unittest.TestCase):
    def test_one_down_delayed_observation_and_finally_release(self):
        down,up=Mock(),Mock(); records=[]
        read=Mock(side_effect=[OPEN,OPEN,BATTLE])
        close_visible_inventory(read,down,up,10,records,now=lambda:0,pause=lambda _:None)
        down.assert_called_once_with(1037,116);up.assert_called_once_with()
        self.assertEqual([r['input'] for r in records if 'input' in r],['mouseDown','mouseUp'])
    def test_unknown_missing_or_wrong_close_never_sends_input(self):
        for rows in [BATTLE,OPEN[:1],OPEN[:1]+[('Close',1,1,20,20)],
                     [('EQUIPMENT',213,108,367,130),OPEN[1]]]:
            down,up=Mock(),Mock()
            with self.assertRaises(AssertionError):
                close_visible_inventory(lambda:rows,down,up,10,[],now=lambda:0)
            down.assert_not_called();up.assert_not_called()
    def test_timeout_keeps_assertion_and_releases_once(self):
        down,up=Mock(),Mock();clock=iter([0,0,0,0,11])
        with self.assertRaisesRegex(AssertionError,'did not close'):
            close_visible_inventory(lambda:OPEN,down,up,10,[],now=lambda:next(clock,11),pause=lambda _:None)
        down.assert_called_once();up.assert_called_once()
    def test_read_failure_and_dispatch_failure_release(self):
        for read,down in [(Mock(side_effect=[OPEN,RuntimeError('read')]),Mock()),
                          (Mock(return_value=OPEN),Mock(side_effect=RuntimeError('dispatch')))]:
            up=Mock()
            with self.assertRaises(RuntimeError):
                close_visible_inventory(read,down,up,10,[],now=lambda:0)
            up.assert_called_once()
