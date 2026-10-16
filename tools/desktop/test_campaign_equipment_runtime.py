import unittest
from campaign_equipment_runtime import affordable_gear, market_wallet, inventory_count, last_owned_icon, has_gear_rows, market_closed


class EquipmentPolicyParsing(unittest.TestCase):
    def test_only_actual_market_header_wallet(self):
        rows = [('KILLS 0230', 1140, 266, 1240, 280),
                ('TOTAL KILLS 800', 500, 350, 700, 370)]
        self.assertEqual(market_wallet(rows), 230)
        self.assertIsNone(market_wallet(rows[1:]))

    def test_no_refresh_skill_or_unaffordable_purchase(self):
        rows = [('[R / GEAR] Arc Lens 180 KILLS', 1000, 315, 1250, 340),
                ('[SR / GEAR] Echo Core 300 KILLS', 1000, 360, 1250, 390),
                ('[R / SKILL] Arc Spark 30 KILLS', 1000, 405, 1250, 430),
                ('REFRESH STOCK 20 KILLS', 1000, 470, 1250, 490),
                ('[R / GEAR] Bogus 10 KILLS', 100, 315, 350, 340)]
        result = affordable_gear(rows, 200)
        self.assertEqual([(item['name'], item['price']) for item in result], [('Arc Lens', 180)])
        self.assertEqual(affordable_gear(rows, 179), [])

    def test_missing_wallet_with_real_gear_offer_is_still_open(self):
        rows = [('[R/GEAR] Arc Lens 180 KILLS', 922, 474, 1074, 502)]
        self.assertTrue(has_gear_rows(rows))
        self.assertIsNone(market_wallet(rows))
        self.assertFalse(market_closed(rows))
        self.assertTrue(market_closed([]))
        self.assertFalse(has_gear_rows([('[R/GEAR] Arc Lens 180 KILLS', 100, 474, 400, 502)]))

    def test_actual_screenshot_wallet_header(self):
        self.assertEqual(market_wallet([('Wi KILLS 0166', 1174, 270, 1251, 279),
            ('7 Sn Ia KILis 0166', 368, 248, 1251, 296)]), 166)

    def test_inventory_header_and_wrapped_icon_boundaries(self):
        self.assertEqual(inventory_count([('EQUIPMENT · 12', 213, 108, 400, 130)]), 12)
        self.assertEqual(last_owned_icon(1), (245, 184))
        self.assertEqual(last_owned_icon(10), (1019, 184))
        self.assertEqual(last_owned_icon(11), (245, 270))
        self.assertEqual(last_owned_icon(20), (1019, 270))
        for count in (0, 21):
            with self.assertRaises(AssertionError):
                last_owned_icon(count)
        with self.assertRaises(AssertionError):
            inventory_count([('EQUIPMENT', 213, 108, 400, 130)])
        with self.assertRaises(AssertionError):
            inventory_count([('EQUIPMENT 2', 213, 108, 400, 130),
                             ('EQUIPMENT 3', 213, 109, 400, 130)])

    def test_ambiguous_wallet_is_failure(self):
        with self.assertRaises(AssertionError):
            market_wallet([('KILLS 10', 1174, 266, 1240, 280),
                           ('KILLS 11', 1174, 268, 1240, 280)])


if __name__ == '__main__':
    unittest.main()
