# Mobile icon controls

Runtime: `9814bc5`.

- Authored 16 reusable control glyphs with editable SVG sources and reproducible PNG generation.
- Chapter tools use 48px icon targets; chapter states use play/lock/clear corner badges; portrait headings and tools no longer overlap.
- Settings uses a gear launcher, fixed header/footer and bounded, touch-scrolling content. Short labels remain where necessary for comprehension.
- Battle pause/speed/reset/challenge controls are icon-led, touch dodge displays art and cooldown, and the joystick appears only while held. Its touch region stays available while hidden.
- Detailed/simple selection uses a book glyph, short mode label and explanatory hint.

## Render checks

Production capture renderer at 844×390, 667×375, 390×844 and 1280×720. Chapter captures assert all six cards fit, heading/tools do not overlap and toolbar targets are at least 48px. Settings captures switch language, request scrolling past the bottom, verify the bounded position and visible audio controls, and preserve access to fixed navigation. Camera captures send actual TouchInput messages at the computed movement-pad center: frame 6 held, frame 10 released.

An initial settings scroll capture exposed a blank-body overscroll. The final captures below use the corrected content-bound clamping and strengthened assertions. Font coverage also caught two unsupported characters in new help copy; equivalent supported wording replaced them.

![Chapter tools](chapters-landscape.png)
![Portrait chapters](chapters-portrait.png)
![Touch held](battle-touch-held.png)
![Touch released](battle-idle.png)
![Settings after overscroll attempt](settings-after-scroll.png)

Raw logs/scripts: `target/mobile-icons-nov15/`. These are controlled production-renderer fixtures, not a complete campaign playthrough.

## Functional verification

- Full library suite: **829 passed, 0 failed, 1 ignored** (opt-in report only).
- Selection interaction through frame 124: real paid draft, detailed/simple/detailed toggles, unchanged skill rank and offers.
- Settings navigation: cancel departures without modifying the run, confirm return to chapters, reopen settings above chapters; visible controls retain minimum touch sizes.
- Trial drawer: tower/gold auto toggles, automatic trial entry, and cancel-all state transitions passed.

![Selection toolbar](selection-icons.png)
![Trial controls](trial-icons.png)

Validation notes: the optional selection fixture's later tidal-area hover assertion fails on the pre-change capture binary as well; it is not used as evidence for this UI change. The settings navigation audit now excludes explicitly `Display::None` controls (the fixture omits the onboarding plugin), while continuing to require at least five real, visible navigation controls and their bounds.
