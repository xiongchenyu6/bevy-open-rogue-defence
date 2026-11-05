# Equipment bought through actual play

The read-only d2527eb Linux package loaded an isolated copy of actual earlier campaign progress:575shards and Strength upgrade5. Public X11 inputs selected the first chapter and StormSavant, verified4× speed, earned kills and opened the genuinely unlocked market. Initial stock contained real random equipment instead of five placeholders.

After further combat and a normal restock, ArcLens cost180kills. The actual wallet decreased455→275, and inventory showed one owned ArcLens with its range, spell-power and penetration effects. The attribute sheet was inspected; the forge catalog showed missing ingredients. No recipe was crafted. Returning to battle resumed the actual countdown979→965seconds. Normal quit returned0 and the copied campaign save remained575shards/Strength5. No stock, currency, kills, time or equipment were injected. [Evidence](verification/earned-equipment-2026-10-09/verification.json).

Ten native screenshots were inspected. Purchase feedback temporarily hides the market-header wallet, so the debit check cross-checked the visible HUD counter before and after. That UI defect is being corrected separately. Earlier failed OCR/modal attempts and the old first-stock defect are not successful acquisition evidence.

The tested public-input harness is[tools/desktop/equipment_runtime.py](../tools/desktop/equipment_runtime.py). It requires a read-only package at/game, helpers at/harness, writable output at/verify, the actual historical575/Strength5 save copied into/verify/state, and PACKAGE_COMMIT matching the package's full version.json commit. It starts its own Xvfb/Openbox and verifies manifest hashes. This narrowly scoped historical probe does not verify earned crafting, all builds, a complete expedition, physical sound or hardware performance.

## 2026-11-05 插画版独立包

`8f4084b` SDK 包的第六章真实存档路线已完成自然购买疾风靴：307战功扣160至147，装备栏显示实际图标与 `[R] Wind Boots`，关闭后倒计时1164→1155。Root查看了原生库存截图，并独立重跑像素OCR核对。[完整购买证据](verification/earned-market-2026-11-05/README.md)。本局后来因关闭黑市时新弹出的10级征募遮住时钟，自动脚本中断；这不证明完整通关或正常结算退出。当前包自然合成路线仍未复验。
