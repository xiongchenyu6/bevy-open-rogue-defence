# Forge reward readability

Recipes and crafted items now put their own added bonus before the inherited effects. Recursive ingredient effects remain intact. Localized footer guidance explains scrolling, and the equipment modal now sits above camp labels so they cannot cover the forge action or details.

Both existing crafting gameplay tests passed. The combined635-test library suite, strict all-targets Clippy and main/capture build passed. Four300-frame native fixture scenes covered both advanced chains, English/Chinese and desktop/compact layouts. Sixteen final screenshots were inspected: the new bonus remains visible before scrolling, and wheel messages reveal the inherited-effect tail. [Evidence](verification/forge-readability-2026-10-09/verification.json).

One compact capture hung during GPU startup. Its log is retained; only the missing case was retried successfully with a shorter runtime path. This does not establish the cause of the hang. The fixtures use injected stock and materials; actual earned purchases and crafting remain separate release requirements. The700d221 package predates this UI change.
