# 48 次冻结版本描述性观察：终态与重复差异

48 次执行全部有真实终态，24 个匹配 A/B 配置中有 **1 个胜负翻转、15 个终态字段不同**。这批数据不能作为精确重放基线，也不足以给当前游戏下总体胜率结论。

数据只对应 gate `237e65086af203da66900bb12d86e6e7a70b8bdf` + simulator/root `5f357900c5619891dcbcaeab0e8f76870001a9a4` 的组合、binary `75c69149…`，在双刀修正之前。185 个冻结美术文件（复制记录 HEAD 为 `9bf313c…`）及 2 个 runner/config 文件均独立重算 hash；并非当前 d454 Linux 包。

配置为两章（索引 4/5，即第 5/6 章）、active/passive、3 个开局角色、2 个 seed，每配置重复 A/B。全部使用强制“此前章节已清、六项永久升级为零”的模拟夹具，并非自然积累的存档。renderer=false、固定 1/60 步长、single-threaded、120000 更新上限、每 case 900 秒；无压力覆盖。48 个实际子进程均 exit 0、valid=true、finished=true，没有 timeout、frame-cap 或 censor；不能把执行成功误写成胜利。

| 章节 | 策略 | A 的 6 次终态 | B 的 6 次终态 |
|---|---|---|---|
| 5 | active | 6 胜 | 6 胜 |
| 5 | passive | 6 负 | 6 负 |
| 6 | active | 5 胜、1 负 | 4 胜、2 负 |
| 6 | passive | 6 负 | 6 负 |

总计是 21 个胜、27 个负的执行记录；A/B 重复和仅两个 seed 不应当作 48 个独立总体样本。

## 重复差异

24 对的结果中 23 对胜负一致，1 对翻转；15 对终态观察字段不同，14 对在共同 checkpoint 更新帧上出现不同。仅 9 对在记录到的终态及共同检查点字段上均一致，这仍不证明未记录的内部 bit/state 或 actor/contact 顺序完全一致。`checkpoint-differences.csv` 保存 5884 行逐字段差异；这不是 5884 个独立问题。

翻转项 `active-h2-behind-sbeef-c5-t0`：同一第 6 章、法师开局、BEEF，A 在 wave25、1542.295 秒胜利且死亡0；B 在 wave6、316.254 秒失败且死亡3。首次**被记录到**的共同帧差异在 frame5400 / 两边 elapsed88.021，字段为 attacks 和 rng_state；不能把该采样点当作根因或精确最早分歧时刻。A 观察到英雄 ID 路径 0→7→6→2，B 仅 0，因此更不能把最终战斗状态视作开局角色的独立效果。

| 配置 | A/B 终态 | wave A/B | deaths A/B | 终态不同字段数 | 共同检查点不同/总数 | 首个共同差异帧 |
|---|---|---|---|---|---|---|
| `active-h0-behind-sa11ce-c4-t0` | win/win | 25/25 | 0/0 | 22 | 47/55 | 14400 |
| `active-h0-behind-sa11ce-c5-t0` | win/win | 25/25 | 0/0 | 20 | 40/55 | 27000 |
| `active-h0-behind-sbeef-c4-t0` | win/win | 25/25 | 0/0 | 20 | 28/54 | 46800 |
| `active-h0-behind-sbeef-c5-t0` | win/win | 25/25 | 0/0 | 21 | 46/55 | 16200 |
| `active-h1-behind-sa11ce-c4-t0` | win/win | 25/25 | 0/0 | 0 | 0/56 | — |
| `active-h1-behind-sa11ce-c5-t0` | win/win | 25/25 | 0/0 | 21 | 37/55 | 32400 |
| `active-h1-behind-sbeef-c4-t0` | win/win | 25/25 | 0/0 | 18 | 25/55 | 54000 |
| `active-h1-behind-sbeef-c5-t0` | win/win | 25/25 | 0/0 | 19 | 31/55 | 43200 |
| `active-h2-behind-sa11ce-c4-t0` | win/win | 25/25 | 0/0 | 20 | 33/55 | 39600 |
| `active-h2-behind-sa11ce-c5-t0` | loss/loss | 21/21 | 3/3 | 18 | 10/42 | 57600 |
| `active-h2-behind-sbeef-c4-t0` | win/win | 25/25 | 0/0 | 20 | 43/55 | 21600 |
| `active-h2-behind-sbeef-c5-t0` | win/loss | 25/6 | 0/3 | 29 | 8/11 | 5400 |
| `passive-h0-behind-sa11ce-c4-t0` | loss/loss | 2/2 | 3/3 | 0 | 0/4 | — |
| `passive-h0-behind-sa11ce-c5-t0` | loss/loss | 2/2 | 3/3 | 7 | 0/3 | — |
| `passive-h0-behind-sbeef-c4-t0` | loss/loss | 2/2 | 3/3 | 9 | 1/3 | 3600 |
| `passive-h0-behind-sbeef-c5-t0` | loss/loss | 2/2 | 3/3 | 0 | 0/4 | — |
| `passive-h1-behind-sa11ce-c4-t0` | loss/loss | 4/4 | 3/3 | 0 | 0/9 | — |
| `passive-h1-behind-sa11ce-c5-t0` | loss/loss | 4/4 | 3/3 | 0 | 0/8 | — |
| `passive-h1-behind-sbeef-c4-t0` | loss/loss | 4/4 | 3/3 | 1 | 1/9 | 14127 |
| `passive-h1-behind-sbeef-c5-t0` | loss/loss | 4/4 | 3/3 | 0 | 0/8 | — |
| `passive-h2-behind-sa11ce-c4-t0` | loss/loss | 4/4 | 3/3 | 0 | 0/9 | — |
| `passive-h2-behind-sa11ce-c5-t0` | loss/loss | 3/3 | 3/3 | 0 | 0/7 | — |
| `passive-h2-behind-sbeef-c4-t0` | loss/loss | 4/4 | 3/3 | 4 | 1/8 | 11959 |
| `passive-h2-behind-sbeef-c5-t0` | loss/loss | 4/4 | 3/3 | 0 | 0/8 | — |

