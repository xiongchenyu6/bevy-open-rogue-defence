# Bond replacement is a public paused choice

Read-only audit found one missing in-scope heading: `Choose a bond to replace`.
`progression.rs` enters this paused draft when the public F chooser selects a new
family at the active bond limit. `progression_ui.rs` supplies that exact heading.
The generic public number-key handler confirms its first choice with 1. The old
`choice_pending` returned false for that heading; the new recognizer returns true.

The helper deliberately does not treat `Choose an encounter` as the same action.
That optional menu requires E/click or the internal autoplay feature, none used
by the stationary 1/g/f equipment strategy. Adventure boss exploration is likewise
optional and has different controls; cultivation procs have no modal. Skill,
bond, evolution and milestone recruitment headings were already recognized.

Regression result: 21 tests passed, including the new replacement recognition and
negative encounter case, alongside actual native screenshot OCR regressions.
The source audit and parsing test do not prove a live post-market replacement.

R6 was already running with the prior loaded policy. It is left uninterrupted;
its original policy SHA256 and matching full source snapshot are retained under
`target/verification/desktop-release-2026-11-05/chapter6-r6/`. Do not attribute
this new recognition rule to that process retroactively or use its result as
coverage of the new branch. A future live trigger still needs verification.
