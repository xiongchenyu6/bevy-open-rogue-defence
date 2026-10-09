# Actual first-unlock Black Market placeholder stock

Package source700d221, publicUI firstchapter/StormSavant, copied actual earned575/T5 state, actual4x toolbar and actual3min unlock countdown21:58. No time/kill/stock/resource injection.

Viewed screenshot: target/package-equipment-oct9-r8/market-request-00.png. ActualUI21:03 (elapsed237s), Level5, wallet121, BLACK MARKET LV1, nextrestock2:04. All five real visible offers are identical [R/USE] Gate Patch120, precisely MarketOffer::PLACEHOLDER. No equipment available in this first stock.

Source causal chain:
- market.rs setup_market1057 rerolls initial stock at Startup.
- run_reset.rs101 calls market::reset_run when expedition starts.
- market.rs2514 reset_run overwrites state with default.
- market.rs545 default offers all PLACEHOLDER and next_stock_at MARKET_UNLOCK_AT+STOCK_INTERVAL =360.
- market.rs1293 maintain_market_stock does not reroll until elapsed>=360.
Thus UI unlocks at180s but shows placeholder offers until360s or paid public refresh. Injected deterministic-stock capture fixtures bypass this defect.

R8 ended actualNO LIVES REMAIN, failed1, no equipment purchase. This is evidence of firstunlock defect and failedfullflow, never a successful acquisition or forge claim. R8 isolated failure award changes only its copy to588/T5. The shared campaign save was never mounted in equipment runs; parent campaign work has independently advanced it to472/T7/unlocked6 by the final read, so this audit does not claim shared progression stayed unchanged across concurrent work.

Next: implement valid firststock reset (normal RNG, no forced item list) and test real expedition reset+elapsed180 stock. Rebuildpackage then rerun publicUI at4x. Improve market wallet OCR with isolated actualwallet ROI since wholeframeOCR read LV1 as Wi; classify actualmarket by title/stock row, never silentlymiss it. Keep previousR1–R8 failedreports/screenshots/logs.
