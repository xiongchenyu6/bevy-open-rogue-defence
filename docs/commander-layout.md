# Commander selection layout

In compact commander selection, the Settings launcher previously stayed in the middle of the top edge and covered the English heading. The launcher now moves to the top-right while an actual picker exists. The heading reserves the launcher's width and a gap, allowing wrapping without reducing portrait space or changing the header height.

The normal centered launcher returns after selection and when the viewport leaves compact layout. The positioning system runs before UI layout and updates only changed fields. It lives in `settings/entry_layout.rs`, keeping this responsibility separate from the Settings page.

Final library tests passed 593 cases, with 0 failures and 1 existing ignored test. Strict all-target Clippy and formatting passed. Eight native software-Vulkan captures completed 2,160 frames. All 56 current images were inspected across the root and one agent. English and Simplified Chinese covered 640×360, 667×375, 390×844 and 1280×720, plus actual compact/desktop resizing while the picker stayed open. Each case used actual initial three-card selection, six-card level 10/20/40 recruitment, fruit recruitment, Settings open/close and restoration of ordinary launcher placement. Level and fruit awards are fixtures, not evidence of earning that progression.

Actual heading and Settings rectangles never intersected at the audited phases; cards and detail footers stayed in the viewport and controls remained clickable. [Before](verification/commander-layout-2026-10-08/before-en-667x375.png) and [after](verification/commander-layout-2026-10-08/en-667x375/frame00014.png) show the original collision and its correction. Raw screenshots, logs and hashes are in [the verification directory](verification/commander-layout-2026-10-08/verification.json).

Some wide hat and weapon edges remain cropped within portrait art areas; faces, names and roles are readable. This art-fit issue and real hardware/Steam validation remain separate unfinished work.
