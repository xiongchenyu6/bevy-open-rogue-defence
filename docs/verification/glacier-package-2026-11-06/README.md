# Linux candidate: Glacier/gate/Cinder corrections and public short flow

Product source `bdd6660f16514a1ba1bd506c3a58f42916183e10` contains the
`3d42d0b` Glacier correction and earlier gate/Cinder corrections. Its source,
asset and vendor bytes match the [740-pass library validation](../glacier-order-2026-11-06/README.md);
that is source validation with the local Nix compiler, not 740 tests rerun in
this SDK or a whole-run equality claim. The development candidate remains
subject to the [retained late replay negative](../post-glacier-repeat-2026-11-06/README.md).

## Build and artifact

Fixed Steam Runtime3 sniper SDK, Rust1.98.0. Actual build exits zero in
108.669 seconds. All 537 audited tracked build/runtime input bytes match the
pinned commit before/after. The two existing untracked art tools are explicitly
excluded from compilation and delivery; their hashes and exclusion basis remain
in the audit. No source was changed during this build or smoke.

- Directory: `target/desktop-packages-glacier-nov6/rogue-commander-linux/`
- Tar: `target/desktop-packages-glacier-nov6/rogue-commander-linux-0.1.0.tar.gz`
- Tar bytes: 62,295,464
- Tar SHA-256: `5bacb0ff3301c69bd90525252a4740c4ff7d690ef038c1e51990971f3e7d409e`
- Binary SHA-256: `f10dd77c63e69916e5cf3471ff94dc1f2cbe1b591f4105977df648b5d18252a8`

Root independently hashes all 892 manifest files and compares all 893 regular
tar members with actual package bytes; ownership and timestamps are zero for all
tar entries. ELF linkage inspection reports no Nix dependencies or search paths.
Relative to the previously accepted f205 package, only the executable and
`version.json` change; all delivered assets and other files have the same hashes.

## Actual independent runtime

The read-only package runs in offline Ubuntu24/X11 software Vulkan from `/tmp`,
with no game source, Rust or Nix mounted. Six harness modules are exact frozen
`git show` bytes from the product commit, separately mounted read-only. They use
public input/screenshot observations, without game resources or save injection.
The actual smoke exits zero in 117.161 seconds and the supervising unit is
terminal inactive/success/exit-zero.

It completes asset readiness -> first guide -> recruitment -> actual running
battle, Chinese settings, fullscreen restoration, preference restart, recruitment
and running battle again, and two normal application quits. Clock endpoints
strictly decrease in both launches: 1498 -> 1492 and 1487 -> 1481, with matching
reads at two image scales. One intermediate restart clock read is unreadable at
one scale and is retained as rejected observation; subsequent consistent reads
satisfy the unchanged bounded observer. No guessed punctuation/number replaces it.

Root viewed `battle.png`, `settings-chinese.png` and
`restart-battle-progress-02.png`: English and Chinese scenes are readable, the
hero is visible on the gate's safe side, and settings are not clipped. This short
flow does not drive the hero through the wall to re-prove cutaway behavior. The
settings screenshot reports no available audio device and automatic reconnect;
there is no physical-output or listening acceptance claim here.

## Limits and verification

This is an unpublished development candidate, not Steam acceptance, hardware
performance testing, all UI states, all heroes, a full chapter/campaign, natural
forge progression, earned-save migration or global determinism. The older earned
routes remain evidence for their own exact versions, not this package. Earlier
controlled cave/camera/cutaway visual proofs remain scoped to their own fixtures.

`verify.py` validates archived receipts/hashes from any working directory.
`--artifacts` additionally checks the retained local package and tar; those large
artifacts are not stored in Git. The archive is read-only evidence, not a rerun.
