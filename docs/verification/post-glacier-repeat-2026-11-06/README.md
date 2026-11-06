# Full repeat after Glacier correction: late exact-state difference retained

Frozen source `3d42d0be0b5d1df5c4e80a420da202194c6a49c4`, simulator SHA-256
`db3d1fc93dc987a5227f1b2b67ee2bb42dd05e8ea8446ff18e32a750cf1c6274`.
The exact release binary from the completed [Glacier validation job](../glacier-order-2026-11-06/README.md)
was copied without another compilation. Its validated source/asset/vendor input
bytes still matched after the source commit; 191 asset files, runner/comparator
and required campaign source were frozen read-only. Binary metadata records that
reuse and the build input manifest hash. No new six-test simulator unit run is
claimed here; the linked validation ran 740 library tests, strict all-target
Clippy and the release simulator build.

Both bounded cases reuse the same diagnostic configuration: fresh isolated
saves, synthetic chapter-5 prior-clear fixture with all permanent upgrades zero,
requested BEEF seed (actual 48880), recruitment starter portrait 2, stationary
active card-zero/market policy, behind position, fixed 1/60 updates, single-thread
schedules, no renderer/window/audio output and no pressure override. Asynchronous
asset/scene loading remains. This is not an earned campaign or public UI run.

| Actual final metric | A | B |
| --- | ---: | ---: |
| Outcome | victory | victory |
| Updates | 100727 | 100735 |
| Game seconds | 1560.317 | 1560.318 |
| Level / kills / deaths | 43 / 2138 / 0 | 43 / 2138 / 0 |
| Gate breaks | 34 | 34 |
| Attacks / damage skill casts | 12408 / 465 | 12408 / 465 |
| Skill rank total | 31 | 31 |
| Gold / wood | 5046 / 187 | 5046 / 187 |
| Final RNG state | 4822865713308169501 | 5815998724421650042 |

The terminal gameplay counters agree, but **exact recorded states do not**.
The strict checkpoint comparator exits 1, first reporting RNG-state difference
at update 95400. Streaming analysis consumes all 100727/100735 actor rows and
96198/96172 contact records. First recorded numeric difference is update
**94475**, enemy HP. First nonidentity contact difference is zero-based **93232**,
at game elapsed 1518.55322265625: Frost Attack, skill index None,
market_proc=true, card_proc=true. The same carrier/critical roll resolves a
different target; exact profile, IDs, HP and RNG fields are preserved in context.
This report does not establish which producer/earlier physical binding caused it.
A contact projection can miss earlier target changes when HP/damage fields match.

No recorded numeric difference appears before update 94475 in this pair. That is
not equivalence of all hidden state, actor identities or physical target binding.
The earlier Glacier local regression remains valid within its scope; this run is
not global determinism, general balance, or an explanation of the old 75c
win/loss flip. No comparison tolerance was widened and no unfavorable result was
replaced. The new late card/market proc needs its own actual producer regression.

The manager finishes successfully because it completed both valid cases and
analysis; comparator exit 1 is a retained **negative**, not a passing replay.
Large raw JSONL/stderr files are retained locally under the exact paths and
hashes in `local-raw-manifest.json`, not in Git. The archive contains extracted
initial contexts, receipts, manifests and scripts, not a portable full trace.
`verify.py` checks these archived bytes and the recorded negative; it does not
run the game or prove all internal state.
