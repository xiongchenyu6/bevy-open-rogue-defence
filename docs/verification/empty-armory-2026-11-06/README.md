# 空背包装饰：磨损的空军械架

空背包现在在原预览位置展示一座原创生成的空军械架，标题在旁，下方保留原说明。架上挂钩与底板均空，没有暗示已持有装备、奖励或新操作。使用磨损木材、黑铁和低饱和旧铜细节，避免发亮塑料感。[原画预览](art-preview.png)。

新增资产仅为 `assets/ui/armory/empty-rack.png`，384×256 RGBA、106233 字节。保留真正透明的四周和架内开口。[素材来源与许可说明](../../../assets/ui/armory/SOURCE.md)、[两次生成记录](../../../assets/ui/armory/generation.json)、[源与运行图哈希/机械缩放记录](../../../assets/ui/armory/manifest.json) 均已归档。没有导入其他游戏图片，没有改动已有市场或合成图集、GLB 或材质。

## 最终布局

只对“非 Forge、普通装备和合成装备都为空”的界面替换原空白预览：桌面 220×147，紧凑与竖屏 110×73，Name 在旁。同一张图只出现一次，详情区下方没有第二张图。原 `No equipment yet` 文案、header、Gear/Forge/Close、已持有卡片几何与颜色保留。已持有装备仍走原 176/88 SelectedArt 分支。

| 实际场景 | 画面 |
| --- | --- |
| 英语 1280×720 | [最终桌面](en-desktop/frame00125.png) |
| 简体中文 667×375 | [最终紧凑](zh-compact/frame00125.png) |
| 英语 390×844 | [最终竖屏](en-portrait/frame00125.png) |
| 原购买/已持有流程 | [已持有装备无空架](owned-regression/frame00230.png) |

首轮将图放在 detail 下方，仍保留了上方无用空白，因此未选作最终布局。[R1 截图/日志/源码快照](r1-detail-placement/) 保留；R1 五场景本身通过，不代表布局被接受。根目录全部验证日志、截图与哈希对应修订后的 R2 源码。主代理已实际审阅 R2 英语桌面、中文紧凑、英语竖屏，以及缺图第 65 帧到恢复第 125 帧；R1 是被布局审查替换，并非测试失败。

## 真实缺图检查

装饰图在 `Assets<Image>` 真实可用前保持 Display::None，不用白色默认图片替代。它不是玩法必需依赖，因此不阻塞战斗/招募；正常空状态文字始终保留。

受控捕获 `equipment-empty-missing` 使用独立真实文件读取器：先阻塞新图的读取，再释放并实际读取缺失路径，确认 AssetServer 为 Failed、图仍隐藏、原文字可见；之后恢复真实文件并 reload，同一面板显示真实图片。没有写入假的加载状态或删除生产资产。[缺图](missing-recovery/frame00065.png) → [恢复](missing-recovery/frame00125.png)；[原始日志](missing-recovery/capture.log) 含显式 missing 与 recovery PASS。

## 验证结果与边界

- [当前实际全库测试](full-tests.log)：681 通过，0 失败，1 忽略。
- [严格 Clippy](clippy.log)：`--locked --release --all-targets -j2 -- -D warnings` 通过；[捕获构建](capture-build.log) 通过。
- 五个实际原生场景全部退出 0，显式 PASS、完整帧数、无 panic：四个空状态/缺图场景各 150 帧，加原 equipment 五次真实购买 fixture 310 帧，共 910 帧。[命令与结果](native-process.json)。
- [位图检查](bitmap-check.log)：运行路径匹配、哈希、尺寸、有效非空内容、透明边缘与架内开口通过。

`python3 docs/verification/empty-armory-2026-11-06/verify.py` 仅验证归档；`uv run --with pillow python tools/verify_empty_armory_art.py` 检查实际运行素材。前者不启动游戏。

这些是新 UI 源码的受控原生截图与真实图像 I/O 检查，不是自然获取装备、操作系统鼠标或新发行包验收。没有改变玩法、经济、攻击目标或公开界面助手。固定旧包上的自然合成与重启证据不被这次源码截图替代。
