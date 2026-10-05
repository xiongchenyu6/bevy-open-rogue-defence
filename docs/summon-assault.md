# Active summon combat

Mobile summons seek living enemies, face them, move into attack range and then attack. When a target dies, they select another; with no enemies they return to a dispersed formation outside the gate, facing the enemy entrance. They cannot move behind the gate.

| Summon | Behaviour |
| --- | --- |
| Skeleton / spirit hound | Close to melee range and strike |
| Tide elemental / frost wyrm | Advance into spell range before firing |
| Ancestor colossus | Close to slam range, pull nearby foes and slam |
| Spirit companions | Leave the gate formation and advance into ranged combat |
| Bolt turret | Deploy on the gate wall; stop firing and disappear while the gate is destroyed, then resume after reconstruction. The summon still expires after 16 seconds. |

Movement stops at attack range even at accelerated game speed. Chasing does not accumulate attacks. Pausing freezes movement, attack timers and summon lifetimes. Existing skill ranks, growth, damage profiles and object pooling remain authoritative.

## 城门外防御阵列

所有移动召唤物出生与待命位置都在城门外侧，待命朝向敌人入口（+X）。无敌人时归位，不再围绕后方英雄；追击也不得越过城门进入后侧。雷弩塔为固定城墙塔，部署于城门顶部；城门损毁时不能射击，重建后恢复。暂停时移动和攻击冻结。
