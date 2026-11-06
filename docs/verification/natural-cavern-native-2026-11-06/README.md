# Cavern candidate: native technical checks pass, artwork rejected

The d058912a candidate and the wide camera proposal were **not adopted**. Formal
assets and the production camera remain unchanged. Only the optional capture tool
adds `den-preview-wide`: eye (15,24,21), focus (7,0,0), vertical height 26. Existing
production, shifted and front proposal settings retain their prior values.

A frozen native capture binary ran the candidate twice in the actual arena with
actual lighting, HUD, gate and camps. Each case completed 400 updates, saved all
nine requested PNGs and exited 0. The existing preview assertions checked removal
of the old den subtree, candidate import at the original parent transform, and
three live maximum-scale fixture enemies moving out along the real road. These
are controlled actors, not an earned campaign or a balance/performance test.

- [Candidate with production camera, quiet](production-camera/frames/frame00090.png)
- [Candidate with wide proposal, quiet](wide-camera-proposal/frames/frame00090.png)
- [Wide proposal with moving actors](wide-camera-proposal/frames/frame00180.png)
- [Wide proposal after emergence](wide-camera-proposal/frames/frame00350.png)

Root viewed these four images. The production camera crops the candidate's mouth
and body. The wide proposal shows more of it but partially overlaps the hero with
the bond HUD. More importantly, the actual moving actors appear to the left of
the visible recess; its CPU forecourt interpretation did not produce a convincing
in-game emergence. The model still reads as a broad flat stone face/eye cavity,
with a conspicuous rectangular base. No technical PASS upgrades these visual and
spatial failures into accepted game art.

The same frozen binary also ran the **original den** for 320 updates, with seven
requested images, exit 0 and its existing quiet / real SpawnDirector-wave checks:
[original quiet baseline](original-den-baseline/frames/frame00029.png).
This is distinct from `production-camera`, which means the *new candidate* under
the existing camera. The original baseline and candidate moving-actor fixtures do
not have identical enemy populations; they cannot support a performance or damage
comparison. The quiet views only support qualitative scene review.

## Inputs and technical evidence

`REJECTED-candidate.glb` is a verification artifact, not a runtime asset. Its SHA
is d058912a9f56a9db4dfaf86d8ce8c03d08de855ed73faa556058a2ed38c900cf:
2,470,092 bytes, 15,064 triangles, embedded 1K baked image, real UVs, opaque PBR and
roughness approximately 0.97. Root independently reran the author's exported-mesh
SAT/payload checker before the native tests. Its finite sampled-animation swept
path has zero reported collisions; the 52 original model files were unchanged.
The author reports a closed single core component and 77.05% inner hits in its
fixed view ROI. These are technical measurements, not visual acceptance or a
formal all-animation interpolation bound. The large / below-ground backing volume
is one reason actual whole-scene review was required.

The isolated asset clone contains all 188 tracked current assets plus one
candidate, 189 files total. Before/after source and asset hashes were checked;
the original asset bytes and default camera were not edited. Source commit
94722df supplies the unchanged inputs; `capture-demon-cave.rs` records the sole
capture-source override. Source/binary receipts and exact commands are retained.
The large executable stays at the pinned target path rather than being duplicated
in Git. The recorded binary is not a new standalone release package.

The author's original Blender wrapper omitted `--python-exit-code 1`. That wrapper
alone was insufficient evidence; the final candidate gate instead checked actual
JSON, Python subprocess status and an explicit Blender failure code. Its report
also records decorative-bound and floating-point comparison adjustments; the
actual actor SAT was not weakened. Root's independent check is archived in full.

Run `python3 docs/verification/natural-cavern-native-2026-11-06/verify.py` to check
archive hashes, original Git inputs, case exit/PNG counts, native contracts and
candidate payload/SAT receipts. This does not automatically judge the images.
