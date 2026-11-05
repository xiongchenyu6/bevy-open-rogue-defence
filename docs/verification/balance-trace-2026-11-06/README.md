# Exact diagnostic trace: divergence remains

The default-off simulator diagnostic was committed as `2f06452` after all six simulator tests and strict all-target release Clippy passed. Two complete runs used the same frozen binary `6d37c568bcd5c0883c2ddcad2b2f2beff10fc4287805e750b2c63499c32076c9`, seed A11CE, active starter policy 2, chapter index 5, training zero and behind-gate position. Both use the production GamePlugin at fixed 1/60-second time with single-threaded schedules and no renderer. Access/prior-clear initialization is synthetic; this is not a packaged UI playthrough.

`ROGUE_SIM_TRACE_RANGE=19000:32400` emits a separate JSONL record after each update, without changing schedules or simulated time. Each run recorded all **13401** trace rows. Every original field is preserved in deterministic gzip files (empty gzip filename, mtime zero); [raw-receipts.json](raw-receipts.json) records compressed/uncompressed hashes, byte counts and row counts. Large executable snapshots remain in target; their hash and build/source receipts are archived here.

| Observation | Frame | Scope |
| --- | --- | --- |
| First traced row equal | 19000 | Every recorded field equal |
| First strict trace difference | 19540 | Two enemy entity IDs differ; numeric payloads match |
| Same recruitment in both | 22648 | Level 10, Storm Savant → Grove Artificer |
| First numeric multiset difference | 30745 | Same enemy ID loses different health |
| First checkpoint difference | 32400 | Coarse checkpoint fields differ |

The strict entity-ID comparison remains authoritative for the recorded trace. A separate limited projection compares **all recorded nonidentity scalars** and actor numeric multisets, removing only entity IDs/order. It does not drop health, position, speed, projectile damage or other recorded numeric columns. It cannot establish actor correspondence or equality of unrecorded status effects, damage profiles, pools and system-local timers.

At frame 30744, the limited numeric projection still matches. At frame 30745, enemy `12884897132` starts at 4710.838867 health and ends at 4363.956543 in A versus 4512.620117 in B: losses 346.882324 versus 198.218750, approximately a 1.75 ratio. Position/max health/speed match. The same RNG state advances in both, kills rise 284→285 in both, and one numerically matching enemy disappears under different IDs. Recorded attacks/casts remain 1020/267. This motivates inspecting actual damage/critical-roll assignment; it **does not prove that critical order, HashMap iteration, or any particular system caused the divergence**.

Subsequent first recorded scalar differences: XP/gold/kills at 30825; level/hero attributes at 30826; projectile numeric multiset at 30831; RNG/attack count at 30838; casts at 30938; wood at 31019; elapsed bits at 31020; owned bonds at 31104. [Semantic timeline](semantic-timeline.json) and [transition context](transition-context.json) retain predecessor rows and exact values. [Strict trace comparison](trace-comparison.json), [numeric projection](numeric-projection-comparison.json) and [full checkpoints](full-checkpoint-comparison.json) preserve their separate scopes.

Both runs reached valid uncensored terminal states: A lost at frame 46801 / 765.200 battle seconds with three deaths (69.19 wall seconds); B won at frame 98250 / 1545.287 seconds with one death (162.41 wall seconds). These are duplicate outcomes, not a win-rate sample. Execution success is distinct from the failed equality gate.

No 24-case matrix, balance tuning or comparator/gameplay change followed this trace. Next work is a bounded trace of actual resolver calls around the first numeric difference. Passing the isolated stationary-collision regression did not establish full-run determinism.
