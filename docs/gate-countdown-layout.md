# Gate rebuild countdown layout

Short landscape viewports use an80px badge and40px digit, placed near the projected wall within a band that avoids the left HUD, top speed controls and bottom action row. Taller desktop viewports retain the120px badge and64px digit centered on the wall. Size and font changes use conditional writes so an unchanged badge does not mark those properties changed every frame; no layout timing benchmark was performed.

Final source passed601library tests,0failed,1existingignored, strict all-target Clippy and both binary builds. Three native summon-assault cases completed2040frames: English1280×720, English667×375 and Chinese667×375. Six screenshots were inspected. The short viewport countdown no longer overlaps the HUD or buttons; the desktop countdown retains its position. Actual pursuit, damage, pause, retargeting, outside-wall formation and finite-health wall rebuild checks passed. [Evidence](verification/gate-countdown-layout-2026-10-09/verification.json).

The earlier October8jobs lost their handles and had incomplete final logs; they are not accepted as final verification. Other sizes, controller/touch input and the packaged release remain unverified. The existing Linux package has not been rebuilt with this source.
