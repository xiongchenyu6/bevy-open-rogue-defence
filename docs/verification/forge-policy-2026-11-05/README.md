# Illustrated natural-forge route preparation

`forge_runtime.py` now inspects all five illustrated shop tiles and their shared
preview, using bounded matching rather than the removed `[GEAR]` text rows. It
selects only naturally stocked Arc Lens/Echo Core not already bought in this run.
Spacing is normalized for exact identities, never substring/fuzzy matching.
Selection is rechecked before Buy, with exact header/HUD balance and price checks.

It searches actual owned detail headings instead of assuming append order. When
both same-run materials exist, it verifies the Astral Prism title, ordered pair
of materials and ready action before forging. It then requires the consumed
forge action, unchanged kill balance, and exactly one owned Astral Prism replacing
the two inputs. Resumed countdown, normal quit and byte-identical campaign save
remain required. It does not independently verify every inherited combat modifier.

The required checkpoint is an unmodified actually earned 575-shard/Strength 5 save,
not a funded art fixture. No paid refresh, stock/resource injection or save editing
is allowed. The route is fixed 1280×720 English, bounded 120 attempts/3600 seconds
after unlock, plus a bounded unlock wait. Normal 1/g public input earns the kills.

Five parser/policy tests pass, including actual archived ready-materials and
crafted-inventory pixels and the illustrated market. Combined smoke/campaign/
equipment/forge regressions pass 33 tests. Some forge screenshots are historical
but the tested panel geometry is unchanged; this preparation does not prove a
current-build live craft. The separate live route remains pending.
