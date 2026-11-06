# Prepared only — next standalone Linux package

No build, container execution, new profile, package write, source/docs/Git edit or route launch was performed. Current candidate HEAD is 8e0b9cbfc166f7eb60ff52d80fa898b6b21aee1d; it is informational, not a frozen final pin. All authorization fields are false.

The actual SDK image is present: `rogue-commander-linux-sdk:rust-1.98.0-sniper`, ID `sha256:5e47fb3110569ab39fac6568db05c900d372e95110a5fc795f8ca6c7ed11d2cb`. Its Dockerfile pins Steam Runtime sniper SDK digest and Rust 1.98.0. This preparation inspected metadata only, not a newly executed SDK compiler. Existing target/steamrt-build (2.1G) and target/steamrt-cargo (360M) caches remain untouched. Offline-smoke Ubuntu24 image is present with ID `sha256:0d4cccee982b31492e0f7b01a50afc1a0ec710d3166dab22e02322ce64832715`.

## Actual blockers

1. Attack owner still has tracked combat.rs/skills.rs/tidal_wave.rs changes and untracked src/game/combat/wave_order_tests.rs. Final accepted source must be committed and Cargo/source lane released before selecting the exact package commit. The input audit deliberately rejects this current state.
2. Root must pin that exact final SHA and release build/smoke authorization. There is no current binary/manifest/tar hash for the unbuilt candidate.
3. The optional short earned-profile migration route now has a target-only `ffab_restart_runtime.py` and `earned-restart-command.json`, syntax-checked only, pending root review. Existing earned_restart_runtime.py hardcodes the old 2/180 checkpoint and spends a shard upgrade; forge_restart_runtime.py hardcodes da5 Forge state and requires its predecessor package version. Neither can be relabeled as accepting the earned-final-C6 checkpoint. No driver was executed and no state/output directory was initialized; this is not a blocker for the already command-ready build/basic smoke.

No missing SDK or runtime image was found. The two untracked demon-art tools are outside declared SDK/package entrypoints and remain the only explicit untracked exceptions; new untracked src/assets/vendor/.cargo/licenses/build-tool input is rejected. Rejected cave prototype art is not a runtime asset or acceptance requirement.

## Exact build / smoke recipe after final GO

1. Record final `git rev-parse HEAD`; require all tracked bytes/index match it. Set expected_source_commit to that full SHA and build_and_smoke_authorized=true in this target's authorization.json; public_routes_authorized remains false.
2. Launch the reviewed manager once, in a fresh destination:

```
python3 -u target/verification/polish-next-package-2026-11-06/build-and-smoke.py FINAL_FULL_COMMIT
```

Durable wrapper: `systemd-run --user --unit=rogue-polish-next-package-nov6 --property=RuntimeMaxSec=3200 --working-directory=REPOSITORY /bin/sh -c 'exec python3 -u target/verification/polish-next-package-2026-11-06/build-and-smoke.py FINAL_FULL_COMMIT > target/verification/polish-next-package-2026-11-06/manager.log 2>&1'`.

The manager invokes existing `tools/build_desktop_linux.sh target/desktop-packages-polish-next-nov6` (1800s), uses the pinned Steam Runtime Dockerfile, workspace RO plus existing separate Cargo/build caches and a new writable /out, locked release rogue_commander -j2 and target-filtered Cargo metadata. It never overwrites an old package. SDK build network behavior remains the existing script's behavior; only the public runtime smoke is explicitly offline.

