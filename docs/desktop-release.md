# Linux 桌面发行候选包

当前独立运行验收通过的候选包来自游戏源码 `040ce5c1084634f3e8d886b65d1f5c2c467997af`：

- 目录：`target/desktop-packages-impact-nov5/rogue-commander-linux/`
- 压缩包：`target/desktop-packages-impact-nov5/rogue-commander-linux-0.1.0.tar.gz`，60,754,413 字节。
- 二进制 SHA256：`a16998fe43d9fa7c830c3614aaae9e61b982579fc718195fea232c312c77a1de`
- 压缩包 SHA256：`ca8c7f4a131d61e4272ef150d8259b9844a934a6597ae9cec29f231629aa7446`

直接运行包内 `rogue_commander`，保持 `assets` 和 `third-party` 同级。移动使用方向键；当前包内旧说明的WASD标注有误，源码随包说明已修正，待下次打包替换。本包包括黑市装备插画、剑类接触校准、密集伤害受击恢复、镜头反馈修正，同帧雷／冰被动与普攻回响次数修正，敌人受击短闪与材质隔离，以及初始剑的平滑渐隐残影、普通木箭和六件合成装备独立插画。它是开发候选包，未发布到 Steam，完整发售门槛见[验收清单](release-readiness.md)。

## 构建和内容核验

使用仓库固定摘要的 Steam Runtime 3 sniper SDK 和 Rust 1.98.0，使用热缓存的实际 release 构建用时 1 分 44 秒。最终 ELF 使用 `/lib64/ld-linux-x86-64.so.2`，没有 Nix 依赖或库搜索路径。882 项包内文件在打包后、宿主复核及隔离容器中逐一验证哈希；新装备插画 atlas 与仓库来源哈希一致。383 项 Rust 依赖的许可证无缺失，完整素材来源、字体 OFL、本地 `bevy_audio` 补丁及其原始许可随包保留。许可收集不是法律审核。

```sh
tools/build_desktop_linux.sh target/my-linux-package
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner \
  -C target/my-linux-package -cf - rogue-commander-linux | gzip -n \
  > target/my-linux-package/rogue-commander-linux-0.1.0.tar.gz
```

工具拒绝覆盖已有输出，并保留 SDK 的独立编译与依赖缓存。5 项打包集成测试通过。[来源、镜像与产物哈希](verification/impact-package-2026-11-05/artifact.json) · [构建日志](verification/impact-package-2026-11-05/build.log)。

## 独立运行验收

Ubuntu 24.04、X11、Mesa 软件 Vulkan：从 `/tmp` 启动只读安装包，关闭网络，不挂载游戏源码、Nix 或 Rust/Cargo。实际 UI 自动化验证：

- 首次章节选择、资源就绪后的指南、跳过指南和基础英雄征募。
- 实际战斗倒计时推进。
- 切换中文并保存，F11 全屏 1600×900、恢复 1280×720。
- 退出后用第二个游戏进程重启，恢复中文与跳过指南偏好，再次征募和战斗。
- 两次正常退出均返回 0；两个游戏日志没有缺路径、ICU、panic 或渲染错误。

Root 实际查看了本轮首次战斗和重启中文战斗两张原生截图；其他路线状态另有截图和自动断言。容器没有可听音频设备，界面正确显示设备暂不可用与自动重连，不能证明真实设备听感。[运行断言](verification/impact-package-2026-11-05/smoke/result.json) · [启动](verification/impact-package-2026-11-05/smoke/startup.png) · [战斗](verification/impact-package-2026-11-05/smoke/battle.png) · [中文设置](verification/impact-package-2026-11-05/smoke/settings-chinese.png) · [重启战斗](verification/impact-package-2026-11-05/smoke/restart-battle-chinese.png)。

```sh
mkdir -p target/package-smoke-new
docker run --rm --network none \
  --mount "type=bind,src=$PWD/target/my-linux-package/rogue-commander-linux,dst=/game,readonly" \
  --mount "type=bind,src=$PWD/target/package-smoke-new,dst=/verify" \
  --mount "type=bind,src=$PWD/tools/desktop/smoke_runtime.py,dst=/smoke.py,readonly" \
  --workdir /tmp rogue-commander-linux-smoke:ubuntu24-oct8 python3 /smoke.py
```

## 仍未覆盖

上述短流程不证明完整25分钟战役、后续征募／支线／死亡／奖励／永久强化全流程或自然合成。旧37fe665路线有五件自然装备和同局实际星界棱镜合成证据，但后到征募中断其恢复／退出；旧32cf594路线也因公开UI脚本读数或开启时序中断，均不能当作新版整局结论。当前040ce5c的自然合成与第六章路线已冻结脚本、复制真实存档，正在顺序复验，尚未通过。

真实Steam客户端、SteamDeck、物理手柄与音频、Windows／macOS、更多驱动／分辨率和长局硬件性能仍需验收。历史证据继续保留，但不作为当前包的通过依据。
