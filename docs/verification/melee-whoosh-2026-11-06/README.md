# Melee windup audio — actual producer and recorded-output evidence

The sword, axe and dagger basic attacks now have original, separate swing sounds.
The existing `PendingSwing` holds a single consumed bit. A narrow system between
`auto_attack` and `resolve_swings` waits until remaining virtual windup crosses
the candidate's real-time peak lead (50/60/20 ms, multiplied by virtual relative
speed). It does not change attack timing, damage, targeting or combat RNG.

The precontact system only records three bounded family bits in the existing
`AudioThrottle`; actual `AudioPlayer` entities are spawned by the existing audio
consumer **after `CombatSet`**. This matters: an intermediate implementation
spawned sound entities before the damage resolver and changed critical-hit
assignment despite identical contact tuples and final RNG state. The retained
regression now checks exact contact order, per-target HP and RNG through a mixed
normal/fusion fixture and a subsequent real attack. It does not establish global
audio-independent determinism.

There is no `Added<PendingSwing>` dependency, no layer/cleave duplication and no
new general event bus. Swings already canceled at the precontact check are consumed silently; paused, Autoplay,
unavailable-output and post-combat pause/finish boundaries cannot build a replay
queue. Playback is real-time limited. At large frame deltas or 32× speed the
intent can precede contact within one update while actual playback follows HP;
there is no promise of physically audible anticipation or artificial HP delay.

## Verified runtime and assets

- Original behavior negative regression: actual `auto_attack` produces one
  `PendingSwing`, but the old audio consumer creates zero swing voices (0 vs 1).
- Final release library: **700 passed, 0 failed, 1 ignored**. The ignored test is
  pre-existing. Strict release Clippy passed. Tests cover lead timing, 30/60/120
  Hz and 1×/2×, same-update/32×, multi-target, actual miss, cancellation and
  suppression, post-phase pause/finish and exact mixed-contact/HP/RNG equality.
- The new audio tests are isolated in `src/game/combat/melee_audio_tests.rs`.
- Three original WAVs are mono PCM16/48 kHz, 190/250/115 ms; peak amplitudes
  0.530/0.600/0.430 and RMS 0.1148/0.1285/0.1098. No clipped source samples.
  `tools/generate_melee_whoosh.py` regenerates their PCM byte-for-byte. No external
  samples, paid generation service or ranged-shot substitution was used.

The final 700-test runtime was not changed afterward. A later six-line
`cfg(debug_assertions or dev-tools)` read-only pending-count diagnostic and its
re-export were added for capture. Default release code does not expose that
probe. Subsequent capture-only Clippy/builds check the diagnostic and controlled
fixture; these were not misreported as another full library run.

## Actual native cases

| Case | Capture stage | Actual child | Frames | Unique swing voices | First source/sink frame | First HP decline | Correlation | Fitted gain |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| Sword | R4b | 0 | 2000 | 64 | 32 | 64 | 0.954222 | 0.207732 |
| Axe | R5b | 0 | 2000 | 68 | 31 | 55 | 0.967499 | 0.220954 |
| Daggers | R5b | 0 | 2000 | 63 | 32 | 63 | 0.828676 | 0.170109 |

These are controlled actual-ECS attacks on the native NVIDIA Vulkan renderer,
with a private Pulse null sink reached through the actual ALSA backend and an
actual monitor recording. Music is zero, effects 100, Autoplay/learned skills and
ultimates suppressed. The optional audio fixture uses **Automatic `Time<Real>`,
1 APS and virtual max-delta 50 ms** to retain a dodgeable windup. These fixture
settings are not production balance changes or a natural-performance benchmark.

For axe/daggers, after the actual first basic attack commits, targets are moved
out of reach and remain there until the real pending-melee count reaches zero
and at least 0.65 real seconds pass. The entire interval asserts zero HP loss.
The targets then return; subsequent production attacks must cause actual damage
and weapon visuals. A 30-real-second bound from the fixture's first update and a
300-second child bound remain. These are the approved R4/R5 bounds; the earlier
premature five-second verification failure is retained below.
Sword's accepted earlier R4b capture used the timed displacement; its actual
zero-HP interval, source/sink, later HP decline and recorded source match are
retained. It was not rerun or relabeled as having the later count diagnostic.

R4b and R5b are **different capture/diagnostic versions** with identical production
`audio.rs`, `audio_melee.rs`, combat, fusion and three WAV hashes. Removing only
the diagnostic function exactly restores R4b `basic_attacks.rs` bytes. Native
before/after input receipts are equal in each build. R5b only ran axe/daggers;
its manager completed successfully. R4b's whole manager failed on its old axe
window extraction, even though the retained sword child had completed normally.
The accepted three cases are not presented as one all-cases manager invocation.

`native/<kind>/output.wav` preserves the full recorded PCM; its bytes match the
original raw-recording hash. Sword has its original `first-miss-window.wav`.
Axe/daggers separately retain `targets-moved-window.wav` and
`zero-damage-window.wav` (ending at the actual first HP-decline wall observation).
The latter **may include a following attack's prehit whoosh**; window content is
not attributed uniquely to a single swing. Source/sink observations, retirement
and zero HP establish the actual first swing/whiff separately.

The fixed offline matcher uses mono averaging, output/reference onset thresholds
16/64 and a ±300-sample search, unchanged from R3. The dagger correlation is lower
and is reported unchanged; no threshold was tuned to obtain a pass. Cropping
uses recording end minus PCM duration and therefore has approximate recorder /
server-buffer alignment. This is source-content evidence inside the observed
zero-damage/music-muted interval, **not sample-accurate synchronization or a
measurement of device latency**.

**No subjective listening approval is claimed.** Representative native frame 60
for all three classes was actually viewed. The waveform/spectrum plot and
recordings are provided for later listening and art-direction review. No new
standalone package, public campaign run or default slow-frame synchronization
claim is included.

## Retained interruptions and reproduction

`history/` keeps all raw text logs, managers, process results and input receipts,
including compile-invalid attempts, the intermediate HP/RNG behavior failure,
initial absolute-warmup-clock failure, default-Automatic observation too late to
move targets, R3's real isolated sword recording followed by premature 5-second
verification, R4's Clippy argument-count failure, R4b's axe extraction failure and
the deliberately stopped R5 build after recursive formatter noise. The latter
noise affected only equipment UI formatting and was reverted before R5b; it is
not part of the final source. Earlier managers with no child-result JSON are not
assigned an invented exact child exit code.

The oldest `native.py` was edited to add a prospective gate after its process had
already loaded; that current file is not claimed as a frozen executed-script
snapshot. Its actual raw logs and source/build receipts remain. The accepted
R4b/R5b scripts and final source snapshots are frozen in this archive.

Run `python3 docs/verification/melee-whoosh-2026-11-06/verify.py` for offline checks.
`source/` contains exact scoped source/assets. `original-files.json` indexes every
original target file, including raw PCM, failed PNGs and large capture binaries
not duplicated into Git; the originals remain under
`target/verification/melee-whoosh-implementation-2026-11-06`. All raw logs are
copied, not summarized away. This proof does not promote historical failures.