3. Before/after SDK input audits enumerate tracked root inputs plus src/assets/vendor/.cargo/licenses/tools and player README, compare each physical byte/symlink against `git cat-file` at the exact pin and SHA256 both phases. Ignored Python caches are allowed only under tools; ignored runtime assets are not exempted. The two art-tool hashes are recorded and excluded from declared entrypoints/delivered package, not falsely claimed physically absent from the RO workspace.
4. Verify version.git_commit, every package-manifest file hash and source asset, readelf linkage with zero `/nix/`, original d454 binary/manifest baseline, and actual delta. From current committed assets the expected addition is three WAVs (886 -> approximately889 manifest files), but the manager records actual count rather than forcing it. SOURCES.md and binary/version should also change. No final-art asset adoption is implied.
5. Build deterministic sorted/mtime0/uid0/gid0 tar+gzip-n, record binary/manifest/tar SHA and sizes. Freeze all six public helpers from `git show FINAL_SHA:tools/desktop/...` with hashes, not mutable worktree files.
6. Run the existing smoke driver in the cached Ubuntu24 image: `--network none`, `/game` RO, frozen `/harness` RO, fresh `/verify` RW, workdir /tmp, `PACKAGE_COMMIT=FINAL_SHA`. Limit600s. Assert no source/Rust/Nix and actual readiness/guide/recruitment/battle/settings/fullscreen/EN->ZH persistence/restart/normal quits. Preserve actual child status, result booleans and view English battle + restarted Chinese battle. A package build or wrapper0 is not a smoke pass.
7. Root independently hashes package/tar, runs archive verifier and views actual screenshots before any route authorization. Do not rename the d454 smoke result as a new-source result.

## Minimal next route, not another automatic full C6

Use the actual final save from `docs/verification/wallet-blades-chapter6-2026-11-06/campaign-final.save`, SHA `ffab0a0d890e9ddd012de5516a7ef6de6a444a7b48108a1ae0fcecd43433cfbb`: six chapters cleared, currency630, upgrades[7,0,0,0,0,0]. Its actual d454 provenance includes A11CF victory, +240 then a real255 Strength purchase. Copy only those exact save bytes into a *fresh isolated* profile when explicitly authorized; no tutorial/audio/config/resource/stock/ECS injection, no synthesized save. This short public route would verify menu630/sixcleared, observe/skip the real first guide if present, recruit actual observed Storm caption, 1x battle, Strength34, empty per-run inventory, strictly advancing countdown, unchanged save bytes and normal quit0. Proposed180s driver/300s outer; do not mask failures by enlarging it. It is earned-save compatibility and basic UI acceptance, not a new-source chapter-clear claim.

The original initial C6 checkpoint remains SHA `4c9a0973fa90b3f75838ccb1999267be31ff8ff9b896a8c9b165ae5ac3e129f9`, 6/645/111110/upgrades6. If a fresh full chapter/economy claim is later explicitly needed, reuse the strict all-owned route with this genuine checkpoint,1x,5280 internal/6000 outer/6200 unit. Preserve requestedA11CE vs actually rendered seed; do not assume replay of old A11CF. No long route is automatically authorized or initialized here.

Old d454 natural Forge + same-profile restart and complete C6/reward/upgrade/restart remain real but source-specific. New trails/Storm/first-punch/whoosh/Orbit source tests/native evidence support their own limited claims; package file inclusion plus a short basic smoke does not prove late-game Storm/Orbit behavior in the new package. A new full route is warranted only for a new terminal claim or a concrete gameplay/progression regression, not merely to copy a historical checklist.


## Prepared ffab driver review

`ffab_restart_runtime.py` adapts the existing Forge restart driver but validates the actual previous d454 C6 victory/training proof rather than forging a new-package predecessor. It independently checksum-validates the untouched saved bytes and verifies public menu630 plus six individual CLEARED status crops at the actual committed1280 layout, the observed first guide, actual Storm caption, Level1/Strength34/1x, empty inventory, real shared close proof, advancing countdown and actual quit0. It does not choose skill/bond/evolution queues: unexpected grant selection fails, rather than silently consuming new rewards. It never buys/trains or writes the save directly. Public mouse/key UI automation is used; no synthetic ECS/Input resources or preference files are injected.

After package/basic smoke review and a separate route GO: create the two new empty output/profile directories listed in plan.json, copy only the exact committed campaign-final.save to profile/campaign.save and verifyffab SHA, then snapshot six helpers from the exact final source plus this target driver's reviewed SHA into output/harness-source. Execute precisely earned-restart-command.json after replacing the finalSHA placeholder, with180s driver/300s outer. Bind /game and /harness RO and /previous-proof RO; state is the only genuine starting progress and actual game-created preferences are allowed thereafter. Assert actual child0/resultPASS/630+sixcleared/Strength34/gear0/resume/savehash equality, retain all actual screenshots and process status; no automatic retry or budget expansion. This route is not launched here.
