# Current standalone package preparation

Execution is disabled pending root final full-SHA GO after UI acceptance and commit. Unique destination is target/desktop-packages-current-nov7. No old package comparison or historical source hash is required.

After root authorizes the exact final SHA, launch this manager from repository root with that SHA as its sole argument. Use a durable unit with RuntimeMaxSec3200. The manager limits SDK build to1800 seconds and offline smoke to600 seconds. Exit status propagates through the manager.

The input audit requires tracked HEAD bytes clean, enumerates source/assets/vendor/config/licenses/build tools and README, rejects undeclared inputs including ignored files, and records only the two established art-tool exceptions. It records before/after input hashes, validates linkage without Nix, exact manifest/package/tar bytes and normalized tar metadata, then freezes all six helpers from the exact final commit. The smoke uses a pinned actual image ID with network none and read-only game/harness mounts. No Docker build, Cargo, source mutation or runtime execution has occurred during preparation.
