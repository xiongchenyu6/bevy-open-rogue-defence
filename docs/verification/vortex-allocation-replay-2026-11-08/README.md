# Vortex allocation full-trace pair

Both immutable same-source cases completed normally and won at1508.875 simulation seconds after93642 frames. Source is e344e68ff21dc9c81b432e33494c2d5a0b739bdb. The frozen binary, assets, runner and211 compiled inputs are pinned. Unit invocation809427bfac0840038942c83375921deb ended success/main0. Actual process receipts are retained.

Strict comparison exited1 and full_recorded_match remains false. The first numeric actor difference is frame83417. After only explicit Entity identity removal, one projectile velocity-z field has bits1120683039 versus3268166687, corresponding to+102.13304901123047 and−102.13304901123047. This is a real velocity direction difference, not signed zero. The complete first-difference context is retained. Earlier raw Entity identity/order differences are also preserved in the comparison report.

All10218 contact records per case agree after removing only projectile/enemy identity fields. Both outer_ordinal and candidate_ordinal, RNG, progress, elapsed, profile, damage and HP remain compared. All202 ordinary records per case agree after root identity normalization and runner_metadata exclusion. These include54 checkpoints. Terminal outcomes and checkpoint statistics match, while strict full recorded state does not.

A read-only source audit identifies symmetric ricochet candidates and an Entity tie-break as a candidate explanation. The corresponding new regression is pending. This proof does not establish a global root cause or replay fix. The separate local Vortex allocation regression remains linked at ../vortex-allocation-2026-11-08/README.md.

## Portable and local evidence

This archive retains original ordinary JSON lines and original contact stderr lines as lossless filtered extracts. Extraction rules, source paths, full-file hashes, selected row counts and decompressed extract hashes are explicit. These are extracted portions, not complete original logs. The full JSONL/stderr/binary and frozen asset byte hashes remain in local-bulk-manifest.json. Full raw traces and binaries stay local and are not copied to Git.

Portable verification independently recomputes the ordinary/contact comparisons, exact first-context actor difference and strict false result. It checks process/source pins and archive integrity. Actor93642-frame coverage is retained from the original complete comparison receipt. The full original trace comparison is not independently rerun from this bounded archive.

Run `python3 PATH_TO_ARCHIVE/verify.py PATH_TO_ARCHIVE`. Add `--local-root PATH_TO_REPOSITORY` to hash every listed local bulk file read-only. This optional mode checks local byte availability and identity without launching or replaying the simulator.
