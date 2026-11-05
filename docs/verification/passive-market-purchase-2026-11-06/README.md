# Single-click passive market purchase observation

Actual oneclick delayed-feedback proof is committed at c352cad in sibling overlap-buy-probe-2026-11-06. This change narrows that diagnostic into one shared public observer for C6 and Forge. It never uses a held fallback or repeats the purchase click.

Both callers retain pre-purchase exact offer and wallet proof. The shared function dispatches one existing Buy click, then passively screenshots the market for at most20seconds clipped to the unchanged visitdeadline, with a finite20observation cap. Each sameframe requires current BLACKMARKET header and independentHUD/header walletagreement. Exact expected debit returns that actual success frame. Unchanged wallet requires same normalized exact name/category/index/price. Wrongdebit, conflictingwallet, stockchange without debit, unreadable/unknownpanel, or timeout fail. Original fullowned identity/deduction/resume checks and original budgets remain unchanged.

Regression coverage includes delayed336→176 afteroneinput, genuine wrongamount, HUDconflict, stockreplacement withoutdebit, unknownpanel, alreadyexpireddeadline/noinput, OCRcrossing observationdeadline, unchangedwallet exhaustion withoneinput, and the two actual committed native probe frames. Forge imports the identical shared observer; neither caller performs another immediate postpurchase screenshot before the verified successframe is archived.

Default Docker unittest discovery PASS103tests (see full tests.log and exact test-process.json command/timing). These are source/unit/nativepixel regressions. No new C6/Forge live route was run after this helper change, so no new complete campaign or craft claim follows.

Final source boundary: full103PASS ran before deleting only the unused Forge verify_deduction import and replacing two unused raw return bindings with _. After that cleanup the entire Forge unittest module passed; cleanup-focused.log/cleanup-process.json retain exact count/timing. source-hashes.json pins all four final Python source/test files. No gameplay behavior or budget changed in that cleanup.
