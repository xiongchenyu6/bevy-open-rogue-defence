# Partial-step freeze and stun expiry

Control ticking previously subtracted the whole frame before enemy AI ran. If a0.1-second freeze expired inside a32/60-second frame, the enemy received all0.5333seconds for moving and attacking. The registered ranged-AI baseline fired once where no attack was due. Four selected baseline regressions failed.

The runtime now records the time already occupied by freeze or stun during the current step. Overlapping controls occupy the maximum remaining duration, not their sum. An expired control retains that spent time until the next runtime tick. All consumers read the same remaining action budget; reading it does not consume it again. New ongoing control still blocks actions, while reset and missing-target cleanup clear state.

Lane movement and attacks, active healer output, challenge/archive attacks, campaign boss age and pending windups, boss-held inactive warnings and adventure cast clocks use the available time. Released hazards, passive auras and overall encounter timeouts keep normal simulation time. Zero-time ready actions retain their previous behavior for uncontrolled actors; actual ongoing control still blocks them.

Seven regressions run production control ticking and test expiry, overlapping freeze/stun across1/60,32/60,32/15and single large steps, repeated readers, pause/reset/removal, real ranged movement and attack cooldown, trial/archive damage, and synchronized boss pending windups and real spawned warnings. The first full suite also found two zero-time readiness regressions; those existing tests were retained and the production guards corrected.

This change establishes the tested action budgets. It does not establish whole-game timing equivalence across speeds. Partial slow expiry, when a new control is applied within a frame, hazard warning-to-active transitions, and actual animation playback time still require separate treatment. The existing5c77f1e release candidate predates this change.

Final627 library tests passed with0 failures and1 ignored; format, strict all-targets Clippy and both binary builds passed. Four400-frame native scenes at1280×720 and667×375 passed; twelve screenshots were inspected for frozen-shell and death/respawn visual regressions. Static screenshots do not prove timing budgets; the registered-system tests provide that evidence. [Verification](verification/control-time-2026-10-09/verification.json).
