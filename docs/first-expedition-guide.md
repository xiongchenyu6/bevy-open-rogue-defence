# First expedition guide

The windowed game shows a short illustrated guide on the first actual expedition, after the recruitment assets are ready. Four steps cover movement and automatic attacks, three commander lives and unlimited gate rebuilding, resources and continuous card selection, then chapter unlocks and permanent Training. Each step has concise English and Simplified Chinese copy. Players can skip it or move back and forward.

Skip or Start records dismissal through the existing native configuration or browser storage. If that write fails, the guide stays dismissed for the current session and Settings retains a visible warning. Settings can replay the guide from chapters, results, recruitment or combat.

The guide owns only its modal pause. Closing restores the prior pause, while leaving for chapters or results preserves their new pause. Modal hotkeys and interactions are consumed; the closing click remains blocked until its pointer is released. Run reset drops transient guide state while keeping the saved preference.

Onboarding is installed explicitly by the windowed entry point. Renderer-free simulations and ordinary capture scenarios do not install it. Dedicated captures install the real plugin and use isolated preference files.

Validation: 592 library tests passed, 0 failed, 1 existing ignored; final strict all-target Clippy and formatting passed. The desktop and capture binaries built. Twelve native software-Vulkan cases completed 1,600 frames, with 51 screenshots actually reviewed across the root and two agents.

English and Simplified Chinese guide/reload cases cover desktop 1280×720 and compact 667×375. Real inputs verify four loaded illustrations, modal key/card blocking, pointer-release protection, preserved picker pause, a fresh choice starting combat and dismissal in a separate process. Additional cases verify Skip, Settings replay from chapters/results, and a real preference write failure followed by filesystem repair, actual replay and warning clearance. The result case configures a terminal UI prerequisite; it does not prove a played victory.

Evidence is in [the verification directory](verification/first-expedition-guide-2026-10-08/verification.json). A pre-existing English compact picker heading overlaps the Settings launcher, visible in [the recorded image](verification/first-expedition-guide-2026-10-08/en-667x375-reload/frame00029.png). Guide and Settings controls themselves fit. The host-layout collision was subsequently corrected and verified in [the commander layout follow-up](commander-layout.md); this older image retains the original evidence. Human comprehension, browser-storage failure, real hardware and overall Steam readiness remain unproven.
