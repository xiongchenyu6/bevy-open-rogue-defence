# Native natural-stock name crop

R5 reached the actual first natural shop, but stopped because the caption OCR
for Gate Patch returned only a dash. Root inspected `natural-stock.png`: the
visible caption is clear and the shared preview correctly says Gate Patch.
The old 120×40 crop began at y343 and included the bottom of the illustration.

The name-only crop is now 108×28 at `(tile_left+6,353)`: it excludes the art and
both side borders. A full-width trial still produced border pipes; narrowing to
the actual caption content produced exact Wind Boots, Arc Lens, Gate Patch,
Skill Tuner and Ember Heart strings in this native frame. The public-pointer,
preview, category and price matching requirements remain strict.

Docker regression: 20 tests passed. The new test checks all five real natural
captions exactly, alongside the existing four native art-fixture frames,
inventory detail, toolbar-speed, stat/seed, modal and transaction regressions.
This does not prove a new completed campaign or every future natural stock.
R5 had no purchases or terminal result; the R3 earned purchase proof remains
valid independently.
