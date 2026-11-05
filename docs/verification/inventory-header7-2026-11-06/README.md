# Isolated native equipment-title OCR — 2026-11-06

Actual86 C6 R2 interruption is preserved in sibling vortex-chapter6-r2-2026-11-06 (commit c854e4f). Original fullheader English+Chinese OCR returned `EQUIPMENT "了` twice on the actual seven-item image; strict numeric inventory_count correctly rejected it. The image visibly reads EQUIPMENT · 7. No character substitution is justified or implemented.

Measured fixed title crop200x32+210+100, grayscale inverted,200% resize,white10px padding, English singleline modes7 and13 both read `EQUIPMENT + 7`. The helper now appends these observed bounded title strings to existing fullheader rows. The numeric parser and ALL-owned identity/count assertions are unchanged. Any valid count disagreement still fails; no digit whitelist, expected count, substring, or nonnumeric mapping is used.

Offline audit covers all38 committed inventory-named PNGs plus the actual failure and current split-panel fixture (40 total). Only two genuine seven-item images changed from unreadable to7. Every previous valid count stayed unchanged;8 closed/battle images stayed unreadable. Raw perimage before/after rows are in holdouts.jsonl. Separate actualbattle,market,recruitment negatives and countconflict/nonnumeric/extra-word/spatial negatives pass.

Docker default unittest discovery of all five desktop test files:93PASS,99.666seconds,exit0; exact command/elapsed in test-process.json and complete tests.log. These are offline OCR/parser proofs, not a completed C6 route. No C6 restart, Cargo, game source or package change occurred.
