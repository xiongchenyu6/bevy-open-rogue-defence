# 当前碰撞与空背包改动的独立 Linux 包

游戏源码 `f5ef6ff8f37ecab467b5ba0baadb6decd8d49aaf`。Steam Runtime 3 sniper SDK / Rust 1.98.0 实际构建退出 0，用时 124.350 秒；独立 Ubuntu24 / X11 / 软件 Vulkan 断网运行退出 0，用时 150.287 秒。包目录只读，从 `/tmp` 启动，没有挂载游戏源码、Nix 或 Rust/Cargo。

包目录 `target/desktop-packages-overlap-nov6/rogue-commander-linux/`；压缩包 `target/desktop-packages-overlap-nov6/rogue-commander-linux-0.1.0.tar.gz`，60,852,005 字节。二进制 SHA256 `76a6da42c0c9f49289fe77b0caea02ec7a425b793930220a43de2219b0ae72cc`；压缩包 SHA256 `fcab603283b214bb069c020dd7f0b2e3fcdeb06cf05f12bf675f0281a64e93b8`。全部 886 项文件在宿主与独立运行中核验；新空军械架运行文件与源码素材的 SHA 一致。完整索引和依赖许可随包保留，没有缺失许可文件；这不是法律审核。

本包在之前 `86cb084` 的基础上纳入指定目标技能锁定、同前沿普攻就近选择、同碰撞进度的几何选择，以及原创空军械架。对应源码回归分别见[指定目标](../designated-strike-2026-11-06/README.md)、[普攻瞄准](../native-aim-2026-11-06/README.md)、[碰撞几何](../overlap-geometry-2026-11-06/README.md)、[空背包](../empty-armory-2026-11-06/README.md)。这次独立短流程不代替这些专项的具体测试，也不代表整局战役通过。

## 实际界面运行

- 首次章节、资源就绪后的真实指南、跳过指南、基础英雄征募、战斗倒计时推进。
- 中文设置保存、F11 全屏 1600×900、回到 1280×720。
- 正常退出；第二个真实游戏进程读取同一设置，保持中文和指南偏好，再次征募并进入战斗。
- 再次正常退出，两次退出均为 0。

[原始结果](smoke/result.json)、[完整执行日志](smoke.log)、[启动命令和阶段耗时](smoke-process.json)。Root 实际查看[英文首次战斗](smoke/battle.png)与[中文重启战斗](smoke/restart-battle-chinese.png)；另有[中文设置](smoke/settings-chinese.png)和[全屏](smoke/fullscreen-chinese.png)。两个游戏日志未出现 panic、缺素材路径或渲染初始化错误。无可听物理设备的容器只验证启动与界面，不验证真实设备听感。

`python3 docs/verification/overlap-package-2026-11-06/verify.py` 检查归档哈希、明确通过标志、进程结果、版本与文件索引、链接与许可范围。它不启动游戏。新包自然合成、同档重启、完整第六章、更多种子与构筑、实体音频/手柄、硬件性能、其他平台和 Steam 客户端仍需独立验证；旧包的通过不能替代本包。
