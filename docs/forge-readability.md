# Forge reward readability

The selected recipe shows two or three ingredient portraits from the existing equipment and recipe atlases. Each portrait has its localized name and an explicit owned or missing label. Green recipe checks mark available crafts. Gold checks mark recipes already forged. Availability uses the same inventory predicate as the forge action.

Portrait equipment panels use the available height. Short landscape panels use smaller output and ingredient art while retaining at least 40 pixels for scrollable effects. The settings launcher is hidden while equipment is open. Added recipe bonuses precede inherited effects, and the existing detail scroll exposes longer descriptions.

Seven controlled native cases cover both advanced chains, English and Chinese, desktop, 320/360-wide portrait and short landscape. They perform five purchases, three crafts, material consumption, duplicate prevention and panel closure through production handlers. Inventory counts and all 29 recorded attribute fields match the previous baseline. All 42 final screenshots were reviewed through independent JPEG decoding. The library suite passes 786 tests with one existing ignored test, and strict release and capture Clippy pass. [Current evidence](verification/forge-materials-2026-11-08/README.md).

The fixtures inject stock and currency and drive interaction components. Naturally earned equipment and forging have separate historical checks. The current standalone package from `6f24fe7` precedes this material UI change. [Earned crafting](earned-forge-probe.md), [current package](desktop-release.md).
