# 86 包：真实合成后同状态重启

本轮实际重启验收通过：进程退出 0，耗时 **155.390 秒**，低于助手内部 180 秒和外部 300 秒预算。[原始结果](result.json)、[进程](process.json)、[日志](runtime.log) 保留。

这次只执行重启，没有重跑自然合成或 C6。前次 [86 自然合成成功](../earned-forge-86cb084-2026-11-06/README.md) 的原始证据及首次 audio 文件前置条件失败保留，不覆盖、不改写。

## 精确同档与固定包

[Docker 命令](command.json) 记录精确原目录 `target/verification/vortex-earned-forge-2026-11-06/forge/state` 读写挂载为 `/earned-state`，没有复制、编辑或补造状态。前次成功 Forge 目录只读挂载为 `/previous-proof`；包和五份冻结助手也只读挂载，网络关闭，无 Rust、Nix 或游戏源码。

使用相同不可变 `86cb08498e446a27b31554f8cb6dd3eb641e214b` 包，882 个文件逐项校验，二进制 SHA-256 `b5bc6c351bf5a781ae43dd6bb8c21ffc015549b70ddd46aa297692e1feea9cf0`。[包校验](package-check.json)、[助手提交与哈希](harness-snapshot.json) 保留；五份助手冻结于 `5e42f0e769015dbc0954b7bc3bede99982398d6a`。

重启前严格检查前次结果 completed=true、正常退出 0、相同完整包版本、一件 Astral Prism，以及初末真实 campaign.save 哈希。删除的仅是“默认音量也必须已经写出 audio 文件”的错误前置条件；实际 config、前次成功和存档约束不变，没有伪造音量文件。

## 实际公开界面结果

| 检查 | 实际结果 |
| --- | --- |
| 原长期进度 | unlocked=5、575 currency、cleared=111100、upgrades=[5,0,0,0,0,0] |
| 基础英雄属性 | Storm Savant，等级 1，1×速度，Strength 30.0 |
| 本轮装备 | EQUIPMENT 0 与 No equipment yet，前次合成装备未跨局保留 |
| 关闭装备后 | 真实倒计时 1464 → 1454 |
| 公开退出 | quit_exit=0 |
| campaign.save | 初末精确相同，SHA-256 `da5bf0a9ca3f4032a265425f9989bda292d8c0c2622848157df6d2c0223ba825` |

[实际 Strength](restart-strength.png)、[空背包](restart-empty-inventory.png)、[恢复前](restart-resume-before.png) 与 [恢复后](restart-resume-after.png) 都来自该原状态目录启动的实际游戏画面。

共享 Close 只按下一次；标题观察 1.800 秒、持续按下 4.760 秒、总关闭操作 6.560 秒。记录包含整体截止时间、独立十秒持续按下截止时间、派发结束、每次同图标题观察与释放时刻。释放后仍用新完整截图严格检查装备标题和 Close 消失。没有重复点击或切换输入。

[启动前状态哈希](profile-before.json) 与 [结束后状态哈希](profile-after.json) 的文件集合及字节完全相同：只有 campaign.save 和游戏实际写出的 config/rogue-commander/tutorial-dismissed。没有 audio 文件，默认偏好缺省状态保持原样。

## 复核边界

`python3 docs/verification/forge-restart-86cb084-2026-11-06/verify.py` 只验证归档结果、挂载路径、助手哈希与期限记录，不启动游戏。

这是这个固定 86 包的“真实自然合成后，同状态重启，长期属性保留、本局装备清空、战斗恢复、正常退出”证据。它不等于 C6 完整通关、实体手柄验收或全部数值回放已一致。
