# Linux 桌面发行候选包

当前通过独立短流程验收的包来自源码 `bdd6660f16514a1ba1bd506c3a58f42916183e10`，包含 `3d42d0b` 冰霜修正及此前城墙／火焰结算修正：

- 目录：`target/desktop-packages-glacier-nov6/rogue-commander-linux/`
- 压缩包：`target/desktop-packages-glacier-nov6/rogue-commander-linux-0.1.0.tar.gz`，62,295,464字节。
- 二进制SHA256：`f10dd77c63e69916e5cf3471ff94dc1f2cbe1b591f4105977df648b5d18252a8`
- 压缩包SHA256：`5bacb0ff3301c69bd90525252a4740c4ff7d690ef038c1e51990971f3e7d409e`

运行包内 `rogue_commander`，保持 `assets` 和 `third-party` 同级；方向键移动。这是未发布到Steam的开发候选。

本轮让城墙逐怪结算、火焰同距目标和冰霜区域脉冲采用稳定位置顺序，避免部分无关实体变化交换伤害或暴击。冰霜伤害、范围、0.8秒脉冲和第三次冻结规则不变。对应[局部真实红绿回归](verification/glacier-order-2026-11-06/README.md)与完整库测试740通过、0失败、1既有忽略、严格Clippy和模拟器构建均有记录。包内游戏源码／资产／vendor字节与该验证输入一致，SDK未另跑740项测试。

相对此前f205候选，交付清单只有游戏程序和 `version.json` 改变，全部资产保持原哈希。正式洞窟、响应式取景、镜头方向输入、窄屏HUD、城门遮挡淡出、普通木箭、近战接触反馈、挥击音效、装备插画与合成继续包含在包中；其专项受控证据仍按各自源码版本保留。

## 构建与文件核验

固定Steam Runtime3 sniper SDK、Rust1.98.0，实际构建108.669秒。537项构建／运行输入前后与固定提交逐字节一致；Root独立核对892项清单文件和893个压缩包常规文件，全部压缩包条目的所有者与时间为0。ELF检查无Nix依赖或搜索路径。

```sh
tools/build_desktop_linux.sh target/my-linux-package
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner \
  -C target/my-linux-package -cf - rogue-commander-linux | gzip -n \
  > target/my-linux-package/rogue-commander-linux-0.1.0.tar.gz
```

工具拒绝覆盖既有输出。实际源码、镜像、输入清单、完整短流程和截图见[新候选包证据](verification/glacier-package-2026-11-06/README.md)。六个验收helper也固定为产品提交的原始字节。

## 实际独立运行

Ubuntu24／X11／软件Vulkan，断网、只读安装包、工作目录 `/tmp`，不挂载游戏源码、Rust或Nix。实际117.161秒完成首次指南、征募、战斗计时推进、中文设置、全屏恢复、偏好重启、再次征募／战斗及两次正常退出。两次启动的精确倒计时端点分别1498→1492与1487→1481，双尺度读取一致；一次不一致的中间OCR观察被拒绝并保留，未猜读替换。

Root实际查看英文战斗、中文设置和中文重启战斗，未见设置裁切，英雄在城门安全侧可辨。设置页显示测试环境无可用音频设备及自动重连提示，本次没有物理声音或主观听感验收。通用短流程也没有实际按键穿越城门；此前[三尺寸遮挡对照](verification/gate-cutaway-2026-11-06/README.md)仍为独立受控证据。

## 验收边界

[同游戏逻辑的Nix模拟器完整重复](verification/post-glacier-repeat-2026-11-06/README.md)仍在后期出现冰霜普攻黑市触发效果的目标／状态差异，精确比较失败；未在SDK包重跑完整重复，不能宣称全局确定性或总体平衡。

本包未重跑完整第六章、自然合成、收费刷新或旧赚取存档兼容。此前[0c真实旧档](verification/polish-next-earned-restart-2026-11-06/README.md)、[d454自然合成](verification/wallet-blades-forge-v3-2026-11-06/README.md)、[d454完整第六章](verification/wallet-blades-chapter6-2026-11-06/README.md)及[f205候选短流程](verification/gate-cutaway-package-2026-11-06/README.md)按各自版本保留，不改算本包通过。长局硬件性能、实体手柄／音频设备、Steam客户端和其他平台仍待验收；见[发售清单](release-readiness.md)。
