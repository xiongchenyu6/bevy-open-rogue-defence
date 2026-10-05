# Campaign progression

Six expeditions each retain 25 waves, five bosses and the final 60-second archive encounter. Enemy health increases by `1 + 0.65n + 0.18n²`, damage by `1 + 0.25n`, and XP/gold rewards by `1 + 0.2n` (chapter index n=0..5).

Clear a chapter to unlock the next. A first clear grants extra shards; repeat clears and defeats also grant bounded rewards. Each run settles once. Spend shards on permanent strength, agility, intelligence, health, attack and spell power. Each has ten upgrade levels with increasing costs. Bonuses apply once after choosing a starting hero and remain through recruitment.

The chapter screen and training screen are available at launch and after settlement. Continue, retry and chapter selection reset the entire run inside the application while retaining campaign progress and assets. Learned skills, bond cards, hero fusion, temporary effects and challenge progress belong to the run and reset.

Progress is saved atomically in the native configuration directory, or in browser local storage. Invalid saves are retained without overwriting them. Screenshot fixtures use isolated saves.

The gate is a forward tank with unlimited full-health respawns after four seconds. Gate destruction does not settle a defeat. A run ends in defeat when the hero exhausts the last of three lives; life-saving skills and items intercept lethal damage first. Restart resets both hero lives and gate revival state.

城门拥有有限耐久，正常承受伤害；“无限”仅指重建次数。城墙横跨道路，倒下时隐藏实体并在原地显示4、3、2、1倒计时；归零满耐久重建。
