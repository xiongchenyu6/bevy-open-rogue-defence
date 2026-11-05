# 装备与合成界面原生验收

本轮将背包改为“收藏列表 + 当前装备大图 + 完整效果”的布局，使用现有装备与六种合成装备原画。没有新增装备槽位或改变合成规则。

桌面左侧为固定宽度收藏区，右侧展示选中装备的大图、品质、名称、材料与全部继承效果。效果独立滚动，合成按钮固定在滚动区之外。紧凑横屏缩小缩略图；竖屏将收藏区放在详情上方，保留可见的操作按钮。

## 实际发现并修复的问题

首次原生手柄路线在紧凑窗口第 70 步失败：选择已备齐材料的 Bloodreaver 后，向下导航经过 Suncoil 卡片，悬停预览将选中配方换成缺材料的 Suncoil，导致合成按钮禁用。[原始失败日志](before-controller-compact/capture.log) 与 [失败画面](before-controller-compact/frame01500.png) 保留。

现在首次点击或 A 键明确选择后，仅移动焦点不会替换选中装备；再次点击或 A 键可以换选。关闭或切换标签重置锁定。生产输入与详情系统的 ECS 回归覆盖了这条路径。原有原生手柄驱动和校验没有放宽。

## 验证结果

- [完整单元测试](full-tests.log)：673 通过，0 失败，1 忽略。
- [严格 Clippy](clippy.log)：`--locked --release --all-targets -j2 -- -D warnings` 完成。
- [原生执行汇总](native-sequence.log) 与 [管理任务终态](native-manager.log)：全部 11 个用例完成，累计 8880 次更新，均包含显式 PASS、预定帧数完成记录，成功用例无 panic。
- 装备与合成：8 个英语/简体中文用例，覆盖 1280×720、667×375、390×844。
- 手柄桌面及紧凑窗口：各 80 步、2400 次更新完整结束；滚动路线 13 步、1600 次更新完整结束。
- 右摇杆向下滚动从 0 到 200.00002，向上恢复到 0.000011444092；两方向均有显式 PASS。

## 代表画面

| 场景 | 证据 |
| --- | --- |
| 桌面装备大图 | [装备详情](after/equipment-en-desktop/frame00230.png) |
| 中文紧凑背包 | [背包](after/equipment-zh-compact/frame00230.png) |
| 竖屏装备 | [装备](after/equipment-en-portrait/frame00230.png) |
| 中文竖屏 World Engine 长效果 | [长效果与固定按钮](after/crafting-1-zh-portrait/frame00274.png) |
| 紧凑手柄可合成 | [选中 Bloodreaver](after/controller-compact/10-forge-ready.png) |
| 紧凑手柄合成完成 | [保留 Bloodreaver 与 Already forged](after/controller-compact/11-forged.png) |
| 桌面手柄合成完成 | [完成状态](after/controller-desktop/11-forged.png) |
| 紧凑效果滚动 | [滚动前](after/controller-scroll-compact/scroll-before.png)、[滚动后](after/controller-scroll-compact/scroll-after.png)、[恢复](after/controller-scroll-compact/scroll-restored.png) |

## 复核与边界

运行 `python3 docs/verification/inventory-split-2026-11-05/verify.py` 只检查已归档日志和 SHA-256，不启动游戏。每个用例的原始日志保存在 `after/<case>/capture.log`；[执行脚本](native.py) 记录场景、语言、分辨率、更新数及成功条件。[源码与二进制哈希](source-hashes.json) 标识捕获时的实际文件；当时 UI 改动尚未提交，基础提交号不是本轮 UI 提交号。原始失败没有可核实的二进制哈希，不作补造。

这是实际原生渲染和生产 UI 路径的验证。库存、钱包由确定性 fixture 准备，再通过实际购买和合成处理器执行；不能当作自然积累材料的流程证据。手柄使用合成的原生 Gamepad 事件，不代表实体手柄验收。独立滚动保留全部效果，长文案需要滚动查看。

本轮未更新公开界面驱动坐标或制作新发行包，也不证明完整战役、实体设备、性能或商店发行准备完成。此前不可变包和公开路线仍对应旧布局。
