# Linux 桌面发行候选包

当前候选包来自源码 `52dd7c82ffda81f96f65b4deade16a07c0606506`。

- 目录为 `target/desktop-packages-current-nov7/rogue-commander-linux/`
- 压缩包为 `target/desktop-packages-current-nov7/rogue-commander-linux-0.1.0.tar.gz`，62,322,004 字节。
- 二进制 SHA256 为 `9f4f82379f5b630c38cb842275a514dd1d87a8de24706ca0dc1ac1e5c90d325f`
- 压缩包 SHA256 为 `323d2a5ec8e5a8babb0ce9f9245bfa16bb7ab99fcff7b329ebcdded18c096899`

运行包内 `rogue_commander`，保持 `assets` 和 `third-party` 同级。方向键移动。本包为开发候选。

## 本轮包含的更改

弓手与法师的实际弹道释放随攻击动作延迟，普通命中与暴击闪色分层。技能选择分别展示基础公式和当前装备、羁绊、奇遇提供的已知非暴击增伤，修正王冠升级与替换技能的归属预览。窄屏选牌标题与操作栏分行，并验证技能、羁绊和付费连续选择。

对应 [释放节奏](verification/ranged-release-2026-11-06/README.md)、[伤害预览](verification/skill-damage-modifiers-2026-11-07/README.md)及[窄屏连续选择](verification/draft-header-2026-11-07/README.md)保留各自真实回归。当前源码对应 764 项库测试通过、0 失败、1 既有忽略，严格 Clippy 通过。SDK 构建另有实际收据。

## 构建与文件核验

固定 Steam Runtime3 sniper SDK、Rust 1.98.0，实际构建 118.717 秒。544 项构建和运行输入前后与固定提交一致。独立核对 892 项清单文件、893 个压缩包常规文件及全部条目的归零所有者和时间。ELF 检查通过。

实际镜像、输入清单、完整日志、截图及可移植核验器见[当前包证据](verification/current-package-2026-11-07/README.md)。此前 [bdd 候选包](verification/glacier-package-2026-11-06/README.md)按原版本保留。

## 实际独立运行

Ubuntu 24、X11、软件 Vulkan，断网、只读安装包，工作目录 `/tmp`。实际 153.970 秒完成首次指南、征募、战斗推进、360×800 窗口公开 G 选卡、实际鼠标选择、恢复 1280×720 战斗、中文设置、全屏恢复、偏好重启、再次征募与战斗及两次正常退出。13 项结果检查通过，实际首次技能手牌为一手。

双尺度倒计时核对分别为 1498→1493、选牌后 1475→1469、重启后 1486→1481。全部原始观察与拒绝记录保留，Root 已查看 20 张原图。恢复窗口时一帧出现旧窄视口与白色延展，后续画面恢复。一次重启 OCR 缺少冒号而被严格拒绝。

## 验收边界

手机卡名仍有逐字符换行，长技能详情需要滚动。测试环境无可用音频设备，实体音频、硬件性能及 Steam 客户端待验收。

当前版本尚未重跑完整第六章、自然合成或旧赚取存档兼容。历史章节、平衡矩阵与整局复跑属于各自版本，详见[发售清单](release-readiness.md)。当前独立包短流程通过，整体美术、完整长局与更多英雄构筑覆盖仍在进行。
