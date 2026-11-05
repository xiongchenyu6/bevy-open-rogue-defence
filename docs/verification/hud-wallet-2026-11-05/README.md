# Public wallet OCR: repeated ones and zero

R6 completed a natural Wind Boots purchase, then bought Arc Lens at 180 from 291.
The native after frame shows 111 in both the shop and HUD and `Bought Arc Lens`.
The old 600% single-line HUD OCR returned 11, so the strict deduction assertion
stopped the harness. Root inspected before/selected/after frames and independently
reran the public-pixel proof; the game's charge was correct.

The HUD crop now uses 800% enlargement and both single-line/raw-line segmentation.
A result is accepted only when all nonempty reads are the same complete digit
string. Conflicting or nonnumeric reads fail; no candidate is selected because it
would make the expected purchase arithmetic pass. Raw-line mode also reads the
actual zero-balance fixture that single-line mode omitted.

Transactions now additionally require the separately located shop balance to
agree with the HUD after buying, as already required before buying. The shop
header parser requires the UI's actual minimum four zero-padded digits. Exact
price deduction and actual owned-inventory inspection remain mandatory. A known
choice already visible after closing the market is drained before opening the
inventory, using the existing public-choice handler.

**26 tests passed**, including actual 111/291/307/147/100000/99880/0 frames and
two dimmed inventory balances, plus conflicting-read rejection and prior
speed/modal/name/stat/seed checks. `verify.py` independently rereads this native
Arc Lens transaction; `pixels.json` records 291→111 and the actual named purchase
notice. It explicitly does not claim the second owned-item detail was checked:
the old assertion stopped before that step. R6 never reached a terminal game
result, and its finally block terminated the game. No win/loss is inferred.

The actual first owned item in `inventory.png` is Wind Boots. Its dimmed HUD is
used solely as a numeric-reading regression. The earlier fully checked natural
purchase remains in [the earned-market proof](../earned-market-2026-11-05/README.md).
These changes affect test automation only, not shipped prices or gameplay.
