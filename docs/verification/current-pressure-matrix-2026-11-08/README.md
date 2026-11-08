# Current stationary-policy pressure samples

All24 requested cases completed valid uncensored terminal runs with process exit0 and no timeout. Manager invocation4f683c309ca549b898e24835df0b8d66 ended success/main0 in1414.7615 seconds. The frozen simulator binary SHA256 is a6099bc9afc343c6840bbcb558182dfd8a543d2d248e7f7b83704302a1a5f2da, built from game source064fe0a. The211 compiled inputs, assets and runner pins are retained.

| Observed group | Active fixed policy | Recruitment-only policy |
|---|---|---|
| Chapter index0 | 6 wins,0 deaths per case | 6 losses,276.278–740.905 seconds |
| Chapter index5 | 4 wins,2 losses | 6 losses,68.742–185.995 seconds |

The first-chapter active minimum alive-health fractions are0.930507–0.992954. Late-chapter active deaths per case in recorded case order are2,0,0,0,3,3, with health fractions reaching nearly zero in some cases. Two active failures end at844.765 and1383.532 seconds. Both request starter2, but later actual hero changes prevent a final class-strength attribution.

Across12 active cases, final learned skills range8–9, stored-rank totals9–20 and ranks at or above base cap5 number0–2. Four cases ever show base cap5 at a checkpoint. The earliest first-cap observation interval is(871.801,900.719] seconds. These samples do not reproduce early universally capped skills. Cap5 is a baseline-rank statistic rather than UR evolution completion, and Crown rank6 remains valid.

## Policy and fixture

The24 combinations use two requested seeds1/BEEF, three selected basic starters, chapter indices0/5 and active/recruitment-only policies, fixed behind-gate stationarity and zero permanent upgrades. Actual run seeds are2/48880. Initial hero IDs map starters0/1/2 to2/1/0. Later compulsory recruitment can change heroes. Active policy retains blind card0 choices, real fixed market actions and the existing simulator policy. Recruitment-only still makes compulsory recruitment choices. Late chapter access is synthetic, not earned campaign progression.

This is a descriptive two-seed sample under fixed policies. It is not a causal before/after study, general win probability, optimal player strategy or balance acceptance. Every original24 JSONL and24 stderr file is preserved losslessly, alongside summaries, commands, receipts and all terminal/30-second curve analysis outputs. No simulation was rerun for this archive.

Run `python3 PATH_TO_ARCHIVE/verify.py PATH_TO_ARCHIVE` to independently recompute terminal identities, outcomes, first heroes/seeds, censor checks, ranks and pressure metrics from raw checkpoints. Add `--local-root PATH_TO_REPOSITORY` for read-only hashing of the frozen local binary/assets/runner. No simulator starts in either mode.
