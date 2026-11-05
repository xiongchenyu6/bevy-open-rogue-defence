# Linux 桌面发行候选包

当前独立运行验收通过的候选包来自游戏源码 `86cb08498e446a27b31554f8cb6dd3eb641e214b`：

- 目录：`target/desktop-packages-vortex-nov6/rogue-commander-linux/`
- 压缩包：`target/desktop-packages-vortex-nov6/rogue-commander-linux-0.1.0.tar.gz`，60,763,871 字节。
- 二进制 SHA256：`b5bc6c351bf5a781ae43dd6bb8c21ffc015549b70ddd46aa297692e1feea9cf0`
- 压缩包 SHA256：`d4323b0bc59ac5434e772d97eaceb699f9919263f0950837c2611f95cd4e7880`

直接运行包内 `rogue_commander`，保持 `assets` 和 `third-party` 同级。移动使用方向键，包内说明已同步。本包包括黑市装备插画、剑类接触校准、密集伤害受击恢复、镜头反馈修正，同帧雷／冰被动与普攻回响次数修正，敌人受击短闪与材质隔离，以及初始剑的平滑渐隐残影、普通木箭和六件合成装备独立插画。本轮另包括区域技能稳定目标排程（[真实回归](verification/vortex-order-2026-11-06/README.md)）、装备收藏列表、选中大图与独立滚动详情、固定合成操作，以及紧凑手柄选定配方后跨焦点移动保持选择。源码原生验收见[装备界面证据](verification/inventory-split-2026-11-05/README.md)。它是开发候选包，未发布到 Steam，完整发售门槛见[验收清单](release-readiness.md)。

## 构建和内容核验

使用仓库固定摘要的 Steam Runtime 3 sniper SDK 和 Rust 1.98.0，使用热缓存的实际 release 构建用时 1 分 42 秒。最终 ELF 使用 `/lib64/ld-linux-x86-64.so.2`，没有 Nix 依赖或库搜索路径。882 项包内文件在打包后、宿主复核及隔离容器中逐一验证哈希；新装备插画 atlas 与仓库来源哈希一致。383 项 Rust 依赖的许可证无缺失，完整素材来源、字体 OFL、本地 `bevy_audio` 补丁及其原始许可随包保留。许可收集不是法律审核。

```sh
tools/build_desktop_linux.sh target/my-linux-package
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner \
  -C target/my-linux-package -cf - rogue-commander-linux | gzip -n \
  > target/my-linux-package/rogue-commander-linux-0.1.0.tar.gz
```

工具拒绝覆盖已有输出，并保留 SDK 的独立编译与依赖缓存。5 项打包集成测试通过。[来源、镜像与产物哈希](verification/vortex-package-2026-11-06/artifact.json) · [构建日志](verification/vortex-package-2026-11-06/build.log)。

## 独立运行验收

Ubuntu 24.04、X11、Mesa 软件 Vulkan：从 `/tmp` 启动只读安装包，关闭网络，不挂载游戏源码、Nix 或 Rust/Cargo。实际 UI 自动化验证：

- 首次章节选择、资源就绪后的指南、跳过指南和基础英雄征募。
- 实际战斗倒计时推进。
- 切换中文并保存，F11 全屏 1600×900、恢复 1280×720。
- 退出后用第二个游戏进程重启，恢复中文与跳过指南偏好，再次征募和战斗。
- 两次正常退出均返回 0；两个游戏日志没有缺路径、ICU、panic 或渲染错误。

Root 实际查看了本轮首次战斗和重启中文战斗两张原生截图；其他路线状态另有截图和自动断言。容器没有可听音频设备，界面正确显示设备暂不可用与自动重连，不能证明真实设备听感。[运行断言](verification/vortex-package-2026-11-06/smoke/result.json) · [战斗](verification/vortex-package-2026-11-06/smoke/battle.png) · [中文设置](verification/vortex-package-2026-11-06/smoke/settings-chinese.png) · [重启战斗](verification/vortex-package-2026-11-06/smoke/restart-battle-chinese.png)。

```sh
mkdir -p target/package-smoke-new
docker run --rm --network none \
  --mount "type=bind,src=$PWD/target/my-linux-package/rogue-commander-linux,dst=/game,readonly" \
  --mount "type=bind,src=$PWD/target/package-smoke-new,dst=/verify" \
  --mount "type=bind,src=$PWD/tools/desktop/smoke_runtime.py,dst=/smoke.py,readonly" \
  --workdir /tmp rogue-commander-linux-smoke:ubuntu24-oct8 python3 /smoke.py
```

## 仍未覆盖

上述短流程不代表完整25分钟战役、后续征募／支线／死亡／奖励／永久强化的整局回归。86cb084包已完成自然材料购买、实际合成、背包逐项核对、恢复战斗、正常退出及同档重启，见下节；更多配方经济和完整第六章胜利仍未验证。旧版中断证据保留，但不替代当前包结论。

真实Steam客户端、SteamDeck、物理手柄与音频、Windows／macOS、更多驱动／分辨率和长局硬件性能仍需验收。历史证据继续保留，但不作为当前包的通过依据。

## 当前包自然合成与同档重启

86cb084已通过[真实获取两材料并合成星界棱镜](verification/earned-forge-86cb084-2026-11-06/README.md)：Arc Lens 320→140（180），Echo Core 363→63（300），背包1→2→1，最终仅有Astral Prism，战斗倒计时995→985，正常退出且长期进度字节不变。

[同一实际用户目录重启](verification/forge-restart-86cb084-2026-11-06/README.md)也已通过：没有复制或改造状态，Strength30保留、局内装备清空、倒计时1464→1454、再次正常退出，前后状态哈希一致。音量文件缺省是正常默认设置，没有为测试补造文件。

这些记录不代表整章通关、所有配方或物理设备验收。040ce5c的历史记录继续保留，当前结论来自本节86cb084的实际执行。
