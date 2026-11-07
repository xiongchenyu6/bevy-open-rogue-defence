# Decision phase ordering and capacity boundary

The tested source is represented by retained source snapshots and the 401-entry input map. The separately distributed package remains source `1404e94`. These tests and captures use the newer patched source.

The runtime ordering moves market updates before decisions, the level-up and free-draw prefix before hero growth, decisions before survival and trials after decisions. The existing hero chain stays before decisions so its healing and trigger work is retained. Skill access refresh runs once after hero growth and before progression decisions. Skills scheduling otherwise stays unchanged. Read-only simulator offer telemetry records fusion capacity after Update.

## Actual results

Final after-r3 completed with unit result success and child exit 0. Four focused tests, 786 library tests and six simulator tests passed. One existing library test is ignored. Release and development-capture strict Clippy, capture build, two native captures, simulator build and two complete simulator runs exited 0.

The true capacity regression uses normal G selection, queued G, a level-ten hero interruption, hero selection and resumed G. Before correction, six owned skills and fusion capacity seven coexist with stale evolution capacity six, producing two replacement choices. The exact capacity assertion fails 6 versus 7. The corrected test passes capacity seven. Each final simulator run independently joins all twenty skill hands to its picker entry and confirms equal picker and fusion capacity.

Earlier attempts remain separate. Before R1 omitted loading and guide fixture setup. Before R2 lacked explicit headless image-loader readiness. Corrected before R3 passed its G/contact test. After R1 passed two cases and failed the healing fixture, which omitted priming effects and used kills instead of total_kills. After R2 corrected these and passed, then the supported hero/capacity boundary motivated R3. The after G test adds prepare warmup to the passing before case. This history is preserved without an unchanged red/green claim.

## Replay diagnostic and limits

The original picker diagnostic A used an incomplete trace target and failed the missing-contact guard. Its replacement R2 pair has twenty identical picker entries/exits and fifty-four identical checkpoints, while one projected contact differs in progress_bits. The later phase R2 and final R3 pairs still diverge. Final R3 has twenty entries per run with two differing entries, sixteen differing checkpoints among fifty-three, and 10897 versus 10895 contacts. Identity-preserving and explicitly identity-excluded comparisons are both retained. RNG, ordinals, elapsed, progress, HP and damage profiles remain in comparisons. A matching diagnostic pair does not establish a resolution of the historical replay mismatch. Separate stdout/stderr streams cannot establish cross-stream execution order.

Root reviewed final desktop English and phone Chinese frame100 as complete. Both frame120 images are retained as partial rendering observations. The same four selected observations from after-r2 are included with its historical scope. Captures use controlled full-slot fixtures and real production input systems. Natural earned progression, hardware acceptance and a rebuilt standalone package are outside this round.

Run `python3 PATH_TO_ARCHIVE/verify.py PATH_TO_ARCHIVE`. Verification independently checks archive membership and hashes, decompressed raw bytes, process/test receipts, source consensus and capacity joins, and retains the known replay divergence. Local binaries, assets and unselected images are represented by hashes only.
