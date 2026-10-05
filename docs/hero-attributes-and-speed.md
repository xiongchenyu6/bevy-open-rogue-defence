# Hero attributes, battle speed and orbit blades

Click the hero portrait in the top HUD to open the live attributes sheet. The sheet pauses combat; closing it or pressing Escape resumes combat. English and Chinese use the same live values. The sheet scrolls by dragging or using the mouse wheel. The desktop panel uses the available height so absorption records do not hide the penetration rows. A growth-source section explains where equipment bonuses come from.

The sheet groups primary attributes, offense and defense. It includes actual basic-attack damage and rate, attack range, critical chance/damage, lifesteal, physical/spell penetration, armor/reduction, magic resistance, evasion, spell/summon power and summon capacity. Black Market equipment provides the secondary attributes: Hardened Plate adds armor and maximum health; Wind Boots adds evasion; Blood Contract adds lifesteal; Serrated Disc adds armor penetration; Arc Lens adds magic penetration; Ember Heart adds critical chance. Armor penetration is displayed in armor points, while magic penetration is displayed in percentage points of resistance. The sheet uses the same live values as combat, including equipped bonuses retained through hero absorption.

The automatic ultimate continues to cast in combat. Its former HUD button, cooldown indicator and tooltip have been removed.

The three speed controls decrease, increase and reset to 1×. Available speeds are 1×, 2×, 4×, 8×, 16× and 32×; both ends clamp. The reset control displays the current multiplier. T increases one step.

Orbit Blade uses a metal chakram with four broad curved teeth, luminous cutting edges, self-rotation and a bounded wake. Rank 1/3/5 has 2/3/4 blades at scale 1/1.26/1.52. The existing orbit radius and per-target hit cooldown remain the combat rules.

Capture scenarios: `hero-attributes` exercises real speed controls, both bounds, reset, live attribute values, pause, close and Escape; `orbit-blade` exercises ranks 1/3/5, distinct meshes, growth, wakes and actual damage.

## Desktop and touch controls

Input mode is independent of viewport size. Native desktop starts with keyboard controls: no virtual joystick, and a compact skill/dodge bar. Android/iOS and browsers whose primary pointer is coarse start with touch controls. A real touch switches to touch controls; a subsequent keypress restores keyboard controls. Window resizing only changes available spacing, never selects phone controls by itself.

`hud-input` captures desktop → touch → touch release → keyboard transitions at both desktop and phone-sized viewports, including real keyboard movement and control bounds.
