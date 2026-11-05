# 发售体验验收

目标仍是持续提高游戏体验，达到可以上架 Steam 的单机游戏品质。这份清单约束实际游玩与交付质量，不代表已满足平台审核，也不因某一轮测试通过而缩小目标。

## 当前证据与下一步

| 体验 | 当前可确认事实 | 达到发售品质还需验证／实现 |
| --- | --- | --- |
| 战斗节奏 | 第一章40局前后对照、544项库测试和1,600帧原生场景已归档；低级伤害技能通过普攻蓄力与冷却施放 | [后两章24组探索性样本](late-chapter-balance.md)为旧源码历史记录；[区域目标排程回归通过，修复后整局重复仍失败](verification/vortex-fix-replay-2026-11-06/README.md)，[精确逐帧诊断](verification/balance-trace-2026-11-06/README.md)已定位首个记录掉血差异，完整原因未证实，当前24组矩阵未启动；六章更多种子、全部12英雄与吞噬路线；新增永久属性后的难度边界；主动操作与支线奖励的取舍 |
| 音效 | 普攻、命中、终结技和首领提示均按真实时间限流；六档速度实际声音实例测试通过，见[音效规则](audio-feedback.md) | [真实隔离ALSA后端断线／恢复](verification/audio-backend-2026-11-05/README.md)已通过暂停／8×、2400帧、新流／音量／非零录音验证；物理设备重连、长局听感、混音峰值与音效辨识仍需验证 |
| 设置和暂停 | 设置支持语言、简洁／详细选牌、独立音乐／效果音、静音与偏好持久化；暂停支持恢复与失焦暂停，见[音量设置](audio-settings.md)；原生窗口／全屏、F11、偏好保存、取消离开、返回章节和退出已在Linux/X11验收，见[窗口和导航](settings-navigation.md) | Windows／macOS、高DPI、多显示器；完整流程长期回归 |
| 资源加载 | 正式入口等待当前征募与可见场景依赖；真实进度、失败重试、慢速重试与输入保护通过585项库测试及450帧原生验证，见[远征加载](expedition-loading.md) | 缺字体回退已通过真实文件缺失／恢复及450帧原生验收；10级后续征募缺模型／重试已通过960帧真实I/O验收；[果实与20／40级缺模型重试](verification/later-recruitment-2026-11-05/README.md)已通过1260帧受控原生验证；自然获取、后续缺字体与安装包文件修复仍需验证；GPU上传与真实硬件的启动时延 |
| 新玩家入门 | 四步短图文引导支持跳过与设置重播；暂停／输入、偏好持久化和写入失败恢复通过592项库测试及1600帧中英文桌面／紧凑原生验证，见[首次远征指南](first-expedition-guide.md) | 新玩家实际理解成本；浏览器存储失败；征募标题与设置入口重叠已在[布局跟进](commander-layout.md)验证修正；帽檐／武器裁切已通过后续59张截图及真实图像边界检查修正 |
| 视觉和输入 | 有原生桌面与窄屏受控战斗截图；52个自包含GLB及图标／立绘载荷校验通过；中文分词及静止界面的重复排版已修正，见[文字排版](text-layout-performance.md)；[黑市24插画与中英文桌面／窄屏、80步手柄回归](verification/black-market-2026-11-04/scope.md)已通过；[剑类接触点](verification/attack-rhythm-2026-11-05/README.md)及[连续伤害骨骼受击恢复](verification/authored-hit-2026-11-05/README.md)有前后原生证据 | 完整流程各状态可读性、遮挡与焦点；不同分辨率／缩放；其他交互状态的布局开销；长局特效峰值与帧时间；[手柄75步三种尺寸模拟原生验收](controller-input.md)已通过；物理控制器、平台映射和Steam Input仍需验证 |
| 战役和存档 | `campaign_progress.rs` 实现章节解锁、战果货币及6种永久强化；原生文件与浏览器存储分开；写入失败后的独立保存重试、原存档保护及奖励不重复已通过真实文件／按钮回归与中英文原生截图，见[保存恢复](campaign-save-retry.md) | 已通过[实际赚取力量并应用到下一局](earned-strength-runtime.md)；[实际安装包第一章通关及奖励延续](packaged-first-clear.md)已核对；[第二章完整胜利脚本及奖励强化重启](packaged-chapter2.md)已通过；[第三章完整胜利及力量26重启验证](packaged-chapter3.md)已通过；[第四章完整胜利及力量28重启验证](packaged-chapter4.md)已通过；[第五章后续实际赚取路线通关及力量32重启验证](packaged-chapter5.md)已通过，保留此前失败；[第六章实际路线失败，但奖励、强化和力量34重启通过](packaged-chapter6.md)；完整通关仍未验证，更多种子与路线仍缺证据；浏览器存储故障恢复；发行升级后的存档验证 |
| 装备和合成 | 首次黑市库存经过真实重置回归与安装包验证；[实际赚取货币购买奥术透镜、扣款、装备查看、属性查看、继续战斗和正常退出](earned-equipment.md)已通过；[合成奖励与材料继承展示](forge-readability.md)有中英文桌面／紧凑截图；[旧版独立包同局正常购买材料并合成星界棱镜](earned-forge-probe.md)已通过，材料消耗、属性与恢复战斗均有证据 | [3922b11新版正常材料获取／合成复验](verification/earned-forge-2026-10-14-r7/summary.json)已通过；进阶配方经济与获取路径；[购买提示期间钱包可读性](market-wallet.md)已有四套最终截图；更多装备组合和长期回归 |
| 内容差异 | 独立英雄、技能、羁绊、装备及营地系统存在；[短射程宿主的继承远程攻击](fusion-reach.md)有真实命中与原生截图证据 | 每个技能、首领和巨型营地敌人的实际机制／外观辨识，避免换数值或放大模型冒充新内容；全流程无占位资产 |
| 桌面交付 | 已有`86cb084`源码的Steam Runtime 3 SDK Linux release候选包，882项文件哈希、首次指南／征募／战斗／中文全屏设置／偏好重启／再次战斗及两次退出通过独立只读包、断网Ubuntu/X11软件Vulkan验收，见[桌面发行](desktop-release.md)；wasm／Android仍有构建脚本 | 真实Steam客户端、长局硬件性能、Steam Deck、Windows／macOS；更多驱动／分辨率；性能／崩溃记录与最终安装验收 |

