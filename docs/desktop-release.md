# Linux 桌面发行候选包

当前独立运行验收通过的候选包来自游戏源码 `32cf5941c92faf465202d8c4895707d83394dffb`：

- 目录：`target/desktop-packages-sword-nov5/rogue-commander-linux/`
- 压缩包：`target/desktop-packages-sword-nov5/rogue-commander-linux-0.1.0.tar.gz`，60,302,370 字节。
- 二进制 SHA256：`f2af1f1a768935d17da4c7305a92938e7da90e94bf167e5da5947db7ad268e35`
- 压缩包 SHA256：`ae6f62ea409b3d2f96522776eab0aed9be888611c544f3740dcd4ab20caba1ec`

直接运行包内 `rogue_commander`，保持 `assets` 和 `third-party` 同级。本包包括黑市装备插画、剑类接触校准、密集伤害受击恢复、镜头反馈修正，同帧雷／冰被动与普攻回响次数修正，以及初始剑的平滑渐隐残影。它是开发候选包，未发布到 Steam，完整发售门槛见[验收清单](release-readiness.md)。

## 构建和内容核验

使用仓库固定摘要的 Steam Runtime 3 sniper SDK 和 Rust 1.98.0，使用热缓存的实际 release 构建用时 2 分 38 秒。最终 ELF 使用 `/lib64/ld-linux-x86-64.so.2`，没有 Nix 依赖或库搜索路径。878 项包内文件在打包后、宿主复核及隔离容器中逐一验证哈希；新装备插画 atlas 与仓库来源哈希一致。383 项 Rust 依赖的许可证无缺失，完整素材来源、字体 OFL、本地 `bevy_audio` 补丁及其原始许可随包保留。许可收集不是法律审核。

```sh
tools/build_desktop_linux.sh target/my-linux-package
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner \
  -C target/my-linux-package -cf - rogue-commander-linux | gzip -n \
  > target/my-linux-package/rogue-commander-linux-0.1.0.tar.gz
```

工具拒绝覆盖已有输出，并保留 SDK 的独立编译与依赖缓存。5 项打包集成测试通过。[来源、镜像与产物哈希](verification/sword-package-2026-11-05/artifact.json) · [构建日志](verification/sword-package-2026-11-05/build.log)。

## 独立运行验收

Ubuntu 24.04、X11、Mesa 软件 Vulkan：从 `/tmp` 启动只读安装包，关闭网络，不挂载游戏源码、Nix 或 Rust/Cargo。实际 UI 自动化验证：

- 首次章节选择、资源就绪后的指南、跳过指南和基础英雄征募。
- 实际战斗倒计时推进。
- 切换中文并保存，F11 全屏 1600×900、恢复 1280×720。
- 退出后用第二个游戏进程重启，恢复中文与跳过指南偏好，再次征募和战斗。
- 两次正常退出均返回 0；两个游戏日志没有缺路径、ICU、panic 或渲染错误。

Root 实际查看了本轮首次战斗和重启中文战斗两张原生截图；其他路线状态另有截图和自动断言。容器没有可听音频设备，界面正确显示设备暂不可用与自动重连，不能证明真实设备听感。[运行断言](verification/sword-package-2026-11-05/smoke/result.json) · [启动](verification/sword-package-2026-11-05/smoke/startup.png) · [战斗](verification/sword-package-2026-11-05/smoke/battle.png) · [中文设置](verification/sword-package-2026-11-05/smoke/settings-chinese.png) · [重启战斗](verification/sword-package-2026-11-05/smoke/restart-battle-chinese.png)。

```sh
mkdir -p target/package-smoke-new
docker run --rm --network none \
  --mount "type=bind,src=$PWD/target/my-linux-package/rogue-commander-linux,dst=/game,readonly" \
  --mount "type=bind,src=$PWD/target/package-smoke-new,dst=/verify" \
  --mount "type=bind,src=$PWD/tools/desktop/smoke_runtime.py,dst=/smoke.py,readonly" \
  --workdir /tmp rogue-commander-linux-smoke:ubuntu24-oct8 python3 /smoke.py
```

## 仍未覆盖

上述短流程不证明完整 25 分钟战役、后续征募／支线／死亡／奖励／强化全流程、自然合成或第六章胜利。旧 `8f4084b` 独立包已另行通过[自然赚取后购买疾风靴、扣款、库存查看及恢复战斗](verification/earned-market-2026-11-05/README.md)；旧版第六章路线被脚本恢复窗口中断，未到胜败结算。后续 `37fe665` 包在第六章自然购入并核验五件装备，但仍被红色卡名OCR误读中断，未到结算；该旧包的独立自然合成正在运行。上述最新 `32cf594` 包仅完成本页短流程，整局与自然合成尚未独立验收。

真实 Steam 客户端、Steam Deck、物理手柄与音频、Windows／macOS、更多驱动与分辨率、完整长局硬件性能仍需验收。历史 `78251d2` 候选包的独立证据保留在[旧版运行记录](verification/desktop-release-2026-10-14-r3/result.json)，不再作为当前包的通过依据。
