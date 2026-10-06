# 召唤待命阵型与视觉分配隔离

此前移动召唤物与英雄伙伴使用 ECS Entity 编号计算待命阵型。模型与视觉子实体共用同一实体分配器，因此它们的加载时机可能改变实际待命位置、追敌距离及攻击时机。

生产召唤池现在为每次成功租借分配递增 FormationSlot；失败的租借不消耗编号，退还时移除旧组件，复用时获得新编号，远征重置通过 CombatPools::default 从零开始。三列九条待命线的范围、移动速度和朝向规则不变，仍位于城墙外侧。骷髅、猎犬、水元素、冰龙、祖灵及英雄伙伴都使用该编号；静态剑塔仍保留城墙联动。

回归测试通过真实召唤池生成三只骷髅，并执行真实 update_skill_summons：两局分别不插入视觉实体和每次召唤前插入 79 个空视觉实体，要求实际召唤实体编号不同，但逻辑阵型、站位与朝向一致。完整库测试 566 项通过、0 失败、1 项既有忽略，包含该视觉实体干扰测试。格式检查、全目标严格 Clippy 通过。

该测试验证一个具体依赖，不证明完整游戏重放确定。战斗目标并列处理及其他共享随机数消费顺序仍需要独立重放审计。旧构建的两次完整通关在第 32400 帧已有记录分歧，见 [重放证据](balance-replay.md)。

[完整库测试](verification/summon-formation-2026-10-06/summon-formation-tests-final.log) · [严格 Clippy](verification/summon-formation-2026-10-06/summon-formation-clippy.log)。修正后的模拟器、游戏和截图程序已构建。独立整局重放仍有观测分歧，尚未完成确定性验证。原生画面验收见下节。

## 原生桌面验证

1280×720、中文、软件 Vulkan、真实 GamePlugin 的 `summon-assault` 场景完成 680 帧并退出 0。真实技能施放生成六类移动召唤物，实际测试靶生命下降；五类模型的行走与攻击动画均被观测，替换存活测试靶后重新追击。暂停期间七个活动召唤物的位置与朝向冻结 29 帧。

第 510 和 650 帧验证无存活测试靶时待在城墙外、面向入口，剑塔位于墙上；城墙倒塌后重建恢复有限生命，阵型仍在外侧。测试靶被场景脚本设置为死亡，尸体模型可能保留，不代表生产死亡清理行为。截图已人工查看；队伍仍有拥挤，需要继续做阵型视觉打磨。此结果不证明硬件帧率、手机真机或全章节平衡。

[检查输出](verification/summon-formation-native-2026-10-06/capture.log) · [验证范围与校验值](verification/summon-formation-native-2026-10-06/verification.json) · [追击](verification/summon-formation-native-2026-10-06/frame00179.png) · [城墙外待命](verification/summon-formation-native-2026-10-06/frame00509.png) · [重建后](verification/summon-formation-native-2026-10-06/frame00649.png)。
