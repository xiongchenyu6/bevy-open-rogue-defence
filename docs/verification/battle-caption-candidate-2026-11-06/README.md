# 模糊 LEVEL 字形：先候选，再严格数字像素校验

[原 overlap Forge 失败](../overlap-earned-forge-failure-2026-11-06/README.md) 已独立提交并保持未完成：实际 battle 被完整 OCR 读成 `LVES3` 与 `LEVELT 6/168 xP`，旧分类器返回 unknown，停止前没有发送该次市场打开输入。没有修改原结果或重跑游戏。

## 最小改动

只扩大固定 HUD 区域 `450≤x<650, 640≤y<695` 内、起始为 LEVEL 且有正确 XP 结尾边界的“请求重读候选”：LEVEL 后 OCR token 从数字/S 放宽为字母和数字。该候选本身不能判定 battle，仍调用已有 `battle_level_pixels` 的原严格数字 fullmatch。已知终态、选牌、背包、市场、设置等遮罩的优先顺序不变；严格 crop、数字规则和其他助手功能不变。

没有 T→7、LVES→LIVES 字符替换，没有猜测等级，也没有把任意 unknown 当 battle。原严格像素读取器对失败图返回 `LEVEL76/168XP`，是既有规则允许的“等级与当前 XP 数字贴连”签名；不把它解释为等级 76 或等级 7。

## 实际离线前后与负例

在真实失败 PNG 上，原冻结助手：完整 OCR unknown，单独严格数字 crop 可读。先在内存中验证候选，再修改生产助手源码；全程只读 PNG，没有启动游戏。

| 画面/条件 | 最终结果 |
| --- | --- |
| 原失败 PNG，不提供像素重读 | unknown |
| 原失败 PNG，旧候选 | unknown |
| 原失败 PNG，新候选 + 原严格数字 crop | battle |
| 真实原生市场 | market，未调用 battle 像素 reader |
| 真实原生空背包 | inventory，未调用 reader |
| 真实等级 10 招募 | choice，未调用 reader |
| 真实英语 Settings | unknown，未调用 reader |
| 真实 NO LIVES REMAIN | terminal，未调用 reader |
| 模糊候选但严格数字 crop 失败 | 仍拒绝/抛出校验失败 |
| 错误坐标、前缀或 XPjunk/XPP/noXP 边界 | unknown，不调用 reader |

原失败图及 raw rows：[原始证据](../overlap-earned-forge-failure-2026-11-06/market-02-open-04.png)、[离线输出](../overlap-earned-forge-failure-2026-11-06/audit/ocr.json)。新测试直接引用这些已提交图片和五个已有原生负例，不复制大图。

## 验证结果与边界

修改源码前，隔离固定镜像中的内存候选对原有完整 **93 项测试通过**，153.502 秒：[日志](offline-before-source-change/candidate-holdouts.log) 与 [返回码/命令](offline-before-source-change/candidate-holdouts-process.json)。该脚本仅是原始诊断记录，不是运行时兼容层。

最终源码的完整 Docker 默认 discover **97 项通过、无跳过**，110.594 秒：[日志](tests.log)、[返回码 0 与完整命令](result.json)。使用镜像 ID `sha256:0d4cccee982b31492e0f7b01a50afc1a0ec710d3166dab22e02322ce64832715`，只读 repo 挂载。[源码哈希](source-hashes.json)。

`python3 docs/verification/battle-caption-candidate-2026-11-06/verify.py` 仅检查归档结果，不启动游戏。

本轮证明离线分类候选和已有原生负例的行为，不证明新的自然 Forge 或重启路线已完成。没有游戏源码、Cargo、购买扣款逻辑或实际 profile 的修改，也没有重跑 Forge。后续路线还需单独真实运行验收。
