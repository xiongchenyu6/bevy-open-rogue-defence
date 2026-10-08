# Linux 桌面发行候选包

当前候选包为 `target/desktop-packages-oct9/rogue-commander-linux/`，压缩包为 `target/desktop-packages-oct9/rogue-commander-linux-0.1.0.tar.gz`（约55MiB）。游戏源版本为 `e109893`，包含新版引导、加载保护、保存重试、音频恢复与成长节奏改动。可直接运行包内 `rogue_commander`，保持 `assets` 和 `third-party` 同级。尚未发布到 Steam。

构建使用固定摘要的 Steam Runtime 3 sniper SDK、Rust 1.98.0。最终 ELF 使用普通 Linux 解释器，没有 Nix 库依赖或库搜索路径。SDK 与所选 Steam 运行时应匹配，见 [Valve Linux 开发说明](https://partner.steamgames.com/doc/store/application/platforms/linux)。

```sh
tools/build_desktop_linux.sh target/my-linux-package
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner \
  -C target/my-linux-package -cf - rogue-commander-linux | gzip -n \
  > target/my-linux-package/rogue-commander-linux-0.1.0.tar.gz
```

工具拒绝覆盖已有包，复制完整素材与来源文件，并收集目标平台正常／构建依赖许可证。本次875项文件逐一验证哈希，383项依赖没有缺失许可文本。`bevy_audio` 清单明确指向本地 `vendor/bevy_audio`，其原始 MIT 与 Apache 文本随包保留，确认包含音频恢复补丁。16项许可补充仍见 `licenses/rust-overrides.json`；清单区分原始声明和标准全文，未补造作者。项目音效、模型来源与字体 OFL 也随包保留。这不是法律审核。

## 独立运行验收

Ubuntu 24.04、Mesa 25.2.8 软件 Vulkan、X11：从 `/tmp` 启动只读包，不挂载源码、Nix 或 Rust/Cargo，网络关闭。真实界面识别等待指南出现，跳过后核对偏好文件为 `1`，再实际选英雄并确认战斗倒计时推进。中文设置、F11全屏1600×900及恢复1280×720、退出与第二进程重启均验证；重启恢复中文和跳过状态，再次征募与战斗正常，两次退出返回0。11张最终截图已实际检查，没有缺字、缺素材或 ICU 错误。

容器没有可听音频设备。设备不可用／自动重试提示完整可读，游戏继续运行；这不能证明真实听感或物理设备恢复。

```sh
docker build -f tools/desktop/Dockerfile.smoke-modern \
  -t rogue-commander-linux-smoke:ubuntu24-oct8 tools/desktop
mkdir -p target/package-smoke-new
docker run --rm --network none \
  --mount "type=bind,src=$PWD/target/my-linux-package/rogue-commander-linux,dst=/game,readonly" \
  --mount "type=bind,src=$PWD/target/package-smoke-new,dst=/verify" \
  --mount "type=bind,src=$PWD/tools/desktop/smoke_runtime.py,dst=/smoke.py,readonly" \
  --workdir /tmp rogue-commander-linux-smoke:ubuntu24-oct8 python3 /smoke.py
```

OCR 同时读取原图与反相灰度图、稀疏与块级排版，保留像素坐标；短小中文设置入口按工具栏位置打开后，仍要求真实设置标题出现。两个早期验收尝试因 OCR 漏识别按钮／中文标题而失败，没有算作通过。最终完整重跑通过，记录见 [验收证据](verification/desktop-release-refresh-2026-10-09/verification.json)。[引导](verification/desktop-release-refresh-2026-10-09/guide.png) · [战斗](verification/desktop-release-refresh-2026-10-09/battle.png) · [中文重启](verification/desktop-release-refresh-2026-10-09/restart-chinese.png) · [重启战斗](verification/desktop-release-refresh-2026-10-09/restart-battle-chinese.png)。

5项打包集成测试、2项 OCR 目标回归与 Python 编译检查通过。当前游戏604项库测试通过；模型加载、召唤计时及倒计时布局分别见 [远征加载](expedition-loading.md)、[有限召唤物计时](rare-summon-cadence.md)及[倒计时布局](gate-countdown-layout.md)。实际 Steam 客户端、Steam Deck、硬件帧率、完整打包战役、Windows／macOS仍未验证，整体发售品质见 [验收清单](release-readiness.md)。旧 Mesa 20.3.5 软件环境不能满足渲染要求，未保留降级兼容代码；玩家需要满足游戏渲染能力的驱动。


本次候选包已包含立绘完整适配、伤害数字避让、紧凑营地图标／说明、黄色精英符文光环，以及失败场景标签重试修复、骨卫／猎犬与四类有限召唤物倍速攻击修正、紧凑重建倒计时布局修正。Source `e109893` 的 SDK release 构建与只读独立运行均已重新执行；Root实际查看本轮11张截图。875项文件哈希在容器与宿主独立核对一致。此验收只覆盖短首次流程与偏好重启，不能代替打包后的完整25波战役、后续征募／支线／死亡／奖励／强化全流程、Steam客户端或硬件性能验收。
