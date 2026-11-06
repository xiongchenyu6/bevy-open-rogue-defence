# Inherited Frost attack market splash: real contact-chain ordering regression

The [previous full repeat](../post-glacier-repeat-2026-11-06/README.md) retains a
late Frost Attack/card+market-proc target difference. Read-only trace linkage
finds parent contact 93225 at update 94474 and an actual f32 equal-distance pair:
both planar squared distances are 1.8549002408981323 (bits 1072524639). Entity
ordering reverses across worlds and the already-created child at the end of
that update occupies a different neighbor's position. Exact extracted frames,
contacts, distance reconstruction and analysis-only scripts are retained in
`trace/`. There is no explicit parent->child provenance or target-lock field in
the old actor trace: the historic linkage uses source/profile, 30% base damage,
geometry and child location consistency, not an invented logged spawn edge.

## Actual red/green regression

The fixture applies actual SerratedDisc item behavior and queues a controlled
incoming pooled contact using the actual FrostOracle inherited profile and
FusionAttack component. Two real `projectile_hits` passes perform parent damage,
Frost contact control, actual market child creation, strict target locks and
child damage. It does not directly inject children, HP outcomes or a splash
sorting result. This is an inherited-contact boundary test, not the full hero
emission/cadence or shop purchase UI.

Three pairs run both worlds before comparing actual HP/RNG:

- Same IDs and geometry, reversed cosmetic archetype row order: passing control.
- Exact recorded equal distances, reversed entity allocations: valid old-code
  failure, HP [9970,14947.5] versus [9947.5,14970], same RNG
  5218726287330236675. `red2.log` records **2 passed, 1 failed**, exit 101.
- Unequal distances with reversed allocations: passing distance-priority control.

Each world checks the actual item ratio, the incoming Frost/card profile,
primary-only damage in pass one, inherited Frost control, two real locked
market+card child carriers, one actual child critical and one ordinary hit,
six parent/child critical/proc RNG draws, no repeated source damage, carrier
retirement, no recursive splash and no kills.

The production change retains distance first, then compares Z/X/Y before the
existing entity fallback for exact ties. It changes the shared nearest-neighbor
helper used by market splash and bond proc chains, preserving source exclusion,
three-meter radius and six-neighbor limit. Existing radius/cap coverage and the
new unequal-distance control remain passing. All four equal-distance printed
controls now give HP [9970,14947.5] and identical RNG; both unequal-distance
controls give [9947.5,14970] with that same RNG.

Full release library validation: **743 passed, zero failed, one existing ignored**.
Selected-file formatting, strict release all-target Clippy and release simulator
build exit zero; source/asset/vendor manifests match before and after. The final
supervising unit is terminal inactive/success/exit-zero.

## Retained invalid attempt and limits

The first attempt in `red.log` is **not valid bug evidence**: all three cases
stopped at a mistaken fixture assertion expecting exactly 30.0. The actual f32
100 * .30 product is 30.000002. Only that fixture expectation was corrected to
the exact f32 product; `fixture-guard-correction.patch` and the failed unit state
are retained. HP/RNG pair comparisons stayed exact; no epsilon or replay
comparator tolerance was introduced. Production ordering was unchanged for both
red attempts; the actual desired-behavior failure is the subsequent red2 pair.

The local fixture does not prove all pooled histories, simultaneous parent
batches, high-speed travel, all proc chains, every target configuration, full
GamePlugin scheduling or whole-run determinism. Coincident coordinates still use
entity identity as final fallback. A new immutable independent full pair remains
necessary. This correction has not been added to the already delivered bdd SDK
candidate. Earlier full-run negatives remain scoped to their own builds.

`verify.py` validates archive bytes and recorded red/green outcomes; it does not
run a game or prove global equivalence. The simulator is hashed in provenance
but not stored in Git. The trace subset is not a complete raw replay archive.
