# 技能卡伤害与真实结算：当前加成遗漏审计

只读源码/现有数字分析，未修改源码、运行构建/捕获。输入SHA见inputs.json。冠冕六阶标题修复只解决6→5显示倒退，不能据此称技能预览已包含全部实际加成。

## 已确认的冰霜骨龙26链路

- skills.rs::cast_learned_skills把scaled_skill_damage(26,rank,...)存入FrostWyrm.damage。
- skills/summons.rs::tick_frost_wyrms以该damage发射真实吐息，明确DamageProfile::for_skill(26)。
- bonds.rs::for_skill按标签得到 **Frost / Summon / skill_index=Some(26)**，不是普通Attack，也不是Reaction。
- combat.rs::projectile_hits先乘bond damage，再暴击、目标类型、英雄距离、market outgoing、奇遇、Chaos，最后目标减免/护盾等。
- market_outgoing_damage_multiplier确实调用top_skill_multiplier(Some26,build)；如果26是最高技能，冠冕使这一乘区为1.30。因此此技能不能因“召唤”而漏掉冠冕。

已见卡面650.8近似来自(80+42×5+0.8×77.2)×1.85=650.756。仅有冠冕、其他乘区1、非暴击且无目标减免时，真实新吐息约×1.30=845.983，显示约846.0。这里77.2是已显示的召唤强度近似，不能拿四舍五入值证明逐bitHP相等；必须由后续真实投射物/结算测试取完整float。

## 原始skill_preview_damage已包含什么

| 项目 | 当前行为 |
|---|---|
| 基础/等级成长/稀有度倍率 | scaled_skill_damage包含 |
| 攻击属性 | 当前build攻击，扣除Triune临时attack_bonus后参与公式，以防重复 |
| 法强 | effective_spell_power，已含市场法强倍率 |
| 召唤继承 | (70%有效法强+50%英雄攻击)×当前市场召唤继承倍率 |
| 三仪类别增幅 | triune_skill_multiplier已包含，不可再乘一次 |
| 技能7 | 特例0.60×build.attack_damage，不能套普通稀有度基础公式 |
| 技能18 | 特例infection.damage_per_tick，rank内部钳1..5 |
| 20/21/28/29 | preview返回None；UI另写utility/命运各结果，不能统一套伤害乘区 |

## 已知遗漏：可精确获取的玩家侧乘区

1. BondRuntime::damage_multiplier(profile)：全局/永久伤害、更多伤害、召唤伤害、元素/特定元素、Attack类增幅、skill_index存在或Spell类的技能伤害，以及仅实际melee/arrow标记触发的类别增幅。骨龙26应走召唤+冰元素+技能分支，而不是普攻/近战/箭矢分支。
2. market_outgoing_damage_multiplier：Threefold(Reaction例外)、Reaction专属元素反应倍率、当前低血契约、最高技能冠冕、LastOath当前激活倍率。26不是Reaction，不能给它乘反应倍率。冠冕按现build最高rank并用低index稳定破同rank；不允许只因持有冠冕就对所有技能×1.3。
3. EncounterState::outgoing_damage_multiplier：已选奇遇带来的当前全局伤害。

现有卡面“Values include active power bonuses; before enemy armor/resistance”范围过宽：只继承了一部分属性/公式倍率，没计上述玩家侧结算增伤。至少应明确“基础伤害/当前属性”，避免把650.8称成已包含冠冕等所有当前增伤。

## 不能伪装成固定卡值的上下文

- 暴击：实际随机roll；召唤还用基础5%+英雄暴击率×召唤继承。不能把暴击倍率恒乘到卡值，也不能让查看卡片消耗RNG。
- 目标普通/首领/爬塔加成；距离增幅用**英雄到敌人**距离，而非骨龙到敌人的距离。
- Chaos有0.7/1.0/1.8结果；如显示区间，应复用chaos_damage_bounds，不采随机结果。
- 首领通用减伤/元素或火抗/冰伤承受倍率、穿透；物理护甲与炮弹壳额外减免；精英护盾吸收、生命截断与特殊复生。卡片没有锁定目标，不应宣称固定实际扣血。
- 有限召唤物26的damage是出生快照；growth只更新其尺度，只有永久骨卫4会每帧刷新damage。新卡预览应指新施放/新召唤的伤害，不保证已在场骨龙立即改成该基础数值。

## 禁止盲乘的实际反例

命运29的雷击/火雨实际使用DamageProfile::new(Storm/Flame,Spell)，没有skill_index。命运召出的SpiritHound存source_skill29，但当前tick_hounds发射硬编码for_skill5。故按实际现状，29的这些伤害不会因“最高技能29”自动获得Crown Some29加成；这可能是另一个来源元数据设计问题，不能让卡面先假装机制已改正确，也不在本轮修正战斗源码。

技能7实际发射for_skill7，未额外as_arrow；不要因名字叫穿透箭雨就在预览另加箭矢增幅。技能描述语义与伤害metadata的差距应单独归档/验证。

## 建议的最小下一步

保留原始skill_preview_damage作为基础公式，不污染所有施法/召唤创建处。仅在详情层新增一个小的、只读的“当前非暴击、目标修正前”计算，复用实际BondRuntime.damage_multiplier、market_outgoing_damage_multiplier、EncounterState.outgoing_damage_multiplier；对当前raw Some的技能使用其实际DamageProfile。utility/治疗/金币/CD/范围/数量保持各自语义，不乘伤害倍率。不要复制CardModifiers公式。

显示两条短行：基础伤害（当前属性）与当前加成后伤害（非暴击、目标修正前）；附一句未计目标/距离/随机/暴击/防御。若只做最窄Crown26修复，也必须显示这层边界，不保留“全部当前加成已包含”的承诺。

next-rank比较时，最高技能归属可能切换，需用当前值的实际build、next值的候选rank只读build快照分别算top_skill_multiplier；不要永久修改PlayerBuild、发装备效果或消耗RNG。显示“按当前属性”可明确不承诺选牌额外永久训练奖励带来的未来属性。

## 有界验证提案

1. 真实召唤链：只学26rank5→实际购买Crown得6→实际新召唤骨龙→真实吐息投射物/profile→无护甲/护盾普通靶→resolver日志。挑actualcrit_multiplier=1的命中核对raw≈650.8、market×1.3后≈846.0；不注入HP/HitEvent。读取真实技能详情文字，比较完整float算得的1位显示。
2. 相邻负例：Crown最高技能在另一index时26不乘1.3；普通/召唤/反应profile各保留真实分支；加入一个真正召唤伤害羁绊，预览与真实noncrit命中均随之改变。读取预览前后RNG state必须不变。
3. 上下文边界：有Chaos时卡片用静态bounds；普通靶/首领靶伤害不同但目标中性数不谎称二者HP相同。现有有限骨龙快照另作说明，不在UI修复里偷偷改其damage更新机制。
4. 原生只需EN/ZH各一组同Crown26流程，baseline/after四case，固定当前C相机与真实详情Text；真实吐息HP/profile放独立短链而非强行塞进暂停选牌截图。保留标题修复已过的6→6/6检查，但不能只以标题作为增伤正确证据。

此提案尚未实施，不构成所有32技能数值/元数据/描述均已正确的结论。
