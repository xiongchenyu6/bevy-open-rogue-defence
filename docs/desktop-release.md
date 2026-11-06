# Linux 桌面发行候选包

当前通过独立短流程验收的包来自游戏源码 `932fba4d3f4a172e9c7421be1b191bb6b989127e`：

- 目录：`target/desktop-packages-cave-feel-r3-nov6/rogue-commander-linux/`
- 压缩包：`target/desktop-packages-cave-feel-r3-nov6/rogue-commander-linux-0.1.0.tar.gz`，62,223,811字节。
- 二进制SHA256：`6e124cd0d123349b649da236931734ffc5657801e30e786bcc41233298d336a6`
- 压缩包SHA256：`1c543ebcc100aeb64d98be33e8fad5f8d7b97c11c643e23d8c69f9bd2b5082bb`

运行包内 `rogue_commander`，保持 `assets` 和 `third-party` 同级；移动使用方向键。该包是开发候选，未发布到Steam。

本包包含已接入生产场景的纹理洞窟、固定角度的响应式取景、跟随镜头方向的移动与闪避、窄竖屏工具栏和圆形触屏摇杆。存活目标暴击增加25ms短停顿，沿用同帧最大请求、0.5秒冷却及4倍以上跳过规则。此前的装备插画、合成、普通木箭、近战接触校准、挥击音效和局部目标排程修正继续包含在包中。

对应产品源码全库728通过、0失败、1忽略，严格release all-target Clippy通过。此前同轮[18个受控原生场景](verification/combat-framing-feedback-2026-11-06/README.md)完成5320更新；25ms请求在30Hz呈现探针中量化为一帧约33ms。新增[真实双相机尺寸回归](verification/primary-camera-resize-2026-11-06/README.md)修复全屏恢复时的渲染目标不同步。原生呈现探针不是实际伤害解析、主观手感或硬件验收。

## 构建和文件核验

固定Steam Runtime3 sniper SDK及Rust1.98.0，实际release构建102.461秒。531项构建输入前后与源版本逐字节一致；892项清单文件和包含清单自身的893个压缩包文件均逐项复核。ELF没有Nix依赖或库搜索路径。

```sh
tools/build_desktop_linux.sh target/my-linux-package
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner \
  -C target/my-linux-package -cf - rogue-commander-linux | gzip -n \
  > target/my-linux-package/rogue-commander-linux-0.1.0.tar.gz
```

工具拒绝覆盖已有输出。实际源输入、镜像、产物、失败记录和复核见[候选包证据](verification/cave-feel-package-2026-11-06/README.md)。后续验证器提交没有重编产品；安装包产品源码仍是932fba4，最终短流程使用单独冻结的c2a0740验证器。

## 实际独立运行

Ubuntu24／X11／软件Vulkan，断网、只读安装包、工作目录 `/tmp`，不挂载游戏源码、Rust或Nix。实际113.718秒完成首次指南、征募、严格递减计时、中英文切换、全屏恢复、偏好重启及两次正常退出；Root实际查看英文战斗、中文设置和中文重启战斗。新验证器要求两尺度精确倒计时一致及两次递减，不使用24分固定前缀或猜读字符。

另有Root直接看画面操作的375×667→1600×1000全屏→375×667恢复，以及1280×720全屏往返和实际退出。该手动会话的管理单元到达600秒watchdog，游戏仍在容器内继续并在后续实际Quit确认后返回0；保留两种状态，不称管理单元通过。完整自动短流程是另一个独立成功运行。

## 验收边界

932未重跑完整第六章、自然合成、收费刷新或旧赚取存档兼容。此前[0c真实旧档](verification/polish-next-earned-restart-2026-11-06/README.md)、[d454自然合成](verification/wallet-blades-forge-v3-2026-11-06/README.md)及[d454完整第六章](verification/wallet-blades-chapter6-2026-11-06/README.md)仍按各自版本保留，不能改算新包通过。精确重复、总体平衡、长局硬件性能、主观听感、实体手柄／音频设备、Steam客户端和其他平台仍待验收；见[发售清单](release-readiness.md)。
