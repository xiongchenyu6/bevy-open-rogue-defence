# 六章平衡审计

本轮扩展生产系统模拟器，补齐章节和永久强化维度，并完成 72 局整局审计。尚未据此修改生产战斗数值，不能把工具验证称为平衡完成。

## 结果

两个种子 A11CE、BEEF，三个基础职业，六章，城墙后固定站位，零永久强化。每章主动成长 6 局全部通关，均零死亡；每章不购买可选成长的 6 局全部失败。72 局均实际结束，章节、强化与技能计数校验通过，无截尾或无效局。

| 章节 | 主动成长通关 | 英雄死亡 | 城墙倒塌次数范围 | 不选可选成长失败 |
| --- | --- | --- | --- | --- |
| 1 | 6/6 | 0 | 0–15 | 6/6 |
| 2 | 6/6 | 0 | 0–26 | 6/6 |
| 3 | 6/6 | 0 | 1–28 | 6/6 |
| 4 | 6/6 | 0 | 3–24 | 6/6 |
| 5 | 6/6 | 0 | 15–48 | 6/6 |
| 6 | 6/6 | 0 | 14–64 | 6/6 |

A11CE 战士开局的主动路线，第一章结束生命上限 57,737.79，第六章 48,762.53；第六章城墙倒塌 58 次、累计暴露 234.35 秒，仍未消耗生命。章节提高敌人生命到 8.75 倍、伤害到 2.25 倍；玩家羁绊的属性成长、生命倍率和回复需与敌人输出共同复核。这是进一步调整的证据，不足以单独证明某个倍率或回复存在代码错误。

下一轮应测生命比例低点、实际受伤与回复来源，再比较修改前后主动路线、被动路线和强化收益。现有绝对最低生命会被早期小生命上限主导，不能解释为后期从未受伤。

## 口径与限制

模拟运行完整生产 GamePlugin，关闭渲染和音频，固定 60Hz、单线程调度。主动策略按真实价格持续选择第一张可用卡及黑市装备；被动策略只完成系统强制英雄选择。两者均不主动走位、启动试炼或打支线。不会把此结果外推所有玩家、最优构筑、性能或完整通关经济。

章节夹具把之前章节标为已通关，并直接设置六项相同强化等级。这用于隔离章节战斗，不代表通过之前章节真实积累的资源。本轮整局均强化 0；另有 12 个短局验证章节 0–5 和强化 0/2 的真实应用，短局全部截尾，不作胜率证据。

模拟器二进制 SHA256：`9f2a78cd3710f5304d7c8872969dd2443bd93423912bcc6a77a8889a83d1726b`。生产战斗规则基于提交 `043483f`，本轮只扩展模拟与分析工具。

[逐局结果](balance/campaign-audit-2026-10-06/analysis/cases.csv)、[采样曲线](balance/campaign-audit-2026-10-06/analysis/curves-30s.csv)、[原始记录索引](balance/campaign-audit-2026-10-06/summary.json)、[原始 JSONL 校验值](balance/campaign-audit-2026-10-06/raw-sha256.json)。

## 复现与工具验证

```sh
python3 tools/run_balance_matrix.py --binary target/debug/balance_sim \
  --batch campaign-six-chapters-zero-training --frames 110000 \
  --seeds A11CE,BEEF --starters 0,1,2 --positions behind \
  --policies passive,active --chapters 0,1,2,3,4,5 --training 0 \
  --jobs 4 --case-timeout 3600 --require-telemetry
python3 tools/analyze_balance.py target/verification/campaign-six-chapters-zero-training \
  --output target/verification/campaign-six-chapters-analysis
python3 -m unittest tools/test_balance_matrix.py
```

4 项工具测试通过，覆盖并发多维存档隔离、越界输入、章节身份不符及超时收据。格式检查、全目标 Clippy 严格检查和模拟器构建通过。未新增生产战斗改动，本轮未重跑库测试或原生渲染截图；之前的技能节奏原生验收见 [第一章报告](combat-balance.md)。

## 后续生命压力观测

新增逐帧 `min_alive_health_fraction`（存活期间的最低生命比例）、`max_health_deficit`（最大缺血量）和 `injured_secs`（存活且缺血超过 0.01 的游戏时间）。它们不把复活等待计为受伤时间，也不把成长后的数万生命与开局数百生命直接比较。数据仍为帧末观测，不能替代逐次受击、治疗或吸血来源统计；同帧受到伤害后立即回满可能不会记录为缺血。旧收据没有这些字段，分析结果保留空值，禁止从旧绝对最低生命补造比例。

该观测改动的 Rust 测试通过（1 项），模拟器构建、格式检查及模拟器严格 Clippy 检查通过，Python 工具测试 4 项通过。第六章三职业、两种子、零强化的整局复测已启动，批次 `campaign-sixth-health-pressure`；完成前不将中间采样计为胜败结果。
