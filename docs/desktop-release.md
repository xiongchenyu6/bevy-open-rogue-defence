# Linux 桌面发行候选包

当前独立运行验收通过的候选包来自游戏源码 `8f4084b22b55be0040a8cb0f7f5a50bd470eb126`：

- 目录：`target/desktop-packages-nov5/rogue-commander-linux/`
- 压缩包：`target/desktop-packages-nov5/rogue-commander-linux-0.1.0.tar.gz`，60,264,483 字节，约 57.5 MiB。
- 二进制 SHA256：`d1f3c7dfc944376012ff5949b60cb1dea56117703ddb0bec36a02aa552b97d4f`
- 压缩包 SHA256：`d98b27d24e2eecdf64ad942a4cfafd0c3990683e15bd07659c309bf055aea22b`

直接运行包内 `rogue_commander`，保持 `assets` 和 `third-party` 同级。本包包括黑市装备插画、剑类接触校准、密集伤害受击恢复及镜头反馈修正。它是开发候选包，未发布到 Steam，完整发售门槛见[验收清单](release-readiness.md)。

## 构建和内容核验

使用仓库固定摘要的 Steam Runtime 3 sniper SDK 和 Rust 1.98.0，实际 release 构建用时 15 分 20 秒。最终 ELF 使用 `/lib64/ld-linux-x86-64.so.2`，没有 Nix 依赖或库搜索路径。878 项包内文件在打包后、宿主复核及隔离容器中逐一验证哈希；新装备插画 atlas 与仓库来源哈希一致。383 项 Rust 依赖的许可证无缺失，完整素材来源、字体 OFL、本地 `bevy_audio` 补丁及其原始许可随包保留。许可收集不是法律审核。

```sh
tools/build_desktop_linux.sh target/my-linux-package
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner \
  -C target/my-linux-package -cf - rogue-commander-linux | gzip -n \
  > target/my-linux-package/rogue-commander-linux-0.1.0.tar.gz
```

工具拒绝覆盖已有输出，并保留 SDK 的独立编译与依赖缓存。5 项打包集成测试通过。[来源、镜像与产物哈希](verification/desktop-release-2026-11-05/artifact.json) · [构建日志](verification/desktop-release-2026-11-05/build.log)。

## 独立运行验收

Ubuntu 24.04、X11、Mesa 软件 Vulkan：从 `/tmp` 启动只读安装包，关闭网络，不挂载游戏源码、Nix 或 Rust/Cargo。实际 UI 自动化验证：

- 首次章节选择、资源就绪后的指南、跳过指南和基础英雄征募。
- 实际战斗倒计时推进。
- 切换中文并保存，F11 全屏 1600×900、恢复 1280×720。
- 退出后用第二个游戏进程重启，恢复中文与跳过指南偏好，再次征募和战斗。
- 两次正常退出均返回 0；两个游戏日志没有缺路径、ICU、panic 或渲染错误。

Root 实际查看了本轮启动页、首次战斗、中文设置和重启中文战斗四张原生截图；其他路线状态另有截图和自动断言。容器没有可听音频设备，界面正确显示设备暂不可用与自动重连，不能证明真实设备听感。[运行断言](verification/desktop-release-2026-11-05/result.json) · [启动](verification/desktop-release-2026-11-05/startup.png) · [战斗](verification/desktop-release-2026-11-05/battle.png) · [中文设置](verification/desktop-release-2026-11-05/settings-chinese.png) · [重启战斗](verification/desktop-release-2026-11-05/restart-battle-chinese.png)。

```sh
mkdir -p target/package-smoke-new
docker run --rm --network none \
  --mount "type=bind,src=$PWD/target/my-linux-package/rogue-commander-linux,dst=/game,readonly" \
  --mount "type=bind,src=$PWD/target/package-smoke-new,dst=/verify" \
  --mount "type=bind,src=$PWD/tools/desktop/smoke_runtime.py,dst=/smoke.py,readonly" \
  --workdir /tmp rogue-commander-linux-smoke:ubuntu24-oct8 python3 /smoke.py
```

## 仍未覆盖

上述短流程不证明完整 25 分钟战役、后续征募／支线／死亡／奖励／强化全流程、自然合成或第六章胜利。后续独立包已另行通过[自然赚取后购买疾风靴、扣款、库存查看及恢复战斗](verification/earned-market-2026-11-05/README.md)；第六章整局后来被10级征募遮挡时钟的脚本问题中断，未到胜败结算。

真实 Steam 客户端、Steam Deck、物理手柄与音频、Windows／macOS、更多驱动与分辨率、完整长局硬件性能仍需验收。历史 `78251d2` 候选包的独立证据保留在[旧版运行记录](verification/desktop-release-2026-10-14-r3/result.json)，不再作为当前包的通过依据。
