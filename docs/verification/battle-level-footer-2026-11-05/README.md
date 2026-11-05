# Merged public HUD footer — 2026-11-05

040ce5c r4 natural-forge route stopped after1455.05seconds on market-16-open-32.png. The actual battle was09:50 with a level24 caption; OCR merged the level/XP caption with adjacent footer buttons into one line. Exact bounded numeric pixels read LEVEL2484/474XP, actual clock09:50 and actual speed1×. Original screenshot, log, result and measured reads are preserved.

Only the raw spatial candidate changes from a complete-line match to an anchored LEVEL…X+P prefix followed by whitespace/end. The candidate still cannot accept battle without independent exact numeric-pixel validation. No raw digits are inferred or repaired. Missing XP, bogus prefix, XPP, XPjunk and known modal cases remain rejected.66 tests passed (`tests.log`).

The stopped route proved only natural Arc Lens331→151 for180 and owned count1. Echo Core, Astral Prism, full crafting/resume/quit and campaign completion are not established. The separate chapter6 route remains independent of this interruption.
