# Campaign save recovery

A failed campaign write now exposes a separate **Retry save** action on the campaign screen. It writes the current rewards and permanent upgrades without purchasing again or repeating reward settlement. A successful write clears the warning and removes the retry action; a failed retry retains the in-memory progress and the existing save.

If loading the original save failed, the session stays detached from that file. The campaign screen explicitly describes this as temporary progress that cannot be saved. It does not offer retry: saving default progress over an unread or subsequently repaired save would destroy the player's original progress.

Two regressions exercise actual UI interactions and real temporary files: a blocked save directory is restored before retry, and a damaged save is repaired externally while the protected temporary session remains open. The tests check unchanged currency and upgrades, one-time reward settlement, and preservation of both damaged and repaired files.

Validation: 581 library tests passed, 0 failed, 1 existing ignored; strict all-target Clippy and formatting passed. English and Simplified Chinese native captures completed 100 frames each. Four before/after images were inspected. The real retry button persisted 60 shards and one Strength rank without repeating rewards, cleared the warning, and disappeared after success. Evidence is in [the verification directory](verification/campaign-save-retry-2026-10-08/verification.json). Captures use software Vulkan and do not establish hardware performance or overall Steam readiness.
