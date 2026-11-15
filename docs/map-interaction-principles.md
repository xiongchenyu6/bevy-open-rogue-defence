# Map interaction principles

World objects are their own controls. If a player can point at a monster, camp,
structure or reward on the map, use that object as the interaction target.
Do not add a second HUD button for the same action. On mobile, add a permanent
button only when the action cannot be represented clearly in the world.

- Make the object readable through pose, light and a small contextual marker.
- Keep locked/ready/fighting/cleared states in the scene. Do not display permanent
  labels or countdowns over every object; reveal details on demand.
- A map tap previews a risky optional encounter. Let players inspect its skills,
  rewards and risk, then challenge or leave without consuming the opportunity.
- Use the visible object plus its footprint as the touch target. Do not move an
  offscreen object's target to a detached screen-edge toolbar.
- Activate scene actions on release. Dragging, cancellation, HUD interaction and
  movement-pad gestures must not trigger them.
- Show achievement in the world: a victory marker requires an actual victory,
  not merely a dismissed or resolved encounter.

Optional camp implementation: scene-anchored hit areas replace the right-hand
shortcut rail; locked camps show their remaining unlock time only on inspection;
ready camps show a small sword marker; challenge previews retain the safe reward
option and add a non-consuming Later action.
