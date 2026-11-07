"""Current split-panel geometry against tracked native pixels, not live earned-route proof."""
from pathlib import Path
import subprocess
import unittest

from campaign_equipment_runtime import (
    INVENTORY_TITLE_CROP, crop_text, inventory_icons, selected_inventory_name,
)
from forge_runtime import (
    FORGE_RECIPE_PRISM_CENTER, FORGE_ACTION_CROP, read_recipe_materials, verify_recipe_selection,
)
from smoke_runtime import normalized

ROOT = Path(__file__).resolve().parents[2]
PROOF = ROOT / 'docs/verification/inventory-split-2026-11-05/after'
GEAR = PROOF / 'equipment-en-desktop/frame00230.png'
READY = ROOT / 'docs/verification/forge-reader-2026-11-08/frames/prism-ready-en.png'
MISSING = READY.with_name('prism-missing-en.png')


class NativeInventoryLayoutTests(unittest.TestCase):
    def test_all_visible_owned_card_centers_and_finite_bound(self):
        centers = inventory_icons(12)
        self.assertEqual(centers, [(261 + 104 * col, 193 + 104 * row)
                                   for row in range(4) for col in range(3)])
        for count in (0, 13, 20):
            with self.assertRaises(AssertionError):
                inventory_icons(count)
        # Real fifth owned card sits at the second row, second column.
        x, y = inventory_icons(5)[-1]
        self.assertEqual((x, y), (365, 297))
        rarity = crop_text(GEAR, f'36x14+{x-45}+{y-44}', None)
        self.assertEqual(normalized(rarity), 'ssr')

    def test_selected_identity_is_beside_art_not_old_bottom_strip(self):
        self.assertEqual(selected_inventory_name(crop_text(GEAR, INVENTORY_TITLE_CROP, None)),
                         'bloodcontract')
        self.assertEqual(selected_inventory_name(crop_text(READY, INVENTORY_TITLE_CROP, None)),
                         'astralprism')
        for image in (GEAR, READY):
            with self.assertRaises(AssertionError):
                selected_inventory_name(crop_text(image, '840x25+220+445', None))

    def test_real_recipe_grid_prism_caption_and_ready_selected_materials(self):
        self.assertEqual(FORGE_RECIPE_PRISM_CENTER, (443, 207))
        # Caption directly below the clicked second-card image identifies Prism.
        self.assertEqual(normalized(crop_text(READY, '144x20+371+247', None)), 'astralprism')
        self.assertEqual(normalized(crop_text(READY, '144x20+215+247', None)), 'bloodreaver')
        materials = read_recipe_materials(READY, None)
        self.assertEqual([card['name'] for card in materials], ['Arc Lens', 'Echo Core'])
        verify_recipe_selection(crop_text(READY, INVENTORY_TITLE_CROP, None), materials,
                                crop_text(READY, FORGE_ACTION_CROP, None))
        missing = read_recipe_materials(MISSING, None)
        self.assertEqual([card['name'] for card in missing], ['Arc Lens', 'Echo Core'])
        with self.assertRaises(AssertionError):
            verify_recipe_selection(crop_text(MISSING, INVENTORY_TITLE_CROP, None), missing,
                                    crop_text(MISSING, FORGE_ACTION_CROP, None))

    def test_selected_art_and_distinct_recipe_art_are_nonblank(self):
        def pixels(geometry):
            return subprocess.check_output(['convert', str(READY), '-crop', geometry,
                '+repage', '-resize', '32x32!', 'RGB:-'])
        blood = pixels('96x96+239+157')
        prism = pixels('96x96+395+157')
        selected = pixels('176x176+556+156')
        for raster in (blood, prism, selected):
            self.assertGreater(len(set(raster)), 50)
        self.assertNotEqual(blood, prism)


if __name__ == '__main__':
    unittest.main()
