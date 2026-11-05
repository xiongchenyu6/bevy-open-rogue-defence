# Native two-item inventory and literal TSV quotes

R7 completed Wind Boots ownership, then correctly bought Arc Lens 281→101 with
matching shop/HUD balances. The actual inventory showed two icons and
`EQUIPMENT · 2`, but the old reader stopped before selecting the second detail.
This was a harness interruption, not a terminal game result.

Root inspected the native frame and reproduced the OCR rows. One pass read the
separator as `+`; another emitted a literal quote. The default CSV quote handling
then swallowed later unquoted TSV rows into that text cell. Tesseract's actual
output is now read with `QUOTE_NONE`, preserving literal quotes and row bounds.
The dedicated equipment-header grammar accepts the observed separator glyphs,
but still requires the exact header, one integer, known header region and a
unique value. Conflicting counts or unrelated text remain failures.

**33 tests passed**: a quoted-text TSV regression, exact OCR of this actual
count-two frame, prior native market/number/inventory cases and five forge-policy
checks. These tests do not claim a new completed campaign or current-package
natural craft. The game source, prices and inventory logic did not change.
