# Battle HUD numeric OCR — 2026-11-05

The fixed b1d7dd1 package r2 natural forging and chapter6 routes stopped after243.11s and270.32s respectively. Their first market opener observed ordinary battle, but general OCR read the small LEVEL5 caption as LEVELS. These are harness interruptions, not game losses or victories; neither reached natural crafting or campaign settlement.

The two original failed screenshots are archived unchanged. `ocr-before-after.txt` records the measured general OCR, narrowly cropped numeric OCR, actual countdown and1× speed. Forge: LEVEL520/132XP,21:32; chapter6: LEVEL59/132XP,21:23. No level/XP partition is inferred.

The classifier only requests numeric pixel OCR when a LEVELS+XP candidate is in the precise lower HUD region. Known modal/choice/terminal/panel checks take priority. The actual pixels must match the exact LEVEL+decimal+/decimal+XP alphabet; failure stops input. No S-to5 text repair or fuzzy lives match. Public opener still separately verifies countdown and1× speed from the same image.

54 tests passed in the isolated runtime image, including both actual screenshot regressions, modal precedence and rejection of nonnumeric fallback. See `tests.log`. A complete fresh runtime route remains required; these screenshot tests do not prove natural forging or chapter completion.
