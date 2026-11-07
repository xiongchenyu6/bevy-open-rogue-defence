# UI image review correction

The earlier complete/partial classification of decision-phase frame100 and frame120 is withdrawn. All four pairs are byte-identical PNGs, covering after-r2 and after-r3 at desktop and phone sizes. The identical PNG inputs received inconsistent displays through the image review tool. Independent ImageMagick PNG-to-JPEG decoding showed complete UI in root-reviewed examples. These observations establish an image-review inconsistency. They do not establish missing runtime UI layers.

This proof retains all eight original PNGs, their SHA256 and decoded RGBA hashes, and eight default ImageMagick JPEG conversions with exact commands and tool version. Root reviewed `/tmp/ui-diagnostic-phone-100-unique.jpg` and `/tmp/ui-original-desktop-120.jpg`. The first uses diagnostic-r2 phone frame100 with the same PNG hash as the old observation. The second uses original after-r3 desktop frame120. The eight new JPEG conversions are diagnostic derivatives. Their existence alone does not claim individual root visual review.

## Preserved experiments

Baseline desktop and phone captures exited 0. Disabling pipelined rendering also completed both captures, with selection-frame bytes unchanged. The historical `experiment-summary.json` records the withdrawn partial-layer interpretation and remains unchanged as history.

The first render diagnostic failed compilation with exit101. Corrected diagnostic-r2 built and completed desktop and phone captures. Its actual extracted-node, queued-index and prepared-batch traces are preserved. Positive node/glyph counts and zero missing batches support the numerical render-path observation. Complete runtime visual acceptance is separate from those counters.

The single-camera experiment built successfully but phone capture exited101 at the old-rank settlement assertion, expected0 and actual4. Its changed camera setup did not settle the pointer choice. Desktop was not run. All logs and receipts are preserved. Capture/probe source copies represent the latest ephemeral single-camera version, rather than exact source snapshots for every earlier attempt. The temporary source experiments were removed. This correction introduces no runtime change.

## Verification

Run `python3 PATH_TO_ARCHIVE/verify.py PATH_TO_ARCHIVE`. The portable verifier checks exact file membership and SHA256, lossless raw logs, four byte-identical PNG pairs, all eight JPEG hashes, process results and diagnostic batch traces. When ImageMagick is installed it also recomputes the recorded PNG decoded RGBA hashes. Local binaries and unselected image bulk are represented by hashes. This proof corrects image-review classification and makes no new full-campaign or package acceptance claim.
