# Same-binary price-policy replay pair — 2026-11-05

Both isolated runs used the same frozen release executable719ae38bbf6bd753fabb371dd5ee274f22648205259ca7678c61c513b099c5b8, seedA11CE, zero-basedchapter5 (chapter6), zero permanent upgrades, active first-card strategy, requested starter2 and behind position, fixed1/60 update and single-thread scheduler. Starter flag2 selects StormSavant, whose HeroKind/actual_hero_id is0; that mapping is correct. Later recruitment follows real first-card choices. These are synthetic chapter-access fixtures, not earned installation playthroughs.

Both ended with real three-life losses, no truncation or timeout. RunA:629.958battle seconds,level15,kills376,frame38519; runB:1019.327seconds,level27,kills988,frame62397. Wall times155.74/211.44seconds. This pair does not estimate player win rates or isolate class strength.

The previous recorded checkpoint(frame19800) matched all recorded fields; the first different checkpoint(frame21600) differed in RNG state, skills,bonds,gold,attacks,gateHP and elapsed time. Complete JSON checkpoint lists differ (23 versus36 records). `comparison.json` checks the standard selected fields; `full-checkpoint-comparison.json` checks every recorded checkpoint field. Neither proves all unrecorded internal states matched earlier. Raw receipts,stderr,scope and curves.csv are preserved.

The24-case matrix was not started because the repeat gate failed. No difficulty/economy/damage values were tuned from these results. A read-only audit identified possible query/archetype ordering hazards; their actual causal role in this pair is not yet established. Targeted collision tests and finer tracing are separate follow-up work. Same seed/fixedstep/singlethread alone must not be described as deterministic replay.
