# Enemy body hit flash — 2026-11-05

A real ordinary sword attack now briefly flashes only the struck imported enemy body. Source materials stay immutable; a 70 ms real-time lifetime does not scale with battle speed. Pause freezes feedback. Material restoration respects death, retirement, reuse and external material changes. Newly applied feedback gets at least one rendered frame even under a long update.

## Evidence

- Optimized library suite: **670 passed, 0 failed, 1 ignored** (`full-tests.log`). Seven new lifecycle/isolation tests.
- Strict optimized all-target Clippy: passed (`clippy-r2.log`). The first run found a collapsible diagnostic conditional; corrected without changing gameplay.
- Four native runs, 200 updates each: before/after at 1× and 8×. Every log has explicit material PASS and final 200-frame completion; no panic. A visible untargetable control shares original materials with the targets.
- `comparison.json`: 32 contact records at 1× and 270 at 8× match before/after in frame, HP and damage, ignoring entity IDs and same-frame entity iteration order. This checks these controlled sword scenes, not every attack class.
- Root visually inspected after 1× frames 20 and 25 and after 8× frame 20: struck bodies flash white, unstruck/control bodies remain red, then restore. The archived before/after PNGs support review. CPU checks also assert original material properties, control handles, alpha and shadow flags remain unchanged.

These are diagnostic scenes driven by actual attack/damage code, not a natural campaign or hardware performance benchmark. The static control pose is deliberate. The current b1d7dd1 standalone package predates this source change. PNG frame numbers alone do not measure physical display duration.
