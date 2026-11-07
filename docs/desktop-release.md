# Linux 桌面发行候选包

当前候选包来自源码 `6f24fe71f0b99aad630d9a89306e914aecc5ed25`。

- 解压目录为 `target/desktop-packages-decision-nov8/rogue-commander-linux/`
- 压缩包为 `target/desktop-packages-decision-nov8/rogue-commander-linux-0.1.0.tar.gz`，62,293,951 字节
- 二进制 SHA256 为 `bc421dce56f28b4d551f43302d562ef938f830da3a8bc721103ff86fda86bfe5`
- 压缩包 SHA256 为 `260041c461904b7dfb8ff582efa0bff81af1b1ef1cf1fd3eecab12379896b34e`

解压后运行包内 `rogue_commander`，保持 `assets` 和 `third-party` 同级。方向键移动，G 打开技能选择，设置中切换语言。本包为开发候选。

## 包含的更改

满技能槽后，已解锁的新 SR、SSR、UR 技能进入抽取池。替换卡显示旧技能等级和新技能 1 级，保留已获得的永久训练。普通手牌最多一个 UR，专属进化方向单独展示。手机卡名保持完整单词换行，三列卡片使用 13–15px 字号，并支持六选项滚动和连续选择。弩箭与继承弩箭使用统一的箭矢增伤分类。

连续选牌在十级英雄选择打断后恢复时，使用吞噬后的新技能容量。市场、英雄成长、选牌与战斗阶段顺序有明确约束，回归见[决策阶段与容量刷新](verification/decision-phase-2026-11-08/README.md)。

对应[满槽替换](verification/full-slot-variety-2026-11-07/README.md)、[手机选牌](verification/phone-draft-readability-2026-11-07/README.md)和[箭矢分类](verification/arrow-classification-2026-11-07/README.md)保存各自回归。打包源码库测试 786 项通过、0 失败、1 项既有忽略，严格发布与开发捕获 Clippy 通过。

## 构建与制品核验

使用固定 Steam Runtime3 sniper SDK 和 Rust 1.98.0，实际构建 115.079 秒。独立核对 551 项构建和运行输入与固定提交一致、892 项包清单、893 个压缩包常规文件，以及全部条目的归零所有者和时间。ELF 检查通过。

实际镜像、输入清单、日志、截图及可移植核验器见[新版包证据](verification/decision-package-2026-11-08/README.md)。此前[满槽候选包](verification/full-slot-package-2026-11-07/README.md)和 [52 候选包](verification/current-package-2026-11-07/README.md)按原版本保留。

## 实际独立运行

在 Ubuntu 24、X11、软件 Vulkan 环境中断网运行，安装目录只读，工作目录为 `/tmp`。149.608 秒完成首次指南、征募、战斗推进、360×800 窗口公开 G 选卡、实际鼠标选择、恢复 1280×720 战斗、中文设置、全屏恢复、偏好重启、再次征募与战斗及两次正常退出。13 项结果检查通过。

三组双尺度倒计时核对分别为 1498→1492、选牌后 1476→1470、重启后 1486→1480。Root 通过独立解码的五张带文件名索引图审阅全部 25 张原图，并额外查看手机选牌和最终退出确认的全尺寸解码图。恢复窗口时一帧仍显示旧窄视口和白色延展，后续画面恢复。最终 `current-ui.png` 完整显示退出确认。一帧重启 OCR 缺少冒号，已严格拒绝并保留。

## 当前限制

长技能详情仍需要滚动。相同种子的额外完整模拟在约 625 秒出现选牌分歧，原始失败比较已保留。全局确定性与整体数值平衡继续验证。

本包验证范围为公开入口短流程。完整第六章、自然合成、旧赚取存档、实体音频、硬件性能及 Steam 客户端仍待覆盖，详见[发售清单](release-readiness.md)。
