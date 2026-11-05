# 86 候选包：自然获取与装备合成

不可变候选包 `86cb08498e446a27b31554f8cb6dd3eb641e214b` 的真实公开界面合成路线已完成，退出码 0，耗时 943.077 秒。条件同档重启随后在启动游戏前被助手的 audio 文件存在断言拦住；**本页不宣称重启通过**。

## 包与输入边界

[固定包清单](reviewed-package-pins.json)：882 个文件逐项校验，游戏二进制 SHA-256 为 `b5bc6c351bf5a781ae43dd6bb8c21ffc015549b70ddd46aa297692e1feea9cf0`。本次证据仅对应这个 86 包，不代表其他源码版本或完整数值回放一致。

断网 Docker 运行发行包，包与四份冻结助手只读挂载，不挂载 Rust、Nix 或游戏源码。[启动命令](command.json)、[助手版本/哈希](harness-snapshot.json)、[冻结助手](harness-source/forge_runtime.py) 保留。初始隔离状态仅复制真实积累的 campaign.save，无教程、音量或配置文件预置，无库存、战功、材料或属性注入。

实际教程画面通过可见公开输入跳过，然后选基础英雄开始战斗。战功来自本轮击杀，材料来自同一轮真实随机黑市，没有刷新库存或制造材料。

## 实际结果

[原始结果](result.json)、[进程](process.json)、[运行日志](runtime.log)：`completed=true`、`quit_exit=0`。

| 步骤 | 严格核对结果 |
| --- | --- |
| Arc Lens 购买 | 180 战功，钱包 320 → 140 |
| 第一次背包检查 | 数量 1，全部名称 `[arclens]` |
| Echo Core 购买 | 300 战功，钱包 363 → 63 |
| 第二次背包检查 | 数量 2，全部名称 `[arclens, echocore]` |
| 实际合成 | 选中 Astral Prism，材料明确为 Arc Lens + Echo Core，按钮为 Forge equipment |
| 合成后全部背包检查 | 两材料消耗，仅一件 `[astralprism]` |
| 关闭后战斗 | 实际倒计时 995 → 985 |
| 退出与长期存档 | 正常退出 0，初始与结束 campaign.save 字节相同 |

[合成前](ready-materials.png) 与 [合成后](crafted-inventory.png) 展示真实定制装备图、材料状态及全部继承效果。[双材料第一件](material-2-icon-0.png) 与 [第二件](material-2-icon-1.png) 是各自实际详情，不用图标颜色代替身份。

四次图标选择只发送一次 mouseDown，观察卡片选中留白后 mouseUp；按下至释放分别 0.286、0.240、0.876、0.212 秒。颜色只确认可见选择，释放后的新截图仍严格解析完整名称，全部持有集合检查不变。两次 Close 的持续按下分别 6.210、4.260 秒，均观察装备标题和按钮消失、释放后再做完整画面关闭检查。原始截图和单调时间都在结果记录中。

[初始存档](initial-campaign.save) 与 [结束存档](final-campaign.save) SHA-256 都为 `da5bf0a9ca3f4032a265425f9989bda292d8c0c2622848157df6d2c0223ba825`；长期进度保持 unlocked=5、575 currency、cleared=111100、upgrades=[5,0,0,0,0,0]。

## 条件重启未通过

管理任务在合成正常完成后自动启动同档重启：精确同一个 forge/state 读写挂载，没有复制或编辑状态；前次结果与包版本、正常退出、一件 Astral Prism、存档哈希等前置检查通过。随后助手要求 `STATE/audio` 是文件，而真实路线没有调整音量，因此没有写出默认音量文件。`STATE/config` 及实际通过教程写出的 tutorial-dismissed 文件存在。[真实状态文件列表](actual-profile-files.json) 保留。

[重启失败日志](restart-precondition-failure/runtime.log) 与 [进程](restart-precondition-failure/process.json)：启动游戏前退出 1，0.967 秒。没有伪造 audio 文件来绕过断言。这是独立的助手前置条件失败，不是游戏重启行为的验证或失败结论；后续重启需单独报告。
