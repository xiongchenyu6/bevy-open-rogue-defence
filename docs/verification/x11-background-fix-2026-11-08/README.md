# X11 uncovered resize background

The production source is fda19e113b08e6e87b006e50f880568729d532df. Four changed files register an X11-only background consumer and direct dependencies on existing raw-window-handle0.6.2 and x11rb0.13.2. The consumer allocates the game ClearColor in the actual window colormap and sets the uncovered X11 window background. Source/baseline files, dependency lock and raw patch are retained.

## Intervention and production observations

The prior df51 package diagnostic explicitly calls XSetWindowBackground with0x15273a before four real resize cycles. All24 observations and thirteen smoke checks complete with exit0. Each first right-region mean becomes0.154248, compared with the earlier four first-white observations at1.0. Root independently reviewed diagnostic cycle0/frame0 through JPEG decoding. This controlled intervention is separate from the production fix.

The new package is then tested with no harness background intervention. Four cycles produce24 original observations. In every cycle the first two observations retain a dark uncovered region. The actual measured ROI is900×720+380+0, with mean0.027451, matching the rounded game clear RGB4,6,11 mean7/255. Frame2 in each cycle shows scene means0.258–0.260. Root independently reviewed all eight cycle0..3 frame0/frame2 originals through JPEGs and an exact mapping. All48 production and diagnostic originals remain retained.

The old white flash is absent in these X11/software-Vulkan observations. Dark transitions still occur. Capture start/completion are observation bounds and do not establish immediate repaint or exact GPU latency. These observations do not identify a general engine cause or cover other window systems/hardware.

## Tests and package

Actual library tests pass792 with1 ignored, strict Clippy and native build exit0, and host hashes remain unchanged. Source validation uses the retained input map. New package build and actual offline smoke children exit0 in125.487678 and164.553219 seconds, with all thirteen smoke outcomes true. The separate production resize process exits0 in210.70 seconds and also completes all thirteen outcomes and normal quits.

The immutable package directory is target/desktop-packages-x11-background-nov8/rogue-commander-linux. Tar rogue-commander-linux-0.1.0.tar.gz has62,392,062 bytes, SHA256 b18506617389f9322177de39e281f483014e7e1fbb77dd1661786a98d176c39d. Binary SHA256 is1c45f07e045f7be6a58dd6f27933e8a707efe6b0037ceecfde0844e16f8ed93b. Independent actual artifact checking verifies553 committed inputs,892 manifest entries,893 tar files with exact bytes and normalized metadata, plus dynamic metadata without Nix-store dependencies. Delivered executables remain local.

Package smoke results are technical evidence here. Visual acceptance is limited to the explicitly reviewed resize observations. No new natural campaign, Forge or hardware acceptance is claimed. Original profile contents are excluded.

Run `python3 PATH_TO_ARCHIVE/verify.py PATH_TO_ARCHIVE` for portable hashes, actual receipts, source/input joins and48 timing/mean records. Add `--pixels` for independent ImageMagick ROI remeasurement, or `--local-root PATH_TO_REPOSITORY` for actual package manifest/tar/binary byte verification. Neither mode starts a game.
