# Linux 桌面发行候选包

当前独立运行验收通过的候选包来自游戏源码 `d4540c44df479041fe4ff4fdb818f8fb2f280f99`：

- 目录：`target/desktop-packages-wallet-blades-nov6/rogue-commander-linux/`
- 压缩包：`target/desktop-packages-wallet-blades-nov6/rogue-commander-linux-0.1.0.tar.gz`，60,894,258 字节。
- 二进制 SHA256：`7dce2d9be5a8a8d8fa9027b1adc5fc7b0f3705b6ffadf6cb8652326e0b7aec24`
- 压缩包 SHA256：`5b380058e2278fee3fcbe5f3ab9226f74c3d6e293a09999bc30ea5251bbf7808`

直接运行包内 `rogue_commander`，保持 `assets` 和 `third-party` 同级。移动使用方向键，包内说明已同步。本包包括黑市装备插画、剑类接触校准、密集伤害受击恢复、镜头反馈修正，同帧雷／冰被动与普攻回响次数修正，敌人受击短闪与材质隔离，以及初始剑的平滑渐隐残影、普通木箭和六件合成装备独立插画。本轮另包括区域技能稳定目标排程（[真实回归](verification/vortex-order-2026-11-06/README.md)）、装备收藏列表、选中大图与独立滚动详情、固定合成操作，以及紧凑手柄选定配方后跨焦点移动保持选择。源码原生验收见[装备界面证据](verification/inventory-split-2026-11-05/README.md)。本包还包含指定目标技能约束、同前沿普攻就近选择、同碰撞进度的几何选择，以及原创空军械架，见[碰撞回归](verification/overlap-geometry-2026-11-06/README.md)和[空背包原生验收](verification/empty-armory-2026-11-06/README.md)。本候选另包含[射线目标排程](verification/ray-order-2026-11-06/README.md)、[同轮城门状态](verification/gate-pass-state-2026-11-06/README.md)、[购买首帧钱包同步](verification/market-hud-sync-2026-11-06/README.md)和[双刀接触时点](verification/dagger-rhythm-2026-11-06/README.md)修正。最后完整库验证为689项通过、1项忽略，严格Clippy通过。它是开发候选包，未发布到 Steam，完整发售门槛见[验收清单](release-readiness.md)。

## 构建和内容核验

使用仓库固定摘要的 Steam Runtime 3 sniper SDK 和 Rust 1.98.0，使用热缓存的实际 release 构建用时 2 分 24 秒。最终 ELF 使用 `/lib64/ld-linux-x86-64.so.2`，没有 Nix 依赖或库搜索路径。886 项包内文件在打包后、宿主复核及隔离容器中逐一验证哈希；新装备插画 atlas 与仓库来源哈希一致。383 项 Rust 依赖的许可证无缺失，完整素材来源、字体 OFL、本地 `bevy_audio` 补丁及其原始许可随包保留。许可收集不是法律审核。

```sh
tools/build_desktop_linux.sh target/my-linux-package
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner \
  -C target/my-linux-package -cf - rogue-commander-linux | gzip -n \
  > target/my-linux-package/rogue-commander-linux-0.1.0.tar.gz
```

工具拒绝覆盖已有输出，并保留 SDK 的独立编译与依赖缓存。打包工具既有的5项集成测试通过；本次独立构建和短流程的具体结果见下列记录。[来源、镜像与产物哈希](verification/wallet-blades-package-2026-11-06/artifact.json) · [构建日志](verification/wallet-blades-package-2026-11-06/build.log)。

## 独立运行验收

Ubuntu 24.04、X11、Mesa 软件 Vulkan：从 `/tmp` 启动只读安装包，关闭网络，不挂载游戏源码、Nix 或 Rust/Cargo。实际 UI 自动化验证：

- 首次章节选择、资源就绪后的指南、跳过指南和基础英雄征募。
- 实际战斗倒计时推进。
- 切换中文并保存，F11 全屏 1600×900、恢复 1280×720。
- 退出后用第二个游戏进程重启，恢复中文与跳过指南偏好，再次征募和战斗。
- 两次正常退出均返回 0；两个游戏日志没有缺路径、ICU、panic 或渲染错误。

Root 实际查看了本轮首次战斗和重启中文战斗两张原生截图；其他路线状态另有截图和自动断言。容器没有可听音频设备，界面正确显示设备暂不可用与自动重连，不能证明真实设备听感。[运行断言](verification/wallet-blades-package-2026-11-06/smoke/result.json) · [战斗](verification/wallet-blades-package-2026-11-06/smoke/battle.png) · [中文设置](verification/wallet-blades-package-2026-11-06/smoke/settings-chinese.png) · [重启战斗](verification/wallet-blades-package-2026-11-06/smoke/restart-battle-chinese.png)。

```sh
mkdir -p target/package-smoke-new
docker run --rm --network none \
  --mount "type=bind,src=$PWD/target/my-linux-package/rogue-commander-linux,dst=/game,readonly" \
  --mount "type=bind,src=$PWD/target/package-smoke-new,dst=/verify" \
  --mount "type=bind,src=$PWD/tools/desktop/smoke_runtime.py,dst=/smoke.py,readonly" \
  --workdir /tmp rogue-commander-linux-smoke:ubuntu24-oct8 python3 /smoke.py
```

## 仍未覆盖

上述短流程不代表完整25分钟战役、后续征募／支线／死亡／奖励／永久强化的整局回归。当前d454包已通过[自然购买材料、合成与同档重启](verification/wallet-blades-forge-v3-2026-11-06/README.md)：奥术透镜332→152（180）、回响核心310→10（300），两材料消耗后背包仅有星界棱镜，计时1003→992，正常退出且长期存档字节不变。同一实际用户目录重启后局内装备清空、永久力量30保留，计时1449→1439、正常退出。Root实际查看合成装备、重启空背包及力量画面。本次没有执行付费刷新。独立第六章完整路线已启动，尚未形成终局，不能据合成流程称整章通关。

较早的86cb084自然合成与同档重启见下节，不能替代当前包。f5候选的[第六章购买观察失败](verification/overlap-chapter6-2026-11-06/README.md)、[R2关闭前预算耗尽](verification/overlap-chapter6-r2-2026-11-06/README.md)、[R3实际钱包显示冲突](verification/overlap-chapter6-r3-2026-11-06/README.md)和[自然合成真实败北](verification/earned-forge-loss-2026-11-06/README.md)均保留为历史证据。钱包显示的生产修正已包含在d454包，但历史失败不能改算通过。更多配方与完整路线仍需验证。

真实Steam客户端、SteamDeck、物理手柄与音频、Windows／macOS、更多驱动／分辨率和长局硬件性能仍需验收。历史证据继续保留，但不作为当前包的通过依据。

## 历史候选包的自然合成与同档重启

86cb084已通过[真实获取两材料并合成星界棱镜](verification/earned-forge-86cb084-2026-11-06/README.md)：Arc Lens 320→140（180），Echo Core 363→63（300），背包1→2→1，最终仅有Astral Prism，战斗倒计时995→985，正常退出且长期进度字节不变。

[同一实际用户目录重启](verification/forge-restart-86cb084-2026-11-06/README.md)也已通过：没有复制或改造状态，Strength30保留、局内装备清空、倒计时1464→1454、再次正常退出，前后状态哈希一致。音量文件缺省是正常默认设置，没有为测试补造文件。

这些记录不代表整章通关、所有配方或物理设备验收。040ce5c的历史记录继续保留，本节只陈述86cb084的实际执行，不作为当前d454包的通过依据。