## 验收口径

- 当前源代码决定功能是否存在；历史文档或截图不能替代当前版本实际行为。
- 每轮改动需有对应的真实系统测试；视觉／交互问题必须重新捕捉并查看截图或录像。声音限流需统计实际 `AudioPlayer` 实体，解码测试不能代替真实设备听感。
- 平衡记录注明章节、难度、种子、成长策略、二进制与截尾状态，帧数到上限不能算通关。
- 发布候选版本需覆盖从首次启动到选择章节、征募、战斗、选卡、黑市、支线、死亡、结算、永久强化、重试和退出的完整流程。每个目标平台独立验收。
- 没有当前证据的项保持未完成。其他平台显示设置、完整新手体验及其他平台与真实Steam安装验收是明确剩余工作，不能把第一章验证称为已可上架。

## 2026-11-06 当前源码与交付状态

已提交的`ecd1723`包含区域技能稳定目标排程、定点伤害的指定目标约束及投射物池复用清理，真实失败→通过见[区域排程](verification/vortex-order-2026-11-06/README.md)和[定点命中](verification/designated-strike-2026-11-06/README.md)。该版本优化库测试678通过、0失败、1忽略，严格全目标release Clippy通过。随后`39ac959`将同前沿普攻选择改为最近目标、再按稳定坐标打破平局，681项库测试和严格Clippy通过，见[选敌回归](verification/native-aim-2026-11-06/README.md)。完整重复两次均正常胜利，但记录数值仅一致到第38009帧，第38010帧仍出现差异；24组矩阵尚未启动，未据此调伤害或敌人成长数值。

最新独立短流程通过的SDK包为`86cb084`：[882项文件哈希、断网只读Ubuntu/X11运行、中文设置／全屏／偏好重启与两次正常退出](verification/vortex-package-2026-11-06/README.md)。该包含[装备收藏与大图详情、固定合成操作及手柄选择修正](verification/inventory-split-2026-11-05/README.md)。[自然购买两材料并合成星界棱镜](verification/earned-forge-86cb084-2026-11-06/README.md)及[同一实际用户目录重启](verification/forge-restart-86cb084-2026-11-06/README.md)均已通过：长期进度和Strength30保留，局内装备正确清空。完整第六章和更多配方仍待验收。

公开界面驱动已通过91项Docker测试，包含[新布局实际PNG几何](verification/inventory-public-geometry-2026-11-05/README.md)、[选中状态视觉确认](verification/inventory-selection-ack-2026-11-06/README.md)及[关闭阶段预算](verification/inventory-phase-budgets-2026-11-06/README.md)。所有已购装备仍逐项核对真实标题和完整集合，独立HUD与黑市余额严格一致。预算为140秒基础＋每件20秒，最多12次访问，累计暂停上限3240秒，1×整局内部上限5040秒；外部执行上限5400秒。此前[86第六章截止时间中断](verification/vortex-chapter6-2026-11-06/README.md)保留原始失败，R2实际路线在1492.662秒因背包数量标题OCR返回空集停止：6次购买已完整核对所有装备、7次恢复计时通过，第7件仅确认扣款，未完成所有权验证。实际画面仍是存活英雄的背包，不能算游戏失败或第六章通关；原始失败见[第六章R2](verification/vortex-chapter6-r2-2026-11-06/README.md)。

现有原生证据覆盖[敌人短闪与材质隔离](verification/enemy-flash-2026-11-05/README.md)、[剑残影](verification/sword-trail-2026-11-05/README.md)、[普通箭](verification/starter-arrow-2026-11-05/README.md)、[批量触发结算](verification/passive-batch-2026-11-05/README.md)、[六件合成装备插画与缺图重试](verification/recipe-art-2026-11-05/README.md)，以及[装备Item鼠标消息集成](verification/equipment-mouse-2026-11-06/README.md)。这些是对应场景的验证，完整长局与主观手感仍需验收。

[窗口性能基线](verification/window-performance-2026-11-05/README.md)来自旧构建的有限场景；当前包长局硬件性能、真实Steam客户端、实体手柄／音频设备、SteamDeck、其他平台、全部英雄与更广的种子／构筑覆盖尚未完成。历史版本证据保留在各自归档中，当前结论以本节标注的版本和范围为准。
