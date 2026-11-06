# Current standalone Linux package and offline UI smoke

Actual compiled source: **0c12230d3bf25c2b1a070c80950e5388a3b7b2ec**.
This advances the old d454 package with accepted melee trails, Storm/Orbit/tidal
contact ordering, first-hit punch and three original melee windup sounds. The
optional cave captures do not adopt rejected/prototype cavern art as runtime
assets.

- Actual Steam Runtime SDK release build: exit0,124.367s,Rust1.98.0.
- Before/after525 declared tracked input bytes match the exact Git commit and
  each other, including vendor/.cargo/licenses/build tooling/player README.
  Only the two named untracked art tools are explicit exclusions; their bytes
  remain physically in the workspace RO mount but are neither declared SDK
  entrypoints nor delivered package files. No ignored runtime assets are exempt.
- Package:889 manifest entries,890 actual files including the manifest; every
  byte/size and deterministic tar uid/gid/mtime0 independently checked by root.
  Readelf finds no Nix runtime dependency; missing license files=[].
- Binary SHA256:`b19b938f0f058c90bca35edec95cb87f39fe47f77e9e0d14720bdda012f45d91`.
- Tar60,931,013bytes, SHA256:`019aff09b4d6ee63df2503628acb2b179216995ecfdcb32ea61c932d6c34bb23`.
- Offline actual UI smoke: exit0,111.633s; readiness/real first guide/basic
  recruitment/battle/fullscreen restoration/English-to-Chinese persistence/
  restart battle/two normal quits all pass. Root and executing agent actually
  viewed `smoke/battle.png` and `smoke/restart-battle-chinese.png`.

The image is the cached Ubuntu24 X11/software-Vulkan runtime, `--network none`,
RO/game and RO/harness, fresh RW/verify, no game Rust source, compiler or Nix.
Six UI helper modules are frozen from the exact compiled Git commit. The SDK
build itself uses the existing SDK script's network behavior; only the public
runtime smoke is explicitly offline.

The package remains at
`target/desktop-packages-polish-next-nov6/rogue-commander-linux` and its `.tar.gz`.
This archive retains the complete manifest/version, actual build/runtime logs,
commands, inputs, harness, result booleans and root's independent
`root-artifact-check.json` plus executed `root-verify-artifact.py`. Root's receipt
was generated before runtime acceptance and deliberately says byte audit only;
actual smoke process/result/screenshots supply the separate runtime proof.

Run `python3 docs/verification/polish-next-package-2026-11-06/verify.py`. It reads
this archive, historical Git input blobs and the retained local package/tar; it
never builds or runs the game. `REVIEW.md` and preparation-status are historical
planning snapshots, not replacements for executed process/result records.

The actual delta from d454 is three new WAVs plus modified audio sources text,
binary and version; no prototype art asset was added. File presence proves
inclusion, **not rendered empty-rack illustration visibility**. That concern has
its own late-frame observation in the separate earned-restart proof.

The short genuine ffab-earned-profile compatibility route is separately archived
at `../polish-next-earned-restart-2026-11-06`. Old d454 full C6 victory/economy and
Forge/restart evidence remains source-specific. This new package smoke is not a
new-source full C6 clear, late-game skill showcase, hardware audio/listening
approval, Steam-client acceptance or global determinism claim. No long C6 run
was repeated for this scope. Root release/readiness documents are not owned by
this scoped archive.
