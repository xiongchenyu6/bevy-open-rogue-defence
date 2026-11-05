# Plain initial archer projectile

Levels1–9 Ember ordinary arrows now use one merged vertex-colored wooden shaft, small iron head and short feathers, with one static non-emissive rough material. The original large gem head/barbs remain at level10+. Mechanical bolts, support arrows and signature release geometry retain their original paths. Collision radius, velocity, damage and volley targeting retain their values.

The blue flight circle came from the generic ground-energy projector treating the radius.48 ordinary arrow as an energy projectile (threshold.4). Actual starter ordinary and inherited Ember shots now use the existing NoGroundProjection marker. Pool rental already clears that marker for reuse. No collision size was reduced to suppress the effect.

## Evidence

Optimized full library **663 passed, 0 failed, 1 ignored**; strict optimized all-target Clippy passed. The initial-arrow mesh/color/tier/support boundaries are tested; the real ordinary-shot test checks levels1/9/10/20/40, one target/one projectile, actual no-projection tag, velocity/radius/damage. Inherited emission uses the same explicit tier predicate in its production path; no separate inherited-arrow native scenario is claimed. Static startup budget is14meshes/61materials.

Five native after scenarios completed250updates each: ordinary Ember, starter single-target, upgraded single-target, upgraded three-target and sword control. Three new growth baselines completed250updates; ordinary Ember and sword baselines reuse the just-verified previous sword-trail binary recordings (same native binary hash before this edit), not new before runs. All400 synchronous first80frame damage/HP/animation observations match exactly. Native first-volley checks observe1arrow for either single-target tier, and3arrows for the upgraded3target fixture. Starter material intensity stays low; level40retains its upgraded emission.

Root inspected plain-arrow flight with the ground circle absent, upgraded single-target flight, and the previous baseline arrow. Native screenshot requests are asynchronous; synchronous logs establish damage/timing. This is fixture rendering/behavior evidence, not natural progression, hardware performance or complete combat-feel acceptance. The release candidate32cf594 predates this change.
