# Isolated numeric market-level validation

The preserved 040ce5c public run stopped at visit 11 because every whole-header OCR pass read `LVS KILLS 1757`. The image visibly contains LV5; this was a harness recognition failure, not a game win or loss. Earlier completed visits verified nine purchased equipment identities and ten resumed countdowns. The final completed visit verified Echo Core and all nine owned identities, with countdown 282 → 272. No campaign completion, reward or normal exit was verified.

The fallback applies only to the complete `LVS KILLS` plus padded-decimal candidate. It locates the observed LV word using OCR word bounds, isolates that pixel field, and reads it with the alphabet restricted to `LV` and decimal digits. This supports changing header alignment without guessing the number from an S character or from the wallet. At 200%, 300% and 600%, the actual isolated field reads `LV5`.

At least two scales must produce numeric LV tags and all valid tag readings must agree. The verified level joins each original observed wallet value in the same complete-header identity consensus. Conflicting wallet amounts, numeric levels, isolated levels or unreadable tags still fail. The independently read HUD balance remains required; it reads 1757 on this actual screenshot. No offer price or expected balance is passed into either reader.

The 72-test Docker suite includes the actual LV5 image, independent HUD balance, prior LV1/LV2/LV4 headers, disagreements and unknown-prefix rejection. The preserved package and screenshots use the old 040ce5c public geometry. This evidence does not validate the newer 495 inventory layout or a restarted campaign route.
