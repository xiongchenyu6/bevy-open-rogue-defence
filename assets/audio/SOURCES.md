# Project audio sources

These audio files are original procedural syntheses, without external recordings
or sample packs. `tools/generate_audio.sh` generates shot, impact, draft, ultimate,
boss_spawn and arena_loop from sine/noise filters with FFmpeg.
`tools/generate_hit_audio.py` generates the hit, critical, kill and hurt cues from
transients, deterministic noise and pitched bodies. `tools/generate_bond_chime.py`
generates the bond-completion cue. The scripts are retained in the project.

FFmpeg is a development tool used to create the files; it is not shipped in the
game package. No third-party music recording is included. These original project
sounds retain the project's ownership; this document does not relicense them.

`tools/generate_melee_whoosh.py` generates the three original sword, axe and
 daggers swing cues from seeded noise, swept one-pole filters and a smooth
rise/release envelope, without external samples or paid services. The PCM RMS
peak windows are approximately 50/60/20 ms after onset respectively; runtime
windup audio uses these real-time leads. These are procedural candidates;
waveform/decoding checks do not establish subjective listening approval.
