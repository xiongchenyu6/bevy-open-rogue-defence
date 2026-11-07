# Next public forge validation work

The new ingredient card layout invalidates the old natural-forge screenshot reader in tools/desktop/forge_runtime.py. FORGE_MATERIALS_CROP still targets310x46+744+253 and verify_recipe_selection splits a plus-joined string. The current package preview deliberately covers zero-inventory missing materials and does not claim a naturally earned live craft.

Update the reader to inspect two current ingredient cards separately from one screenshot. Require ordered Arc Lens and Echo Core names, both Owned, the Astral Prism title and Forge equipment action. Remove the old plus-string contract. Preserve wallet/purchase/ownership/consumption/restart assertions. Update tools/desktop/test_forge_runtime.py around line96 and tools/desktop/test_inventory_layout_native.py around line43. Use the current forge-materials proof desktop phase1 frame00215 for ready pixel fixtures, and actual package preview for missing status where applicable. Historical earned-forge screenshots remain historical and must not pass through the current parser.

Package source5e8981d is frozen and already built. Do this work after the package proof commit and input-archive join, so changes to tools do not invalidate the committed552-input audit.
