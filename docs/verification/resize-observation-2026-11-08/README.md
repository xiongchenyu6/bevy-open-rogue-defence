# Actual resize observation bounds

The target-only diagnostic completed on immutable package d629e50 with actual process exit0 in174.829 seconds. Unit invocationbb9fca09684244f7aff9f8a751c95684 ended success/main0. All thirteen smoke outcomes are true, including two normal quits. The game source and package are unchanged.

The frozen six-helper snapshot is based on399caa121a60446450ed8f6513358ac84bc72829. Only the diagnostic smoke helper differs, adding four real360→1280 resize cycles and six screenshots per cycle. Original helper hashes, six baseline files and a target-only patch are retained.

All24 observations record capture start and completion relative to observed geometry. The900×720 right region at+380,+0 is pure white in each first observation, mean1.0 in4/4. Each next observation is nonwhite, around0.26, starting0.336/0.323/0.316/0.322 seconds after the geometry reference. Remaining observations are also nonwhite. Exact timestamps and means are retained in raw resize-observations.json.

Capture start/completion are observation bounds, not exact GPU presentation latency. New X11 geometry preceding complete paint is a possible interpretation. This experiment does not prove an engine cause or game fix, and covers this software Vulkan environment. All24 originals are retained. Root reviewed the first/next pair of each cycle through eight independent JPEGs and a contact sheet. Other sixteen originals are measurement evidence without individual root visual acceptance.

Run `python3 PATH_TO_ARCHIVE/verify.py PATH_TO_ARCHIVE` for exact archive/raw hashes,24 timing records, helper/source pins and actual process results. Add `--pixels` to recompute all24 right-region means using locally available ImageMagick. This optional mode performs image analysis only, without starting a game.
