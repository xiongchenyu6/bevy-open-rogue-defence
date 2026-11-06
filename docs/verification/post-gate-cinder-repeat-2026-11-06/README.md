# Full replay after gate/Cinder correction: still different

Frozen source `7e87900e2eb59dfe91de0a0020d49dd1656e4e2d`; release simulator
SHA-256 `558f626d5c5055914296bfe1ece2a0f409493c42bce5323919cbe52bd7997996`.
191 actual asset files (the console's 201 asset paths also counts directories).
Fresh simulator tests: 6 passed, zero failed; build exited zero. Source, assets,
vendor and runner/comparator input manifests match before/after the build and
were checked through both cases. All frozen input files were read-only.

This uses the same controlled fixture as the [earlier negative](../balance-repeat-2026-11-06/README.md):
headless real GamePlugin, fixed 1/60 delta, single-thread schedules, fresh isolated
saves, chapter index 5 with synthetic prior clears and zero permanent upgrades,
requested seed BEEF (actual 48880), requested recruitment starter 2, stationary
active card-zero policy, default behind position, no pressure override. It is not
an earned campaign or rendered/input-based acceptance run. Asynchronous asset
and scene loading remains present. Final heroes can change through recruitment.

| Actual terminal result | A | B |
| --- | ---: | ---: |
| Outcome | victory | victory |
| Updates | 100903 | 99620 |
| Game seconds | 1559.256 | 1539.005 |
| Level | 43 | 42 |
| Kills | 2150 | 2137 |
| Deaths | 0 | 0 |
| Gate breaks | 38 | 43 |
| Skill rank total | 31 | 29 |
| Attacks | 11572 | 12276 |
| Damage skill casts | 483 | 484 |

Both simulation processes exit zero with valid receipts and actual terminal
victories. The strict checkpoint comparator exits **1**, first differing at
update 82800 (attacks 9841/9834). Streaming analysis consumes all 100903/99620
trace rows and 81893/81117 contacts. Recorded numeric values first differ at
update **81007**, enemy HP. The first nonidentity contact difference is zero-based
46770: Frost spell skill 14, same carrier and RNG roll but a different target.
No recorded numeric difference appears before update 81007 in this pair; this
is not proof that all hidden state or physical target binding was identical.

## Glacier contact evidence and next test

The first changed contact itself is noncritical and does equal damage, so it
alone does not establish why HP differs. The preserved selected window supplies
a stronger observation: physical target entity bits 403726901544 has the same
position and pre-hit HP 106910.75 in both cases. At contact 46770 in A it takes
3710.646484375 (ordinary), finishing at 103200.1015625. At contact 46772 in B it
takes 6493.63134765625 (critical), finishing at 100417.1171875. See the exact raw
fields and associated frame actors in `glacier-contact-window.json`.

Read-only source inspection finds Glacier14 area pulses still sort targets by
entity identity, while StormMatrix pulses sort by geometry. A reverse-allocation
actual producer regression is the next causal test; it has **not run** here.
`glacier14-readonly-suggestion.txt` is explicitly an unexecuted suggestion, not
passing evidence. Same-ID cosmetic relocation is a useful passing control because
identity sorting already handles that case. No further production correction or
balance tuning was applied during this experiment.

Earlier gate and Cinder local red/green tests remain valid within their scope;
this full negative means they did not establish global determinism. The old
75c win/loss flip remains a separate historical negative, not causally explained
by this later run. This one fixture does not estimate general difficulty.

## Evidence retention

Process receipts, exact final checkpoints, comparisons, initial divergence
contexts, scripts and input manifests are archived here. `local-raw-manifest.json`
hashes the large raw traces and stderr retained under local `target/`; those raw
files and the simulator binary are **not stored in Git**, so the local paths are
not a portable full replay archive. The selected contact window is an extracted
subset, not the complete pulse or full trace. `verify.py` validates the archived
bytes and recorded negative result, not a new replay or all internal game state.
