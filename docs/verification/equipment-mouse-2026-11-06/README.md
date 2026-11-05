# 已持有装备 Item 分支的鼠标消息集成

本轮用于定位公开 C6 路线“点击三张已持有装备，详情都停留在 Hardened Plate”的失败。空 Forge 配方选择通过并不覆盖 Item 分支，因此新增独立 `equipment-mouse` 原生捕获场景，原 equipment 场景不变。

## 测试边界

这是**受控原生鼠标消息测试，不是操作系统鼠标输入或自然获取证据**。复用现有五次实际购买 fixture：确定性库存、充足战功经真实购买处理器取得五件装备，再打开真实装备面板。fixture 在第 200 帧开始停止全部 Interaction 写入；之后的选中与关闭全部经 Bevy MouseButtonInput、CursorMoved 和逻辑 Window 的鼠标位置进入 InputSystems 与 UiSystems::Focus，不直接写 Interaction 或私有选择状态。

捕获器关闭 Winit，渲染目标为离屏 Image；Bevy 的 UiFocus 只处理 Window。该场景在输入帧将真实摄像机 RenderTarget 暂时设为逻辑 Window，UiFocus 后立即恢复原始 Image，随后才运行 UI 输入与渲染。此桥接仅用于捕获 fixture，没有新增游戏 API 或生产界面逻辑。

每个目标使用真实 Name 查找、ComputedNode 检查与 UiGlobalTransform 中心坐标，非公开路线硬编码坐标。五个已持有卡片依次执行：

1. 同一 tick 派发 Left Pressed 与 Released 消息。
2. 分开帧派发 Pressed，保持三帧，再 Released。
3. 每次验证实际 DetailName 对应预期本地化装备名称，SelectedArt 的 Image handle 与 atlas rect 精确等于对应 Item 卡片。
4. 同样通过真实鼠标消息按下 Close，验证面板消失与战斗恢复。

顺序选择覆盖明确选择锁定之后再切换其他卡片，没有绕过 Item 输入分支。

## 结果

[严格 Clippy](clippy.log) `--locked --release --all-targets -j2 -- -D warnings` 与 [捕获构建](capture-build.log) 均退出 0。[构建命令与结果](build-process.json) 和 [原生命令与结果](native-process.json) 保留。

| 场景 | 实际结果 |
| --- | --- |
| 英语 1280×720 | 5 次同 tick、5 次持续按下选中全部通过；鼠标关闭通过；380 次更新正常完成，退出 0 |
| 简体中文 667×375 | 相同十次选中与关闭全部通过；380 次更新正常完成，退出 0 |

两份原始日志均有逐项显式 PASS、最终完整 PASS、`wrote 380 simulation frames`，无 panic：[英语](en-desktop/capture.log)、[中文紧凑窗口](zh-compact/capture.log)。代表画面：[桌面选中](en-desktop/frame00322.png)、[紧凑选中血契](zh-compact/frame00322.png)、[关闭恢复](zh-compact/frame00355.png)。异步截图提供视觉证据，严格标题和原画断言来自对应同步模拟帧。

没有重现同 tick 消息导致 Item 选择失败，不能认定公开 C6 的零持续时间操作系统输入就是根因。该 C6 失败仍然保留，未被本测试转为通过。没有修改生产选择逻辑，没有启动完整自然合成或 C6 重试。

[哈希](hashes.json) 记录捕获场景源码与实际二进制。捕获时这些场景改动未提交，基础提交号不是本轮捕获场景提交号。

最终 R2 明确将恢复 RenderTarget 排在 UiFocus 之后、SettingsInputSet 之前（装备输入在 SettingsInputSet 之后）。针对这一最终源码重新执行严格 Clippy、捕获构建和两个完整原生场景，均成功；该版本记录保留在 r2-before-rustfmt。最初仅约束 after UiFocus 的通过记录保留在 initial-before-explicit-restore-order，未作为最终源码的验收替代。R2 管理任务已结束，Result=success。

新捕获模块经 rustfmt 格式化后，最终 R3 再次从严格 Clippy、捕获构建到两个原生场景完整执行。根目录日志、截图及哈希均对应最终格式化源码的 R3，管理任务已结束且 Result=success。
