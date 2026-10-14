# Controller controls

Controller input is implemented and verified in source with ECS tests and synthetic native gamepad events. The full library suite passed (654 passed, 0 failed, 1 ignored); strict Clippy passed for all targets. All 75 native route steps passed at 1280×720, 1280×800 and 667×375, with screenshots reviewed. The existing desktop package has not been rebuilt with this change. Physical controllers and Steam Deck hardware remain unverified.

| Context | Controls |
| --- | --- |
| Battle | Left stick moves; South dodges; Start pauses; left/right shoulders decrease/increase speed; East or Select toggles HUD navigation |
| HUD and menus | D-pad or left stick moves the visible focus; South activates the focused control; East closes a cancellable view; right stick scrolls the focused list |
| Forced recruitment/cards/camp choices | Navigate and choose with South; Back cannot bypass the choice |
| Pause | South on Resume, East, or Start resumes; the settings entry remains reachable |
| Settings confirmation | East cancels only the confirmation, retaining settings and the prior battle pause |
| Equipment and forge | Navigate tabs and recipes to preview; activate the forge confirmation to craft; missing-material confirmations and locked chapters are excluded from navigation |

Displayed A/B letters use Xbox positions and include South/East qualifiers; device-specific hardware mapping is unverified. Prompts follow the current input mode and selected language. A deliberate controller button or meaningful stick change selects controller mode; keyboard, pointer or touch input restores that mode. Connected idle controllers and stick noise do not change prompts. No keyboard input is injected.

The input path uses Bevy 0.19 gamepad components and events, a radial stick deadzone, real-time navigation repeat, and one active controller. A new deliberate action can switch controllers; a held stick on the old controller cannot steal control back. Disconnect and window focus loss clear movement and dodge and pause an active battle. Reconnection does not resume it.

UI navigation reads explicit modal scopes and visible public buttons. Named ancestry supplies stable focus keys through panel rebuilds. Only the active scope receives confirmation or Back; startup/tutorial blockers own input even if they have no actionable button. Focus previews use a separate component, preserving real pointer hover and click state. Confirmation presses the existing public button for one frame, so purchases, crafting, cards and departure use their existing validation and pause ownership. A held South button cannot select a subsequent rebuilt card or repeat a purchase.

Source owners are `controller_input.rs` for sampling/prompts/reset, `controller_navigation.rs` for focus/activation/reveal, and the existing UI modules for preview and actions. Input-mode state is one `ControlScheme` enum; old boolean fields were removed.

The ECS suite covers held confirmation across rebuilt cards, departure Back consumption, no-button startup blockers, hidden/disabled controls, focus persistence, pointer takeover, multiple-controller takeover, movement/dodge/disconnect, directional reveal and right-stick scrolling, real-time navigation under virtual speed/pause, modal-first default focus, and same-frame controller camp activation, and visible versus banked draft camp eligibility. The native `controller-input` capture route uses actual gamepad connection and raw button/axis messages and navigates public controls through chapters, recruitment, combat, pause, audio/display settings, departure cancellation, cards, equipment, market purchases, forge and return to chapters. The route banks a paid draft with Later, enters a camp, and reopens the identical offers without increasing paid draws. Held confirmation must increase total skill ranks exactly once; a second independent press must increase them once again while funded drafts continue. It records individual screenshots and state assertions. Its wallet/time and deterministic market stock are injected fixtures, explicitly logged; they are not evidence of earned progression.

Native capture also passed the existing keyboard/touch route at 667×375 and the controller-driven genuine loading failure/retry scenario. The compact Settings launcher remains reachable in the draft footer with reserved description space, and the combat hint sits below the launcher. Native bounds assertions verify launcher/detail-toggle and hint/launcher separation. Scroll/reveal and real-time navigation under virtual speed/pause are verified by ECS fixtures; a separate native right-stick scrolling sequence was not recorded.

Choosing Later keeps the paid card offers while returning to battle. A banked draft now permits camp entry; a visible draft still blocks it. The native route banks offers, chooses a safe camp reward, reopens exactly the same offers with an unchanged paid-draw count, and verifies the wallet retains the reward plus normal passive income. This fixes the shared mouse/controller camp restriction without clearing banked choices.

The local evidence archive is `target/verification/controller-input-2026-10-14/final-evidence-r13.json`. It records source, binary, log and screenshot SHA-256 hashes and successful service handles. Full tests are in `library-tests-r11.log`; final strict checks in `clippy-r13-final.log`; native results in `native-unit-r13.log`; screenshots are in each size's `frames/` directory. The immutable native binary is `capture-r13`, SHA-256 `f6560f7722dd1c8df23eaf0c2ecd08f693947c28f6bc84f9fb0ecb6330176ca3`. Earlier failed runs are preserved separately; `r9-passed/` is an earlier successful route that cleared the outstanding draft rather than exercising banked reopening.

Physical Linux discovery/mapping/hotplug and Steam Deck suspend/Steam Input configurations require separate hardware runs. Synthetic native events verify the software path and public UI handlers, not physical devices or the existing released package.

Archived logs, source hashes and native screenshots: [verification evidence](verification/controller-input-2026-10-14/evidence.json). Root independently checked source/artifact hashes and reviewed 20 final screenshots.
