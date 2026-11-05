# Current packaged natural equipment purchase

The isolated Steam Runtime package from `8f4084b` ran chapter six at 1× using an
unmodified actually earned checkpoint: first five chapters cleared, 645 shards,
Strength training 6. The input hash/provenance is recorded. No stock, kill tokens,
health, skills or victory were injected. Public 1/g/f choices plus natural-market
inspection were the only strategy; no movement or stock refresh was used.

On visit 3 the real shop held 307 kill tokens. The script inspected all five offers,
selected Wind Boots at 160, required matching visible preview, clicked the public
Buy button, and verified the HUD dropped exactly to 147. It opened Equipment,
selected the actual owned icon and read `[R] Wind Boots`. The visible description
states +15% move speed,−15% dodge cooldown,+8% evasion; this does not independently
measure those combat modifiers. Root inspected the native inventory image.
After closing both panels, the real countdown decreased 1164→1155 seconds.

`verify.py` independently rereads archived native pixels through Tesseract and
ImageMagick; `pixels.json` is its output. It verifies exact deduction, owned
detail, resumed time, and recognition of the actual later level 10 recruitment
screen. The full report retains all offer OCR, selected inventory icon and the
three completed resume proofs. Run with `tools/desktop` on `PYTHONPATH`.

The run then stopped on visit 4, when a level 10 recruitment screen opened after
closing the shop. The previous clock helper read the obscured HUD before draining
that new choice. This was a harness interruption, not an observed chapter win or
loss; the finally block terminated the game. Full campaign completion and normal
end-of-run exit/reward/restart remain unverified.

The follow-up helper only presses 1 after recognizing a public choice, then
requires a strict readable countdown. An unreadable HUD without a known choice
still fails. Two focused regression tests plus existing native OCR tests pass:
**10 tests passed**. Live post-market level 10 drain still needs a new run.
The next route also leaves 60 seconds after each completed market visit, so slow
software rendering does not cause immediate back-to-back visits. Limits remain
12 visits, 90 seconds per visit and 2400 seconds for the battle route.

This is real earned purchasing in the new illustrated package, not the earlier
funded art fixture. It does not prove full campaign balance, current-package
natural forging, real hardware sound/performance, physical controllers or Steam
client acceptance.
