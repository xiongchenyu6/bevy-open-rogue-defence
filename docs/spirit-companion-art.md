# Spirit companion art

Spirit companions previously rendered the shared 0.16-radius projectile sphere at scale 2.3, using the Spirit Keeper hero's purple material. This was placeholder geometry, rather than a modeled companion.

The replacement is an authored hooded spirit with a recessed mask, pale eyes, a curling tail, and two cloth ribbons. One vertex-colored mesh and one matte material are shared by the existing summon pool. Local dimensions are 0.70 × 1.15 × 0.445; companions use scale 0.95. The model faces local −Z, matching production pursuit and idle facing.

An initial native capture confirmed the new silhouette, but exposed overlapping ultimate companions, large artificial ground shadows, and unchanged purple sphere shots. The final revision spreads newly summoned ultimate spirits around a 0.9-radius formation, excludes souls from ground projections, and uses a small traveling spirit silhouette for their projectiles. Projectile damage, collision radius, lifetime, and firing cooldown remain unchanged. Projectile pool reuse clears the ground-projection exclusion so it cannot leak into other weapons.

## Verification

- Full library suite: 579 passed, 0 failed, 1 existing ignored.
- Strict all-target Clippy and formatting checks passed.
- Geometry tests verify dimensions, finite normals, valid indices and winding, eye direction, and the three-dimensional tail.
- Production ECS checks verify dedicated mesh/material selection, shadow exclusion, pause/pursuit behavior, and projectile pool reuse.
- Fresh baseline capture reproduced the original purple ball through real summon casting and actual damage.

Final gallery capture completed 300 frames, including real passive/ultimate summons, actual enemy damage, and a minimum pairwise horizontal spacing above 0.6. Combat and idle screenshots show the authored forms without the large circular ground shadows or purple sphere shots. A further 680-frame capture verified pursuit and real damage across six summon families, 29 paused frames with unchanged transforms, target replacement, outside-wall facing, and finite-health wall reconstruction. Both captures exited successfully. Four final images were reviewed. Evidence is archived in `verification/spirit-companions-2026-10-08/`. Nonfatal long Wayland socket-path and offscreen ShadowLodOrigin warnings remain in the logs. The capture harness uses software Vulkan and does not establish hardware frame rates or final Steam release readiness.
