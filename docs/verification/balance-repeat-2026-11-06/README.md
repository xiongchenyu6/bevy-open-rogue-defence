# Current balance repeat: full-run divergence retained

Source: `3f19ae7b3cecf8c4bd181a727bf1e34abddc00bd`. Release simulator SHA-256:
`a83bb93da477aee5267e5fe60829647b513c40145903c0d56a7d4e5ffca67207`.
Fresh release simulator tests: 6 passed, 0 failed. Build and all six simulation
processes exited 0. Frozen source inputs matched before/after; 191 asset files.

This is a controlled headless real-GamePlugin experiment, not public-input
acceptance or an earned campaign playthrough. Each case uses a fresh synthetic
prior-clear fixture (chapter index 5, all six permanent upgrades zero), requested
seed BEEF, actual seed 48880, requested starter 2, stationary active greedy card
zero policy, default behind position, fixed 1/60 update, single-thread schedules.
Rendering/audio output are disabled, but asynchronous asset/scene loading remains.
The requested starter is a recruitment index, not a fixed final hero identity.
No pressure override or balance tuning was applied.

| Pair | Recorded updates per side | End | Recorded numeric/contact comparison |
| --- | ---: | --- | --- |
| Early | 7200 / 7200 | cap; elapsed 117.769 each | equal; 138 contacts each |
| Long | 21600 / 21600 | cap; elapsed 352.754 each | equal; 1597 contacts each |
| Full | 100566 / 98487 | both victory | **different**; 76424 / 73741 contacts |

The two prefix results are censored, not completed replay equivalence. Full pair
A took 1554.939 game seconds, B 1536.514; kills 2162/2100, gate breaks 37/36,
hero deaths 0/0, skill rank totals 30/31. The same outcome hides substantial
state differences. This is not a validated balance baseline or win-rate estimate.

## Earliest observed differences

- Full pair raw enemy query order first differs at update 8535.
- First recorded numeric difference: update 25462, only gate HP bits
  `1094650247` versus `1094650248` (11.939826011657715 versus
  11.939826965332031). Previous actor records match apart from query order.
- First nonidentity projectile contact difference: zero-based contact 13067,
  update 47222. Same Flame skill 11 projectile, critical roll and RNG state,
  but different target with different HP. Raw context is preserved here.
- Strict checkpoint comparator exits 1; first checkpoint difference at update
  48600, attacks 1721/1744. This is expected failure evidence, not a passing check.

Source inspection finds immediate per-enemy gate subtraction in the raw query
iteration of `move_enemies`. Floating-point subtraction order is a plausible
cause of the first one-bit gate difference. This inspection alone does not prove
that it causes the later contact divergence or different terminal builds.
No production correction has been applied as part of this evidence commit.

## Evidence and limits

`outcomes.json` preserves exact final checkpoints; per-pair reports retain the
first differences and contexts. Comparisons consume every recorded row, preserve
all scalar/actor bits and actor multiplicity, and compare contact sequence.
Numeric projection removes actor entity IDs only; raw identity/query-order
differences are separately recorded. Hidden timers, target locks, armor and other
untraced internal state are not proven equal. Numeric/contact equality cannot
prove identical physical target binding where the trace lacks that information.

The approximately 1.4 GB of raw local traces/logs are **not included in Git**.
`local-raw-manifest.json` identifies their local target paths, exact sizes and
hashes; these paths are not portable archival evidence and require the retained
workspace. Source/frozen manifests and copied process results are included.
Do not interpret archived report JSON alone as a rerunnable full trace archive.
The original older 75c binary win/loss flip remains a separate historical
negative; this experiment does not establish its root cause.
