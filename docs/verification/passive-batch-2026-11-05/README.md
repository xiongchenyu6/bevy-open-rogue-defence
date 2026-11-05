# Same-frame passive accounting

Production ECS regressions reproduced three dropped-output defects. Twelve external spell-projectile signals produced two Storm passive projectiles when split across updates, but one when batched. Ten signals similarly produced two Frost zones versus one. At 300 signals, the old batched output remained one versus fifty Storm projectiles or sixty Frost zones. Eight basic-attack counter increments produced two UR28 Surge strikes/cues when split 4+4 versus one when batched.

Production now consumes all complete threshold groups and retains only the remainder. Storm/Frost counters use u64 rather than truncating a frame at 255. Original thresholds, damage profiles, damage coefficients and ranges are unchanged. Generated passive projectiles remain excluded from the input query. UR echoes retain their actual typed 70%-attack physical strike.

`after-full-tests.log`: optimized library suite **660 passed, 0 failed, 1 ignored**. Before logs deliberately contain failing assertions. These tests use production systems and pool helpers with injected spell entities or attack-counter values. They do not constitute an end-to-end attack/collision, native presentation, performance, or campaign victory test. Native verification is now complete for the bounded fixtures below.

## Native evidence

Four native NVIDIA/Vulkan scenes completed fifty updates each, with twelve saved frames per scene. The production observer, pools, renderer and damage resolver handled synthetic invisible spell inputs; ordinary attacks, learned skills and automatic ultimates were disabled. Both batch/split modes emitted two outputs, retained the partial group without recursion, then emitted exactly one more output and stayed at three. Final Storm target HP matched at 99942.390625; Frost matched at 99857.218750. Exact synchronous output logs establish counts; overlapping screenshots cannot count projectiles. Simulation steps were 10Hz for batching and 60Hz for splitting, both at 1×. These are deterministic stimulus fixtures, not a performance or natural-play benchmark.

The existing UR28 Surge native scene separately completed 400 updates with actual paid branch selection, ordinary attacks, target damage, cues and pause freeze checks. It does not force eight attacks in one frame; that exact parity is established by the ECS regression test.

The primary agent inspected Storm batch/split, Frost batch and UR28 screenshots. They verify live rendering; no claim is made that these retained effects meet the final art direction or subjective combat-feel bar. Strict optimized all-target Clippy with `-D warnings` passed. The debug-only native fixture is excluded from the default shipped release.
