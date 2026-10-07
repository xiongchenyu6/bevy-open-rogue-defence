# Expedition loading

The windowed entry point now installs an expedition loading gate before recruitment can enter combat. It checks the current offered heroes and portraits, the draft backdrop and UI font, and visible scene roots, including recursive dependencies. The player sees the actual number of completed resource handles. Unoffered heroes and unrelated failed resources do not block recruitment.

The gate blocks both card buttons and keyboard confirmation. Combat and survival systems wait from their first frame. Recruitment continues to own its pause; the loading gate does not overwrite the player's or Settings' pause state. The renderer-free balance simulator installs GamePlugin without this presentation plugin.

A failed resource exposes **Retry loading**. Retry reloads the failed parent file and failed leaf dependency paths. If the external UI font itself is unavailable, the loading message uses Bevy's embedded English font so the error and retry action remain readable. Once the UI font loads, the screen uses the selected language.

Resource readiness means CPU assets and their dependencies are loaded. It does not prove GPU upload completion or frame-rate performance.

Validation: 585 library tests passed, 0 failed, 1 existing ignored; strict all-target Clippy and formatting passed. The desktop and capture programs built. Tests cover direct and nested missing files, a real retry held longer than 1.1 seconds, repeated clicks, another failed attempt, recovery, input guards and pause preservation.

Three native software-Vulkan captures completed 450 frames in total: English and Simplified Chinese at 1280×720, plus English at 640×360. Nine images were inspected. The initial recruitment clock and selection remained blocked through pending, failure and retry; the real backdrop read count stayed at two despite another mouse click; recovery reached 24/24 and a fresh card click began combat. A genuinely failed, unoffered hero scene did not block readiness. Evidence is in [the verification directory](verification/expedition-loading-2026-10-08/verification.json).

The first native attempt exposed Bevy retaining the old Failed state while a new read was active. The final code waits for a fresh failure event or full readiness rather than using a time threshold. Native late recruitment and missing-font fallback screenshots remain unverified, as do hardware performance and overall Steam readiness.
