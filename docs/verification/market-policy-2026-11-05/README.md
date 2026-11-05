# Illustrated market campaign policy

The chapter-six public UI equipment policy now reads the illustrated market,
hovering all five tiles and OCRing each tile's name, price and shared preview's
Equipment/Consumable category. It selects the cheapest affordable equipment,
rechecks the visible offer, then clicks Buy. No stock refresh, game-state read,
resource injection or save mutation is introduced. Existing exact wallet deduction,
owned inventory proof and bounded panel closure checks remain required.

The previous parser expected `[GEAR]` text rows and an old wallet position; it
cannot establish purchases from the redesigned UI.

Independent root rerun, Docker image
`rogue-commander-linux-smoke:ubuntu24-oct8`, network disabled and source read-only:

```sh
docker run --rm --network none \
  --mount type=bind,src="$PWD",dst=/project,readonly \
  --workdir /project/tools/desktop \
  rogue-commander-linux-smoke:ubuntu24-oct8 \
  python3 -m unittest -v test_campaign_equipment_runtime
```

**5 passed**, including real Tesseract OCR of committed native market frames
35/96/125/145 under `black-market-2026-11-04/en-1280x720`. They verify wallets
100000/99880/0/0, all five prices and selected item name/category. Artificial
rows only test parsing/transaction constraints, and are not native evidence.

This verifies reading existing native pixels, not a live campaign transaction,
progression, equipment affordability in a natural run or chapter-six victory.
The live packaged route remains an open gate. Scope is fixed 1280×720 English.
