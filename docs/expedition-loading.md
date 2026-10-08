# Expedition loading

The windowed entry point now installs an expedition loading gate before recruitment can enter combat. It checks the current offered heroes and portraits, the draft backdrop and UI font, and visible scene roots, including recursive dependencies. The player sees the actual number of completed resource handles. Unoffered heroes and unrelated failed resources do not block recruitment.

The gate blocks both card buttons and keyboard confirmation. Combat and survival systems wait from their first frame. Recruitment continues to own its pause; the loading gate does not overwrite the player's or Settings' pause state. The renderer-free balance simulator installs GamePlugin without this presentation plugin.

A failed resource exposes **Retry loading**. Retry reloads the failed parent file and failed leaf dependency paths. If the external UI font itself is unavailable, the loading message uses Bevy's embedded English font so the error and retry action remain readable. Once the UI font loads, the screen uses the selected language.

Resource readiness means CPU assets and their dependencies are loaded. It does not prove GPU upload completion or frame-rate performance.

Validation: 585 library tests passed, 0 failed, 1 existing ignored; strict all-target Clippy and formatting passed. The desktop and capture programs built. Tests cover direct and nested missing files, a real retry held longer than 1.1 seconds, repeated clicks, another failed attempt, recovery, input guards and pause preservation.

Three native software-Vulkan captures completed 450 frames in total: English and Simplified Chinese at 1280×720, plus English at 640×360. Nine images were inspected. The initial recruitment clock and selection remained blocked through pending, failure and retry; the real backdrop read count stayed at two despite another mouse click; recovery reached 24/24 and a fresh card click began combat. A genuinely failed, unoffered hero scene did not block readiness. Evidence is in [the verification directory](verification/expedition-loading-2026-10-08/verification.json).

The first native attempt exposed Bevy retaining the old Failed state while a new read was active. The final code waits for a fresh failure event or full readiness rather than using a time threshold. Native late recruitment and missing-font fallback screenshots remain unverified, as do hardware performance and overall Steam readiness.


## Native missing-font follow-up

The harness now supports `asset-readiness-font`, using the same isolated real FileAssetReader and held-read protocol as the backdrop scenario. The copied font file is genuinely absent; production assets and readiness flags are not modified. Actual overlay TextFont sources must use the embedded default font while loading/failure/retry is shown in English. Missing-font card and keyboard input remain blocked and the clock remains frozen. After restoring the copied font and actual Retry input, recovery requires AssetServer loaded-with-dependencies, the actual Chinese UI using the production font handle and a fresh actual card choice before the clock advances. Repeated retry clicks do not create a third read.

NVIDIA Vulkan captures at 1280×720 and640×360 in Chinese mode, plus the original English backdrop regression, completed450 frames. Root viewed all12 images: pending/error/held-retry text and retry controls are readable, recovered Chinese cards/HUD render correctly. Capture Clippy, formatting and compilation passed. This change extends verification only; production behavior is unchanged. The native retry hold is25frames≈0.83simulation seconds; the separate prior unit test remains the evidence for a longer-than1.1-second hold. Later level/fruit recruitment, natural progression and packaged missing-file recovery remain unverified. [Evidence and hashes](verification/font-loading-2026-10-08/verification.json).


## Later recruitment and labeled scene retry

A real missing-model capture exposed a production retry bug. Nine non-R hero GLBs were absent in an isolated copy. The initial three R offers correctly loaded and selection started the clock; fixture level10 then opened the actual six-card recruitment. Its four missing offered scenes correctly blocked input, but stripping `#Scene0` from their retry paths left no live parent handle for AssetServer.reload. All nine GLB read counts stayed at1, while the gate remained retrying. The failed native log is retained.

Retry now retains the actual requested asset path, including a scene label, and matches fresh failures by underlying file path. This reloads the live failed scene handle and its file. Shared genuine asset I/O was extracted into `capture/asset_io.rs`; the initial backdrop/font and later recruitment scenarios use the same reader rather than duplicated mock loaders.

Full library tests passed596 cases (0failed,1existingignored), strict all-target Clippy and both binaries passed. The final capture-only follow-up chooses a genuinely reloaded non-R hero, GroveArtificer, and passed capture Clippy/build/format. Four English/Chinese desktop/compact later-recruitment captures and two font/backdrop regressions completed960 frames. Root inspected30 final images. Offered missing GLBs were read exactly twice, unoffered GLBs once; repeated real pointer retry clicks issued no additional reads across a35-frame held retry (>1.1simulation seconds). Blocked cards and Digit1, including the first frame after opening the later picker, preserved the old hero, frozen clock, milestone, skills, bonds and legacy. After current offered scenes loaded with dependencies, one fresh choice committed exactly once, retained prior ranks/cards, absorbed the old hero and resumed the clock. The reloaded hero appears in the world screenshot.

Only level10 is fixture-awarded; this does not prove natural progression, fruit consumption, later20/40 milestones or packaged missing-model repair. Those remain separate checks. [Failure log, final screenshots, raw logs and source hashes](verification/recruitment-loading-2026-10-08/verification.json).
