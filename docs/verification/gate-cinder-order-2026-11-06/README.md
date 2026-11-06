# Gate settlement and Cinder equal-distance ordering

Two independent production corrections address concrete storage-order dependencies
observed in the [previous full replay](../balance-repeat-2026-11-06/README.md).
They do **not** establish complete replay determinism or a balance win rate.

`move_enemies` now snapshots starting positions and processes attackers in
X/Z/Y order, with entity identity only as a final coincident-position fallback.
Per-attacker immediate damage, interception and the frame-start live-gate snapshot
remain intact. It does not aggregate damage or switch later attackers to hero
pursuit in the middle of the pass. A temporary vector is built once per pass.

Cinder aura alone retains boss-first and distance priority, adding Z/X/Y geometry
and then entity identity for exact ties. The shared targeting order of other
spells is unchanged. This preserves Cinder cooldown, radius and damage formulas.
Different noncoincident tied targets no longer exchange critical rolls merely
because unrelated components or entity allocations change their storage order.

## Actual red/green evidence

- Gate negative: real `move_enemies`, three stationary siege attackers with damage
  1/1/1000, gate HP 11.94 and 1/60 delta. Inserting/removing an unrelated component
  changes raw query order while IDs, positions and stats stay fixed. Old code
  fails exact HP-bit equality: 1086523963 versus 1086523965. One actual failed test,
  exit 101, in `red-release.log`.
- Gate correction: `green2.log` records 115 combat tests passed, zero failures.
- Cinder negatives: real `cast_learned_skills` dispatcher, only rank-four Cinder,
  explicitly ready cooldown, the two exact positions recorded at update 47221.
  Both squared distances have actual f32 bits 1072252846. Real strike generation,
  pooled strict-target carriers and `projectile_hits` produce two strikes, one
  cast, cooldown 0.5, one critical and one ordinary hit, and four RNG draws.
  Storage-row relocation and reversed entity allocations each exchange damage:
  HP [9976.3,14958.525] versus [9958.525,14976.3], same final RNG state
  16263334312487546840. Both desired-equality tests fail before the Cinder fix.
- Final full library run: **737 passed, zero failed, one existing ignored test**.
  All four Cinder controls now give HP [9976.3,14958.525] with the same RNG state.
- Selected changed Rust files pass formatting checks. Release all-target strict
  Clippy and release `balance_sim` build exit zero. Full source/asset/vendor
  manifests match before and after the sequential validation job.

Intermediate failures are retained: the first cold debug build was deliberately
cancelled before running tests to use the existing release dependency cache;
its log is not a test result. The first gate correction added a seventeenth
Bevy system parameter and failed compilation (`green.log`). The final version
uses a function-local position snapshot and has no extra system parameter.

## Scope

The first previous gate difference reconverges when the wall falls at update
25465; it is not presented as the cause of the later Cinder contact difference.
The Cinder fixtures reproduce that later exact-distance tie independently. These
are controlled ECS tests, not rendered/input-based player acceptance. They do
not test learning UI, cooldown aging over time, every skill, overlapping targets,
all possible asynchronous allocation histories, or a full route. Coincident
positions still use identity as a final tie-breaker. A new immutable independent
full pair is required before using this binary as a repeatable balance baseline.

`provenance.json`, `tracked-source.patch` and `new-cinder-order-tests.rs` describe
the source change from the recorded base commit. The release simulator is hashed
but not stored in Git. Logs, input manifests and terminal unit receipts are
included and covered by `manifest.json`; run `python3 verify.py` here or from any
working directory. The verifier validates archived evidence, not a new execution.
