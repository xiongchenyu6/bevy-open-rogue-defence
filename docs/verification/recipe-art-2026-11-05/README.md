# Six distinct crafted-equipment illustrations

All six recipes now use their own original generated illustrations: Bloodreaver cleaver, AstralPrism orbital crystal, StormBastion shield, Suncoil conductor, NightCrown and WorldEngine. Crafted results no longer reuse ordinary ingredient icons. `RecipeId` selects the same validated3×2 atlas for recipe buttons and actual owned output. One411408byte RGBA PNG is shipped with provenance/prompts/manifest; it uses the existing enabled decoder, with no new dependency. Bitmap validation derives the six required names from the actual recipe enum and checks nonblank/distinct cells, hash, bounds and transparent gutters.

The new atlas is requested with the market art at startup and included in the normal readiness gate. An unavailable atlas prevents entry and has the same explicit retry path, rather than showing empty recipe art.

## Verified evidence

- Optimized full library663passed/0failed/1ignored; strict optimized all-targetClippy with-Dwarnings passed.
- Four native310update real-UI crafting fixtures: ENdesktop1280×720 phase0, ZHdesktop phase1, ENcompact667×900 phase1 and ZHcompact phase0. Each fixture buys five prepared ingredients through the actual UI, blocks crafting with missing materials, executes its three-recipe chain, checks actual inherited/extra combat statistics, material consumption, single advanced output, duplicate prevention and close/resume. Together they cover all six recipes in both desktop and compact layouts. Funds/stock are deliberately prepared in these fixtures; these are not natural earning or end-to-end campaign evidence.
- Two150update actual asset-reader failure/retry runs target exactly`ui/recipes/recipe-atlas.png`. Logs show25/26loaded, one actualNotFound, precisely one genuine retry (two total reads),26/26ready, blocked selection/clock until ready, then fresh hero selection and advancing battle. Temporary controlled asset I/O is used; no production readiness flags or source files are changed. The second bounded run captures the stable failed/loading UI because screenshot requests are asynchronous.
- Total1540native updates. Root inspected the recipe gallery, crafted outputs, Chinese desktop, both compact languages, actual failed/retry and restored battle frames. Screenshots establish layout/art; synchronous assertions establish recipe/state/bonus behavior. This does not certify overall art quality, natural progression, hardware performance or a release package.

`verification.json` is bitmap-only; `native-ui.json` records runtime scope. Original prompts/tool paths and source hashes are in`assets/ui/recipes/generation.json`. Current code, atlas and native binary hashes are recorded separately. The older32cf594 SDK candidate predates these icons.
