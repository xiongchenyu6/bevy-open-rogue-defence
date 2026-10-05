# 深化策划 v2 实施计划

设计依据：`docs/game-design.md` §15 异常清单与 §16 深化设计 v2。

## Stage 1: 规则正确性
**Goal**: 修掉会破坏规则的异常：调试功能随正式版发布、章节倍率放大挑战怪、战果刻印输赢无差别、Boss 数据三处重复、起手技能越阶、自动抽羁绊 60 秒节流、遗留兼容字段。
**Success Criteria**: 正式构建没有 F7/F8；第 6 章塔 1 层仍是 1,200 HP；击破刻印比超时多拿碎片；Boss 数值只有一张表；起手技能品阶 ≤ 英雄品阶 + 1。
**Tests**: 每条规则一个单元测试；`cargo test`、`cargo clippy --all-targets` 全绿。
**Status**: Complete（445 tests）

## Stage 2: 基线测量
**Goal**: 在工作区快照里用 `balance` 长局模拟跑多个种子，量出每分钟等级、金币收支、木材、技能总等级、塔层、W 级、DPS，并给出塔 15–24 层曲线、技能价格、招募等级的推荐值。
**Success Criteria**: 一张基线表 + 一组有模拟证据的推荐常量。
**Tests**: 同种子两次结果一致。
**Status**: In Progress

## Stage 3: 经济深化（金币出口）
**Goal**: 技能池抽空后 G 给出"战备训练"三选一（属性成长，价格递增）；技能价格、塔曲线、招募里程碑按 Stage 2 推荐值落地。
**Success Criteria**: 模拟长局结束时未花金币 ≤ 收入的 15%；20:00 前技能不会全部满级；塔可达层数 16–20。
**Tests**: 训练卡结算、价格公式、塔表单调性测试；长局模拟复核。
**Status**: Not Started

## Stage 4: 奇遇系统
**Goal**: 每击败一个检查点 Boss 存入一次免费"奇遇"三选一，12 个有取舍的事件，复用抽卡面板。
**Success Criteria**: 一局出现 4 次奇遇；每个事件效果有测试；中英文案齐全；手机布局不溢出。
**Tests**: 事件结算单测 + capture 截图场景。
**Status**: In Progress

## Stage 5: 文档与验收
**Goal**: `gameplay-spec.md`、README、专题文档与代码同步；`game-design.md` 更新；全量测试 + 截图验收。
**Success Criteria**: 异常清单 M8–M10、L5–L7 关闭；所有新功能有截图证据。
**Tests**: `cargo test`、clippy、wasm `cargo check`。
**Status**: Not Started
