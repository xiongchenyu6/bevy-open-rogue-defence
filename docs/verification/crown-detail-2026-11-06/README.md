# Crown rank 6: remove the displayed downgrade

This evidence supports a narrow UI correction: a Crown-boosted skill at rank 6
no longer previews a base-rank downgrade to 5. It does **not** establish that the
preview includes every outgoing damage modifier. In particular, the displayed
650.8 appears to be the raw skill carrier value; inclusion of Crown's additional
30% final outgoing bonus remains an open, separate audit. Do not read this as
“all active bonuses are correctly reflected,” even where the existing UI copy
says that values include active power bonuses.

## Exact source and completed checks

All three recorded units ended inactive with `Result=success`, `ExecMainStatus=0`.
Their before/after input maps contain 390 source/assets/Cargo paths; the only
changed digest is `src/game/skill_details.rs`. The after map also matches the
completed after-input map. Exact source copies are retained. The verifier
reverses only four expressions and requires byte equality with the before file:
next rank cannot fall below current rank, and the displayed denominator cannot
be below current rank, in simple and detailed views. No damage, attack or balance
formula was changed.

The extracted `skill_details/tests.rs`, Crown capture fixture, `capture.rs` and
`capture/attacks.rs` snapshots are included with their input-map digests. These
same test/diagnostic sources were present in both builds; the release-phase
observer is capture instrumentation, not a production attack change.

- Before: the actual new regression exited **101**, one behavioral test failed,
  with `Rank 6 → 5/5` retained in the raw failure. This is not a compile failure.
- After: the regression passed. The complete library had **746 passed, 0 failed,
  1 ignored**; strict all-target release Clippy exited 0.
- Base ranks: **384 observations** = 32 skills × 2 languages × ranks 0–5.
  Each observation contains both simple and detailed strings (768 strings per
  stage). Independent raw-log parsing confirms the complete identity set and
  exact before/after string equality, not just a comparison-report boolean.

## Eight actual native UI cases: desktop and portrait

Before/after × English/Simplified Chinese × desktop1280×900 / portrait720×1280
each completed240 simulationupdates. Desktop saved all32 requestedPNGs percase;
portrait saved exactly2 requestedPNGs percase. Root actually viewed frame99 in
all8cases, including localizedheader/damagenumbers/threeURcards. Root did not
view frame179; both99/179 from eachcase are archived (16 selectedPNGs total).
The existing details panel has a scrollable body; below-fold text is not evidence
that it is unreachable. Fulltooltip fit/scroll interaction is not tested here.

| Actual UI log update | Before EN/ZH | After EN/ZH |
| --- | --- | --- |
| 100 | Rank 6 → 5/5; damage 650.8 → 573.1 | Rank 6 → 6/6; damage 650.8 → 650.8 |
| 180 | Same observed downgrade | Same-rank preview remains unchanged |

The native fixture grants **prerequisite rank 5**, not rank 6. It controls market
stock, paused state, elapsed time and 50000 kill-wallet/gold resources to make
this case reachable. It then presses the actual Crown offer's Interaction,
checks the real production purchase's exact kill debit (skill gold unchanged),
checks actual rank 5→6 and localized purchase feedback, closes the actual market,
and sends KeyG through the game input resource. The real draft contains three
distinct UR26 evolution paths and actual localized selection-detail text. It does
not inject a fake UI string or directly set native rank 6.

This is controlled Interaction/KeyG-resource reachability evidence, **not**
physical mouse/keyboard input, naturally earned stock/currency, persistent
progression, a public campaign or a Steam/hardware acceptance test. The exact
wallet assertion and branch checks are in the archived shared fixture and were
executed by each of eight successful native processes; the log does not print numerical
wallet endpoints, so none are invented here. These cases do not select/complete
an evolution route or prove its final combat damage.

## Portable verification and retained bulk

Run `python3 verify.py` without local target files, assets, Rust, GPU or binaries.
It checks the exact archive fileset and hashes, the one-file/four-expression
source delta, shared source digests, expected negative 101 and final successful
receipts/unit states, all 384 before/after base observations, actual eight native
purchase/detail/damage records at updates100/180, 240-update completion footers,
and the exact 16 selected PNG hashes/dimensions. It separately preserves the
30%-bonus preview limitation; equal 650.8 values do not prove full buff accuracy.

All original logs and receipts/drivers are retained, with logs losslessly gzipped
and uncompressed byte hashes in `raw-log-manifest.json`. All 136 original native
PNGs and two capture binaries remain in the local target tree; their paths,
sizes and SHA values are recorded in `local-full-frame-binary-manifest.json`.
Those omitted local files are not a portable all-frame or executable rerun
claim. The scoped `.gitattributes` disables text conversion for the whole archive.
