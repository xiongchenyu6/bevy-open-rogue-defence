# Combat render timing probe

Set `ROGUE_CAPTURE_TIMINGS` to a writable CSV filename when running `capture frames` or `capture still`. Each row records a zero-based simulation frame, elapsed milliseconds around the actual application update plus blocking GPU submission completion, and post-update entity/enemy/projectile counts. Screenshot queuing occurs before the timer; screenshot processing inside the update is included. CSV formatting and entity counting occur after the timer. Simulation-only mode does not produce timings.

This measures serialized offscreen frame work, not windowed FPS or presentation latency. The current binary uses the optimized development profile. Driver warm-up, asset loading, screenshot readback and paused phases must be identified before interpreting a distribution; none should be silently treated as steady combat. A release-profile, long-session, actual-window check is still required for a shipping performance claim.

The first controlled workloads are `skill-spectacle-combo` (rank-five fire/frost/virus/buff skills against eight durable targets, with real damage assertions) and `summon-assault` (six summon families, pursuit, damage, target replacement, pause and finite-health wall revival). These are mechanism fixtures rather than a naturally earned late-game build or a maximum-density monster wave. Their results cannot establish a full stress-test pass.

Both controlled workloads completed 680 frames on NVIDIA RTX 3070 Laptop / Vulkan / driver 615.71.09 at 1280×720. Actual damage/effect assertions passed; summon pursuit, target replacement, pause and wall rebuilding passed. Four screenshots were inspected. Strict capture Clippy and compilation passed.

| Workload | Median ms | P95 ms | P99 ms | Maximum ms | Peak enemies | Peak projectiles |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Skill combination | 9.54 | 12.75 | 15.03 | 30.40 | 8 | 4 |
| Summon assault | 7.16 | 9.49 | 11.85 | 26.89 | 2 | 9 |

The summary excludes frames 0–119, screenshot frames 299/599 and their immediate successors 300/600; summon pause frames 249–279 are additionally excluded. Raw CSV retains every frame. These fixed exclusions do not prove all loading/readback activity has ended. Enemy counts show that this is a first instrumentation probe, not maximum wave pressure. Peak world entities were 5,664 in both cases, including map/model descendants. No windowed FPS claim follows from these numbers.

The screenshots expose flat bright-green virus geometry and stacked damage labels obscuring targets. That visual issue remains unfinished, alongside higher-density stress and shipping-profile measurements. [Raw logs, timings and screenshots](verification/combat-timing-2026-10-08/verification.json) preserve the evidence.


A subsequent `raid-pressure-25` NVIDIA capture completed 1,795 frames with two actual portal warnings/arrival bursts and a peak of 96 living enemies / 11,336 world entities. Root reviewed three images. With the first 120 frames and screenshot frames450/900/1500 plus their immediate successors excluded, 1,669 samples gave median9.81ms, P95 13.28ms, P99 16.61ms and maximum30.98ms; 15 samples exceeded16.667ms. The fixture raises hero/gate health and disables player attacks and skills, so zero projectiles is expected. It proves a higher actor-count render probe, not simultaneous maximum-density player effects, natural progression or shipping FPS. Evidence is archived with [damage readability](verification/damage-readability-2026-10-08/verification.json). `tools/analyze_render_timings.py` reproduces summaries and rejects missing frames, non-finite timings and negative counts; it was exercised against a real680-row CSV and three invalid-data fixtures.
