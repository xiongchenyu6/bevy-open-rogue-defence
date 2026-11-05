# Full-prefix replay: actual zone critical assignment differs

Two normal complete runs reused the same frozen resolver-diagnostic binary (`3af6a3d59492b116a5071032161ed42da6de64358f8581688f1a02b41f52b6cb`, diagnostic source `4fa93e8`), seed A11CE, chapter index 5, zero permanent upgrades, active starter policy 2 and behind-gate position. There was no new build/source change for these runs. Actor tracing covered **every update from frame 1 to each actual terminal frame**, with a 120000-frame cap and 900-second per-case wall bound. No blind initial interval remains in these recorded fields.

| Run | Normal terminal result | Actor rows / final frame | Battle seconds | Deaths | Kills | Wall seconds |
| --- | --- | --- | --- | --- | --- | --- |
| A | Loss | 46271 | 756.369 | 3 | 555 | 63.85 |
| B | Win | 98484 | 1548.250 | 2 | 2174 | 180.80 |

Both receipts are valid and uncensored. They are diagnostic duplicates, not a population win-rate sample. The underlying chapter access is still synthetic simulator initialization, not a packaged UI clear.

Strict identity differences begin at frame 24. Every recorded nonidentity scalar and actor numeric multiset matches through frame **39556**; the first numeric difference is enemy health at **39557**, battle elapsed 646.8880615 seconds. This limited projection removes only entity IDs/order and does not assert equality of unrecorded statuses/pools or establish logical actor mapping in general.

At that first numeric difference, actual resolver logs provide a direct local mechanism: seven Storm Spell contacts from skill index 15 (Vortex Column) have the same projectile IDs/order, base damage and RNG stream. Two target assignments swap:

| Contact ordinal | Projectile ID bits | Target in A | Target in B | Critical roll | Multiplier / damage |
| --- | --- | --- | --- | --- | --- |
| 2 | 4294959609 | durable 4294959798 | weak 12884895598 | 0.10964847 | 1× / 234.073914 |
| 3 | 8589931742 | weak 12884895598 | durable 4294959798 | 0.03337067 | 1.75× / 409.629333 |

Before the update, the durable enemy has 6662.907227 health and the weak one 229.398392 in both runs. Both normal and critical damage defeat the weak enemy, so A spends the critical on an already-lethal hit while B deals the critical to the durable enemy. Both end with the same RNG state. Predecessor raw enemy slots 11/12 are reversed across the runs; target positions/health match. [Actual contact extracts](first-difference-contacts-a.json) and [B](first-difference-contacts-b.json) include original log lines, profiles, RNG states and mapped previous/after target rows.

Source inspection links this to a concrete candidate for regression: `update_spell_zones` collects affected enemies in query order and schedules typed strikes in that order; `resolve_skill_strikes` rents their stationary projectiles. The logs demonstrate target-order-to-critical-assignment behavior in this update. They do **not** prove why upstream traversal order changed, prove that `HitPunch`/HashMap caused it, or establish complete determinism after any future fix. No numerical tuning or 24-case matrix was run.

## Evidence and summary-counter correction

Full stdout and stderr are losslessly gzip-compressed with empty filename and mtime zero. [raw-receipts.json](raw-receipts.json) records exact compressed/decompressed hashes and independently counted actor/contact rows. Every field and raw contact is retained; large redundant derived all-contact JSONL files are not copied.

The original [sequence.log](sequence.log) is preserved unchanged. Its final `counts` field is an **analysis summary-counter bug**: it reports contact counts **8577 / 109429**, rather than actor-row counts. Correct actor counts are **46271 / 98484**, verified directly from raw JSONL and in [actor-comparison.json](actor-comparison.json). The mislabeled summary does not indicate missing actor rows or fabricated runtime logs.

The [exact predecessor/context](first-difference-context.json), [checkpoint comparison](checkpoint-comparison.json), full runner receipts, parser/manager source and binary receipt accompany this archive. Next verification requires a real zone→strike→pooled-projectile regression before changing production ordering.
