# Observed owned-check marker

Natural Forge R6 bought Arc Lens for180 with wallet304→124 and Echo Core for300 with wallet356→56. Both actual inventory ownership observations and acquired materials remain in the result. It then failed recipe readiness verification because both actual OCR statuses were ¥ Owned. Root reviewed the original ready screenshot, showing two owned ingredients and an active Forge equipment action. Crafting and restart were not completed.

The consumer accepts ¥ as one additional observed check glyph before exact Owned. Exact ordered material names, localized recipe title and active action checks remain unchanged. The two-source patch is retained against fae70b12eefb3acc32f0fa51dda70cd0e7afcdde.

An actual offline immutable Docker replay calls real read_recipe_materials, title/action crops and verify_recipe_selection on the original ready PNG. It passes with [SSR] Astral Prism, Arc Lens and Echo Core, both ¥ Owned statuses and Forge equipment. This is retrospective pixel verification without game input. Its actual process receipt and raw output are retained separately from the failed natural result.

The final actual offline seven-module suite passed122 tests in94.986 unittest seconds, with process exit0. The proof covers the observed marker parsing. Live crafting, consumed ingredients, Prism ownership and same-state restart remain separate work.

Run `python3 PATH_TO_ARCHIVE/verify.py PATH_TO_ARCHIVE` for exact archive/raw hashes, actual receipts, source snapshots and fixture checks.
