# Complete recorded replay match after equal-distance splash correction

Frozen source `acbdd2d75c814c6e05ac398f5b3e025dea1b4662`, simulator SHA-256
`cc31f1b4ecf07ebb96dee3c28c9479dec2e9641f72611a9b1b1b876f474fde84`.
The exact release binary from the completed [743-pass local validation](../frost-splash-order-2026-11-06/README.md)
is copied without recompilation. Source/asset/vendor input bytes still match;
build input manifest and completed job receipts are identified in binary
metadata. All 191 asset files and required runner/comparator/campaign files are
frozen read-only. No source or asset change occurs during either run.

## Actual complete result

Both independent fresh cases actually finish with victory at update 100735,
game elapsed 1560.318 seconds. Both have level 43, 2138 kills, zero deaths,
34 gate breaks, 12408 attacks, 465 damage skill casts, skill-rank total 31,
gold 5046, wood 187 and final RNG state 5815998724421650042.

- Strict checkpoint comparison exits zero: **57/57 checkpoints**, completed ends,
  status `complete_equal_replay`, not a capped prefix.
- Streaming actor comparison consumes **100735/100735 complete update rows**;
  all recorded numeric scalar/actor fields match after removing actor identities
  from the documented actor arrays. No missing row or first numeric difference.
- Contact comparison consumes **96172/96172 events**; the complete recorded
  nonidentity contact projection matches, including damage/HP bits, profile,
  critical rolls and RNG fields. No first nonidentity difference.
- Manager and both simulation processes are terminal success/exit-zero. Hashes
  of pinned source/assets/tools and binary are checked through both cases.

Identity is **not** identical: raw enemy query order first differs at update
5031, and raw contact identity first differs at zero-based event 11684. These
are retained in `comparison.json`; they have not been discarded or presented as
byte-for-byte replay equality. Actor projection preserves multiplicity, not just
sets, and the analysis consumes both entire sequences.

## Configuration and claim boundary

This is one controlled fixture: renderer/window/audio-output disabled real
GamePlugin, fixed 1/60 updates, single-thread schedules, asynchronous asset/scene
loading still present. Fresh isolated saves get a synthetic chapter-index-5
prior-clear fixture and zero permanent upgrades. Requested seed BEEF becomes
actual 48880; requested recruitment starter portrait 2, stationary active
card-zero/market policy, default behind position, no pressure override.
Recruitment can change the final hero. This is not earned campaign progression,
a public-input acceptance run, a different-platform SDK test or general win rate.

This proves a **complete match of the recorded projections for this pair**.
Unrecorded control timers, target locks, armor, equipment internals and other
hidden state are not proven equal. Contact logs do not themselves record every
physical target position; matching HP/profile fields cannot prove all target
binding identical when multiple actors share those fields. The result does not
establish global determinism across seeds/classes/routes or identify the root
cause of the older 75c win/loss flip. Earlier negatives remain valid historical
evidence for their own exact builds; comparison tolerances were not widened.

The next descriptive sample expands first-chapter seed/starter coverage with the
same immutable snapshot; it must not be pooled with older source versions as a
causal before/after comparison. No numeric tuning has been made from this pair.

## Retention

Receipts, exact final checkpoints, comparisons, nullable initial contexts,
scripts and input manifests are archived. Large full JSONL/stderr files and the
simulator are **not in Git**; exact local paths/sizes/hashes are in
`local-raw-manifest.json`. This is not a portable full raw replay archive.
`verify.py` validates archived bytes and reported equality scope, not a new run
or all internal game state.
