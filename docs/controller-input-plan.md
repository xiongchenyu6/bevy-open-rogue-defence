# Controller input plan

Status: audited 2026-10-14; **not implemented or runtime verified**. No physical controller or Steam Deck verification has been performed.

The current game has keyboard/touch combat controls and mouse/touch menu interactions, but no `Gamepad` usage in `src`. Bevy is pinned to 0.19.0; its locally installed `bevy_input` exposes entity-based `Gamepad`, `left_stick()`, `pressed()` and `just_pressed()`. No new input dependency is needed.

## Smallest complete implementation

Add `src/game/controller_input.rs`, registered by the game plugin in `src/game/mod.rs`. Keep controller sampling, active controller selection, button edges, stick deadzone/repeat, and UI focus in this file. Keep purchases, crafting, recruitment and pause ownership in the existing handlers. Do not emulate keyboard keys: Escape is currently consumed/cleared by several modal handlers, and injecting keys would couple unrelated actions.

Use the most recently deliberately operated connected controller. Ignore stick noise below a radial deadzone (initial 0.2), rescale outside it, clamp magnitude to one. Disconnect clears held movement and pending button edges and pauses an active battle; reconnection must not automatically resume. Window focus loss clears controller intent and pauses through the existing pause path. Do not choose an arbitrary controller by query iteration order.

Combat mapping: left stick moves; South dodges; Start pauses/resumes; shoulders decrease/increase speed. East opens the existing pause/menu controls so every HUD action can be reached without memorizing many combinations. While a modal is open, South confirms the focused control, East goes back only when that view is cancellable, D-pad/left stick navigates, shoulders change explicit tabs where present, and right stick scrolls the focused list. Forced recruitment/draft choices must retain their existing no-cancel behavior. Speed and dodge bindings are disabled while navigating a modal. Use positional button glyphs initially; do not label every controller as Xbox.

Replace `ControlScheme.touch: bool` in `mobile_layout.rs` with a single input-mode enum (keyboard/mouse, touch, controller), updating all call sites. Actual button presses or meaningful stick motion switch mode; noise/connected devices do not. Mouse movement or click restores pointer mode. Controller focus draws a visible outline and uses the same tooltip/detail content as pointer hover. Keep touch layout independent of controller mode and viewport size.

UI entities receive a small `ControllerFocusable` component containing a stable semantic key, panel/scope, explicit row/column/order, and eligibility. One resource holds the active scope and selected key. Select the first enabled control on opening; restore by semantic key after a panel rebuild; fall back to the nearest enabled item when that key disappears. Never store only an entity: campaign, equipment and draft panels rebuild/despawn their entities.

Apply navigation after Bevy UI focus and before current interaction consumers. On controller confirmation, mark only the focused eligible control `Interaction::Pressed` for one frame so the existing `Changed<Interaction>` handlers execute. Reset only controller-owned interactions next frame, after UI focus, without clearing actual pointer input. Focus previews need explicit checks in hover consumers; do not continuously counterfeit pointer hover. Handle back/pause through explicit one-frame controller actions read and consumed by the topmost existing handler. Verify schedule ordering and one-frame release before relying on this bridge; `Interaction` mutation must never create repeat purchases.

Modal priority must be explicit and follow current visible state: loading/onboarding blockers; settings departure confirmation; settings; forced choices/adventure; equipment/collection/attributes/market; manual pause; chapters/results; battle HUD. A single confirm/back edge can reach only one scope. Preserve existing prior-pause restoration. During automatic/forced drafts no battle action can leak through.

## Source entry points

| Surface | Existing owner to extend |
| --- | --- |
| Movement/dodge, transient reset | `mobile_input.rs`: `read_move_input`, `clear_transient_input`, `reset_run`; movement runs after `SurvivalSet`, before `CombatSet` |
| Input mode/HUD hints | `mobile_layout.rs`: `ControlScheme`, `update_control_scheme`; `ui.rs` dodge/HUD controls |
| Pause/speed and HUD entry actions | `ui.rs`: `handle_manual_pause`, speed handler around line 1338; annotate pause, speed, recruitment/cards/camp and panel entry buttons |
| Recruitment/fusion preview | `hero.rs`: `handle_hero_choice`, `HeroChoiceOrdinal`; `hero_fusion_ui.rs` button creation |
| Skill/bond/encounter cards | `progression.rs`: `request_skill_draft`, `request_bond_draft`, `request_encounter_draft`, `handle_draft_buttons`, `handle_draft_actions`, `handle_draft_keys`; annotate controls in `progression_ui.rs` |
| Chapters/training/results/save retry | `campaign_ui.rs`: `button`, `input`, `render`, `scroll`, `tooltips` |
| Settings/audio/display/departure | `settings.rs`: `handle_settings_input`, `SettingsInputSet`; `settings/audio_controls.rs`, `display_controls.rs`, `run_actions.rs` input handlers; confirmation owns Back before settings |
| Equipment/forge | `equipment_ui.rs`: `input`, `details`, `scroll`; `equipment_ui/panel.rs` control creation; recipe focus updates preview, South on ConfirmCraft invokes existing readiness-guarded craft path |
| Camps/market and combat encounters | `market.rs` existing entry/purchase/action handlers (Pressed consumers around lines 1346–1457); `trial.rs`: `challenge_input`; adventure public controls; annotate all their enabled controls |
| Attributes/collection/choice details | `hero_stats_ui.rs`, `bond_collection_ui.rs`, `selection_detail.rs`, `ui_art.rs` hint activation |
| Startup gates/tutorial | `asset_readiness.rs` retry button, `onboarding.rs` public guide controls; controller must not bypass readiness/input blockers |

## Verification required before claiming support

1. Pure tests for deadzone/sign/magnitude, deliberate-device selection, disconnect/focus loss, D-pad/stick repeat delay, and mode changes without noise flicker.
2. ECS tests using actual Bevy gamepad input messages/components: analog motion and one dodge; pause stops combat; resume has no residual dodge; speed unchanged during modal navigation. Confirm through annotated public controls, not direct crafting/purchase calls.
3. Focus tests: deterministic initial order; locked/hidden controls excluded; rebuild preserves semantic focus; list navigation scrolls target into view; tooltip/recipe/hero preview follows focus. One held confirm yields one purchase/craft/draft choice; closing a modal cannot activate the button beneath it.
4. Exercise every public path with controller only: chapters → recruitment → movement/dodge → draft/cards → camps/market → equipment → forge tab/recipe/confirm → settings audio/display → cancel departure → results/save retry → next chapter. Include forced-choice Back, unavailable forge recipe and locked chapter rejection.
5. Add `src/bin/capture/controller_input.rs` alongside the existing `capture/hud_input.rs` message-driven harness. Save native screenshots at 1280×720 and 1280×800 of chapter focus, recruitment preview, combat hints, card focus, scrolled camp/market, forge selection/result, settings confirmation, and results. Record assertions on visible focus geometry, viewport containment and authoritative state changes. Use an isolated save path as existing capture workflows require.
6. A native message-driven harness validates game logic and rendered UI only. Separately test physical Linux gamepad discovery, hotplug/disconnect and actual button mapping, then Steam Deck hardware/suspend and Steam Input configurations. Until those exist, report synthetic native evidence and explicitly leave hardware/Deck unverified.

No production sources, running balance/capture jobs, saves, or lockfiles were changed for this audit.