每对完整终态 A/B 值及差值见 `matched-ab-terminal.csv`，缺失的各自终态帧也单列，未删掉较早结束的一侧。checkpoint 在 frame32、每1800次更新、终态或上限输出：所谓“30秒”是固定更新名义间隔，菜单暂停会使实际 elapsed 间距变化。比较按共同 frame 对齐，同时保留两侧实际 elapsed；没有插值或四舍五入掩盖差异。

## 成长观察

| replica/章/策略 | 末波分布 | 死亡分布 | 已学技能数范围 | 技能 rank 总和范围 | rank≥5 技能个数分布 | 首次观察 rank≥5 的 elapsed 范围 |
|---|---|---|---|---|---|---|
| a/5/active | 25:6 | 0:6 | 8–9 | 28–31 | 2:5, 3:1 | 617.863–1321.669s |
| a/5/passive | 2:2, 4:4 | 3:6 | 1–1 | 1–1 | 0:6 | 未观察到 |
| a/6/active | 21:1, 25:5 | 0:5, 3:1 | 7–9 | 24–30 | 1:4, 2:1, 3:1 | 471.174–1440.535s |
| a/6/passive | 2:2, 3:1, 4:3 | 3:6 | 1–1 | 1–1 | 0:6 | 未观察到 |
| b/5/active | 25:6 | 0:6 | 8–9 | 28–32 | 2:6 | 617.863–1321.669s |
| b/5/passive | 2:2, 4:4 | 3:6 | 1–1 | 1–1 | 0:6 | 未观察到 |
| b/6/active | 6:1, 21:1, 25:4 | 0:4, 3:2 | 6–9 | 9–31 | 0:1, 1:1, 2:4 | 471.174–1116.136s |
| b/6/passive | 2:2, 3:1, 4:3 | 3:6 | 1–1 | 1–1 | 0:6 | 未观察到 |

原 `skill_ranks` 字段是 32 元 rank 数组的**和**，并非已学技能数量。已学数量按 rank>0 另算，分布按数组逐项保存于 `case-metrics.csv`。冻结内容中 32 个基础技能定义 max_rank=5；本批 active 已学6–9个、rank总和9–32、rank≥5个数0–3；23/24个 active 执行观察到过任意一个 rank≥5。其首次被采样到的 elapsed 为471.174–1440.535秒。这个范围不是精确升级时刻，也不是“所有技能已经满级”。CSV 同时给此前 checkpoint 与首次观测 checkpoint 的闭区间；UR 分支/进化选择和当前可用槽上限未记录，不能由这几个字段补推。passive 全部只保留1个 rank1技能，且都在死亡3次后结束。

active 不是最优玩家：冻结策略按固定更新节奏选第0张牌、处理吸收和技能/羁绊，并按货架顺序尝试购买；passive 仍处理强制招募、基础自动攻击与技能。两种策略同时改变多项行为，不是单一技能、装备或开局职业的因果实验；也不同于当前公开 UI 的 cheapest-equipment / 自然材料 Forge 路线。

## 可据此安排的下一步

1. 先保留这批冻结输出作为描述性档案；在把相同 seed 用于改动 A/B 数值比较前，优先定位翻转配置最后一致的采样区间到 frame5400 的实际状态分歧。当前 trace 未记录完整 actor/contact，不能在本报告给出原因，也不启动额外模拟。
2. 后续若要评估“技能太容易满”，记录真实首次 rank5 事件时间、技能 ID、UR 演化分支和当时可用槽/已解锁池；不要把 rank总和当已学数。先区分该具体策略的观察与普通玩家体验。
3. 在重复口径可靠后，再扩大 seed 覆盖、分开真实积累升级与零升级夹具、保留英雄吸收路径，并与同版本公开 UI 证据交叉核对。两个 seed 的重复不能支撑当前版本职业增强/削弱、章节经济或总体胜率调参。

本轮不修改任何平衡数值、Rust、冻结 runner 或公开 UI 路线，不把 headless 胜负当作渲染手感验收。

## 原始证据

[完整审计及 CSV](../verification/gate-descriptive-analysis-2026-11-06/README.md) 包含两份原始24summary、全部48 JSONL和48 stderr、模拟夹具状态、原 manager 日志/实际退出、资产/runner hash、逐对 CSV 和全部1440 checkpoint曲线行。174MB simulator binary 的三份原路径/SHA 保留且已核验，不在 Git 再复制；没有删去失败 case 或覆盖原日志。可运行该目录 `verify.py` 独立重算配对结论。
