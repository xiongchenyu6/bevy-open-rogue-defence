# Skill grounding

Solid summons and flying weapons receive ground contact projections. The arena's directional light also casts physical geometry shadows. Fire, ice/water, lightning/arcane and infection effects illuminate the ground in their element color; transparent energy does not cast an opaque silhouette.

Projection position, orientation and scale follow the current rendered world transform and real skill growth. The fixed pool contains 80 projections with shared meshes and materials; companions, solid weapons and large skills receive priority. No per-particle lights are added. Pause freezes projections, inactive pooled effects hide them, and expedition completion hides all projections.

Native capture verification covers every skill and selected skills in the production camera on desktop and touch viewports. This does not establish browser rendering or performance on a user's GPU.
