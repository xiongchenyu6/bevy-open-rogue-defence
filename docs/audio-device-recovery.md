# Audio device failure and recovery

Repeated `rodio::stream` / ALSA `POLLERR` messages come from the audio output worker. The pinned CPAL ALSA implementation continues its worker loop after a poll error, and Rodio's default callback logs each error. Hiding that log alone would leave the failed stream running.

The local patch to the same Bevy Audio 0.19.0 version installs a fault callback for every device/configuration fallback. It records the first warning and marks output unavailable immediately. The main thread then drops the broken stream; dropping it inside its own callback would attempt to join that same worker thread.

Recovery attempts run every five seconds of real time, independent of game pause or battle speed. Existing audio sink components are removed when the stream fails. Music entities and their playback settings remain, so the recovered mixer can start the loop again. Music restarts from the beginning. Settings show a localized device-unavailable notice while reconnecting. The game discards queued effect voices during disconnection and consumes suppressed ultimate/crafting cues, preventing a burst of stale sounds on recovery.

No sound service, system device configuration, or global logging filter is changed. Device opening and stream shutdown still run synchronously on the main thread. The code cannot repair a persistently broken operating-system audio service or guarantee recovery until a usable output device returns.

## Verification

- Send 10,000 synthetic POLLERR events through the production callback; exactly one warning and a failed status.
- Bind device maintenance to the main thread; verify the actual system is non-Send.
- Verify a failed stream drops once, retries wait five real seconds, unsuccessful attempts wait another five seconds, and initial device absence also retries.
- Exercise game ECS cleanup: effects disappear, music/settings and gameplay entities remain.
- Exercise actual feedback systems: unavailable output produces no queued shots and consumes old ultimate cues.
- Run the library suite, strict all-target Clippy, and formatting checks.

All five recovery tests pass; the complete vendored backend suite also passes all 17 tests. A first complete game run exposed two missing Chinese glyphs in the new notice; the pinned Noto Sans SC subset was rebuilt with the updated localization corpus. The corrected game suite passes 575 tests (0 failed, 1 existing ignored). Final source and dependency checks are archived in `verification/audio-device-recovery-2026-10-07/`.

These controlled tests do not reproduce the user's physical ALSA device failure or prove audible recovery on that device. Native device-loss and reattachment verification remains outstanding.
