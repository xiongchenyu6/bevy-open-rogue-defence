# Linux 桌面发行候选包

当前通过独立短流程验收的包来自游戏源码 `0c12230d3bf25c2b1a070c80950e5388a3b7b2ec`：

- 目录：`target/desktop-packages-polish-next-nov6/rogue-commander-linux/`
- 压缩包：`target/desktop-packages-polish-next-nov6/rogue-commander-linux-0.1.0.tar.gz`，60,931,013 字节。
- 二进制 SHA256：`b19b938f0f058c90bca35edec95cb87f39fe47f77e9e0d14720bdda012f45d91`
- 压缩包 SHA256：`019aff09b4d6ee63df2503628acb2b179216995ecfdcb32ea61c932d6c34bb23`

运行包内 `rogue_commander`，保持 `assets` 和 `third-party` 同级；移动使用方向键。它是开发候选包，未发布到 Steam。

本包在此前装备插画、合成界面、普通木箭、接触校准、钱包同步和城门状态修正的基础上，包含六类近战透明渐隐、低帧率首击挤压、剑／斧／匕首挥击声，以及 StormMatrix、回旋刃和水波的局部目标排程修正。最近对应源码的全库测试为704通过、0失败、1忽略，严格 Clippy 通过。各项源码与受控原生证据见[发售清单](release-readiness.md)；这些证据不等同于每项效果都在新包内经过完整长局验收。

未通过验收的洞口模型和候选相机没有进入正式场景。可选捕获工具不是游戏默认相机。

## 构建和文件核验

使用固定摘要的 Steam Runtime 3 sniper SDK 和 Rust 1.98.0，实际热缓存 release 构建124.367秒。525项构建输入在构建前后与源码版本逐字节一致。ELF 使用 `/lib64/ld-linux-x86-64.so.2`，没有 Nix 依赖或库搜索路径。

889项清单文件均核对尺寸与SHA256；加上清单自身，实际目录与压缩包都恰有890个文件，内容逐字节一致。压缩包采用固定时间、所有者和排序。依赖许可证收集无缺失，素材来源、字体许可及本地音频补丁许可随包保留；这不是法律审核。

```sh
tools/build_desktop_linux.sh target/my-linux-package
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner \
  -C target/my-linux-package -cf - rogue-commander-linux | gzip -n \
  > target/my-linux-package/rogue-commander-linux-0.1.0.tar.gz
```

工具拒绝覆盖已有输出。实际命令、源输入、镜像、产物及独立复核见[新包证据](verification/polish-next-package-2026-11-06/README.md)。证据提交 `e71a9f3` 没有重新编译游戏；包的源版本仍为 `0c12230`。

## 实际独立运行

Ubuntu24／X11／软件 Vulkan，关闭网络，从 `/tmp` 启动只读安装包，不挂载游戏源码、Rust或Nix。实际111.633秒短流程通过首次指南、征募、战斗计时、中英文切换、全屏恢复、偏好重启及两次正常退出。Root独立查看英文战斗与重启后的中文战斗画面。

另以旧版真实通关赚取的原始存档，分别复制到全新用户目录：公开界面核对630战果货币、六章CLEARED、Storm Savant一级／力量34、局内装备0、关闭面板后计时推进及正常退出，存档字节始终不变。首轮征募后过早读取HUD的失败保留；后续使用同一名称区域的一次正常按压并观察界面响应，通过122.044秒流程。再一次131.977秒流程确认包内空军械架插画实际显示，未改变产品代码。见[真实旧档复验](verification/polish-next-earned-restart-2026-11-06/README.md)。这不证明唯一点击失败原因或精确图片加载时延。

## 验收边界

新包没有重跑完整第六章、自然合成或付费刷新。此前 `d4540c4` 的[自然合成与同档重启](verification/wallet-blades-forge-v3-2026-11-06/README.md)、[完整第六章及奖励强化重启](verification/wallet-blades-chapter6-2026-11-06/README.md)仍是有效的旧版证据，不能改算新包通关。此次只验证原始已赚取存档的兼容和短流程。

全局精确重复、更多构筑与种子的平衡、长局硬件性能、主观听感、实体手柄／音频设备、Steam客户端、SteamDeck及其他操作系统仍未完成。完整门槛见[发售清单](release-readiness.md)。
