# Native Ember/Storm release candidate: bounded timing evidence

Source basis is `d36e0e5` plus the explicit tested patch snapshots retained here;
the future final commit is not inferred. Production changes are in `combat.rs`
and the new narrow `combat/ranged_release.rs`; test adaptations/new tests are
separately included. All final392 source/assets/Cargo input hashes match their
completed frozen map. No production tuning or archive-time source edit occurs.

Only native Ember/Storm release phases change (.25/.28 of the fitted action
window). Fusion/melee paths and accepted Sword .60 / Daggers .50 are controls.
Counters, attack proc opportunities and commit-time RNG remain at commit;
projectile rental and actual impact happen later. Aim/damage are frozen at
commit, origin follows the current hero, pause freezes pending age, and an
invalid/dead/stunned/changed owner cancels it. These are actual gameplay timing
changes: general old/new impact, crit binding, target HP and campaign outcomes
are **not** asserted identical.

## Actual completed verification

Final focused tests:14 passed. Full release library:**756 passed,0 failed,1
ignored**. Strict all-target release Clippy, native capture build, normal
Ember/Storm/Sword210-update cases, both old/new numeric trace cases and four
before/after portrait supplements all exited0. Original unit/process receipts
and complete lossless logs are included. No hardware/Steam or physical input
acceptance is inferred from these controlled native fixtures.

The original desired-contract test exited101 on an actual immediate emission,
not a compile error. Historical iterations remain distinct:

- `r1-immediate-fixture-failures`: focused10 passed, full741 passed/11 failed.
  Old immediate-projectile assumptions were adapted to real deferred release
  while preserving arrow counts/bounces/aim/counters and explicit untouched
  Frost Oracle immediate-route controls.
- `r2-elite-radius-fixture-failure`: a “no early contact” fixture accidentally
  retained elite radius and received legitimate partial-step damage. Its target
  configuration was corrected; no damage assertion was removed.
- `r3-visual-age-candidate`:755 passed, native3/Clippy0, yet source-time inference
  varied between regular cycles. This technically green candidate was not final
  visual/timing acceptance. All its original source/maps/logs are retained.
- `r4-large-frame-metadata-fixture-failures`:14 focused passed, full750 passed/6
  failed. Corrected initial full-frame age could legitimately release in the
  same update; metadata helpers retained pre-release obligations and exact final
  counts/RNG rather than requiring every intermediate state to stay pending.

Final initial age uses the original movement allowance (`travel` when present,
otherwise full current delta), aligning normal and backlog animation age while
preserving existing catch-up ages. The actual first partial step, subsequent
full step, real damage, pause/cancel and1×/8× accounting assertions are retained
in the final source/tests. This is not a claim of artist-authored calibration.

## Numeric release observations and conditional inference

`native-comparison.json`, `phase-inference.json` and raw trace logs preserve every
row, changed value and comparison. Last-visible active carriers are not an exact
spawn callback; a same-update retired carrier is not observed. Observable Graph3
seek restarts are animation cycles, not directly logged commit-counter events.

| Trace fixture | Old first-active count | New count | Old/New contacts in210-update window |
| --- | ---: | ---: | ---: |
| Ember |24|24|24 /23|
| Storm |13|13|13 /13|

The final Ember arrow is observed at update205, x−1.9399614, vx18.44. Under the
source-confirmed stationary fixture, nearest ordinary target surface is x2.04.
Even **maximum** full1/30 motion puts it at x1.1333719 by210 and x1.7480386 by
211 (one actual screenshot-wait update); first possible full-step crossing is
212. Hit-stop can only reduce this1× movement bound. Lifetime1.4s exceeds this
short flight. Raw last contact is203 with no unassigned tail hit. Thus24 emitted
and23 observed hits are consistent with the last arrow still in flight at the
fixed horizon, not evidence of a lost attack. This is a source/log trajectory
bound, not a directly logged frame210 Transform or an extrapolated hit count.

All23 shared Ember and13 shared Storm resolver records have identical logged
base/scaled/raw/final damage, crit roll/multiplier, RNG transitions and HP-bit
values/profiles in this fixture. Differences are preserved: Ember progress_bits,
projectile identity and elapsed_bits differ on23 rows, outer_ordinal on1;
Storm progress/identity/elapsed differ on13. Contact update timing changes.
No equal final endpoint HP or broader determinism claim follows.

For eligible straight first-active observations, the conditional estimate is:

`source seek − (distance from fixture origin / velocity magnitude) × playback speed`

Source snapshots validate hero origin(−2,0,0)+Y.25, Autoplay removed, stationary
controlled targets atx3, spawning disabled and fixed1/30 capture. Guards require
one Graph3 clip and one visible release per observable cycle, positive collinear
flight before contact, at most one update of initial travel; multi-catchup is
outside this inference. Every rejected observation/reason remains available.
Velocity/playback constancy and no Automatic melee-audio override are explicit
fixture assumptions. The result is derived simulation source time, not a hand,
string/socket, pixel or device-latency event.

- Old immediate: approximately0 source seconds.
- R3 all-cycle range: Ember0.275890–0.398770; Storm0.265438–0.322473.
- Final regular range: Ember0.266666639–0.266666712; Storm0.261333349.

The reference clip fractions imply approximately0.2666667/0.2613333 seconds.
No tolerance was forced before observations; all unfavorable R3 dispersion is
retained. This does not establish universal high-speed authored pose alignment.
Sword retains exact80ATTACK_TIMING and210RELEASE_PHASE row counters versus the
prior close-up control, with zero Last-visible short-lived carriers expected.

## Actual pixel scope

Exactly seven root-viewed originals are delivered: current near Ember14/Storm20;
before/after portrait Ember14; before/after portrait Storm20; after portrait
Storm23. Root saw clear delayed flight. The large overhead cavern geometry is
present in both portrait versions; **overall portrait visual quality is not
accepted by this proof**. The launch is not socket-perfect (crystal near the held
staff), and no artist event has been calibrated. Other frames remain local,
listed by hash, not attributed to root review.

These controlled launch fixtures are not a new full campaign sample. Previous
24-case stationary balance measurements belong to their earlier frozen
executable; their outcomes must not be relabelled current timing acceptance.

## Portable verification and bulk boundaries

Run `python3 verify.py` without target paths, models, binaries, Rust or GPU. It
checks exact file hashes/fileset, final input/source digests, historical/final
process outcomes, independently recomputes old/new hit fields/counts/shared
prefix differences, horizon bound, all inference values/spreads, unchanged Sword
counters and seven selected PNG dimensions. It does not replay the omitted
binary or automatically evaluate subjective pose quality. Original comparators
and their richer measured rows remain included.

All original logs are losslessly gzipped with raw byte hashes; all iteration
receipts/drivers/reasons/source snapshots are preserved. Full PNG sets and native
executables are **not** copied; `local-full-frame-binary-manifest.json` records
retained local paths/sizes/hashes only. Local `.gitattributes` disables all text
conversion. The archive is portable evidence verification, not a self-contained
runtime distribution or global RNG/HP-equality proof.
