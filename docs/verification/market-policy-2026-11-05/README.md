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

**8 passed**, including real Tesseract OCR of committed native market frames
35/96/125/145 under `black-market-2026-11-04/en-1280x720`. They verify wallets
100000/99880/0/0, all five prices and selected item name/category. Artificial
rows only test parsing/transaction constraints, and are not native evidence.
The actual committed Arc Lens inventory screenshot also verifies selected-detail
OCR at the unchanged panel geometry. Current-build live inventory is not yet proven.

The policy searches visible inventory icons using exact selected-detail names;
it does not assume new purchases append to the grid. Header closure recognizes
`EQUIPMENT · N`, queues include evolution choices and must drain before opening
the market. Wrapped tile names must match exactly across at most two contiguous
name lines. After closing both panels, the live policy additionally requires the
public battle countdown to decrease, with strict parsing and the existing visit
time budget. This directory records the earlier eight-test parser stage.
Subsequent actual packaged purchasing and three live countdown-resume proofs
are archived in [the earned-market run](../earned-market-2026-11-05/README.md).
That newer route has ten equipment-policy regressions, plus nine common
campaign checks including the isolated speed-label failure frame.

These earlier pixel-reading tests alone do not prove transactions, progression
or chapter-six victory. Use the newer earned-market evidence for the natural
purchase claim; complete chapter-six victory remains unverified. Fixed scope:
1280×720 English UI.
