# Linux 桌面发行候选包

本轮产物为 `target/desktop-packages/rogue-commander-linux/`，压缩包为 `target/desktop-packages/rogue-commander-linux-0.1.0.tar.gz`。它是release构建，可直接运行包内`rogue_commander`，需保持`assets`与`third-party`目录同级。尚未发布到Steam。

构建使用固定摘要的Steam Runtime 3 sniper SDK与Rust 1.98.0，避免把开发机Nix路径链接进程序。SDK和所选Steam运行时应匹配，见[Valve官方Linux开发说明](https://partner.steamgames.com/doc/store/application/platforms/linux)。最终ELF使用普通Linux解释器，无Nix依赖或库搜索路径。

从仓库根目录执行：

```sh
tools/build_desktop_linux.sh target/my-linux-package
```

工具拒绝覆盖已有包。它复制完整素材与来源文件，收集目标平台正常／构建依赖的许可证，保留版本、来源、SHA256和875项文件清单。当前383项依赖记录无缺失文本；16项许可证补充记录在`licenses/rust-overrides.json`。其中两项上游只声明SPDX而未提供独立许可文件，保留固定提交的原始声明，并附对应官方标准全文，清单明确区分来源，没有补造作者。项目原创音效、模型来源与字体OFL也随包保留。收集记录不是法律审核。

`tools/package_desktop.py`可显式打包已有二进制；release会拒绝Nix依赖、缺失许可证和既有输出。debug模式明确标记为开发测试包，不能替代发行候选包。

## 运行验收

Ubuntu 24.04独立运行时使用Mesa 25.2.8软件Vulkan和X11。从`/tmp`启动只读游戏目录，不挂载项目源码、Nix或Rust/Cargo，网络关闭。验证875项文件哈希后，实际操作章节、基础角色征募、战斗、中文设置、F11全屏／恢复、退出和重新启动；中文偏好恢复，两次正常退出均返回0。素材与中文正常，没有缺资源或ICU错误。

可复跑自动验收：

```sh
docker build -f tools/desktop/Dockerfile.smoke-modern -t rogue-commander-linux-smoke:ubuntu24 tools/desktop
mkdir -p target/package-smoke
docker run --rm --network none \
  --mount "type=bind,src=$PWD/target/my-linux-package/rogue-commander-linux,dst=/game,readonly" \
  --mount "type=bind,src=$PWD/target/package-smoke,dst=/verify" \
  --mount "type=bind,src=$PWD/tools/desktop/smoke_runtime.py,dst=/smoke.py,readonly" \
  --workdir /tmp rogue-commander-linux-smoke:ubuntu24 python3 /smoke.py
```

[启动](verification/desktop-release-2026-10-06/startup.png) · [征募](verification/desktop-release-2026-10-06/recruit.png) · [战斗](verification/desktop-release-2026-10-06/battle.png) · [中文设置](verification/desktop-release-2026-10-06/settings-chinese.png) · [重启恢复](verification/desktop-release-2026-10-06/restart-chinese.png) · [验收记录](verification/desktop-release-2026-10-06/validation.txt)。

旧Mesa 20.3.5 lavapipe的软件测试不能满足基本渲染能力，启动失败；它不作为支持环境。测试中的局部GPU预处理回退无法解决光照与呈现限制，已删除，未保留兼容层。游戏需要支持所需Vulkan特性的当前驱动。没有据软件渲染结果宣称硬件帧率、Steam客户端或Steam Deck已通过。

5项打包集成测试、Python编译与shell语法检查通过。游戏Rust源码仍对应已验证的`437b675`，本轮没有改变游戏机制。Windows／macOS、真实Steam客户端、硬件性能、完整新手引导及全章节平衡仍见[发售体验清单](release-readiness.md)。
