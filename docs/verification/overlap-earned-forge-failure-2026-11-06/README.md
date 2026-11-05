# 新 overlap 包自然合成：严格界面分类中断

本次真实公开界面路线**未完成**。Forge 进程退出 1，400.590 秒；管理任务传播退出 1，401.400 秒，没有把 wrapper 成功当作游戏或路线通过。尚未购买材料、没有合成，条件重启未启动。[结果](result.json)、[进程](process.json)、[管理进程](manager-process.json)、[原始 traceback](runtime.log) 保留。

## 固定输入与包

独立不可变包 `f5ef6ff8f37ecab467b5ba0baadb6decd8d49aaf`，二进制 SHA-256 `76a6da42c0c9f49289fe77b0caea02ec7a425b793930220a43de2219b0ae72cc`，886 个清单文件逐项通过校验。断网只读包/四份冻结助手，运行镜像 ID 固定 `sha256:0d4cccee982b31492e0f7b01a50afc1a0ec710d3166dab22e02322ce64832715`。[包 pin](reviewed-package-pins.json)、[实际命令](command.json)、[冻结助手](harness-snapshot.json) 保留。

初始独立 profile 只复制真实 canonical campaign.save，SHA-256 `da5bf0a9ca3f4032a265425f9989bda292d8c0c2622848157df6d2c0223ba825`；无配置、音量、库存、战功、材料或 ECS 注入。结束后的 [实际存档](actual-campaign.save) 哈希不变，只有游戏实际写出的教程状态增加。[真实 profile 文件与哈希](audit/profile.json)。未触碰其他 C6 容器或 profile。

## 实际失败画面

[market-02-open-04.png](market-02-open-04.png) 是实际活跃战斗：画面显示 LIVES 3、LEVEL 7、6/168 XP 与 20:12，无市场、招募、设置或结算遮罩。前次 [市场打开](market-01-open-03.png) 正常；[current-ui.png](current-ui.png) 是失败图之前约 14 秒的另一张战斗图，不冒充失败之后的截图。

原始完整 OCR 将 `LIVES3` 读为 `LVES3`，将 `LEVEL7` 读为 `LEVELT`。冻结分类器的直接战斗分支要求数字，兜底候选只允许数字或 S，因此没有请求已有严格数字像素读取器，分类返回 unknown 并严格停止。这不是已证实的游戏功能故障。

原 `panel_open_attempts` 的失败记录为 requests=0，无 input 字段；该次在未知画面分类后未发送打开市场的 B 键。此前成功记录中才有 B 输入。原助手没有逐键绝对时刻日志，故不能补造准确的 keydown/up 时间；[文件时间](audit/file-times.json) 只给截图与结果写入的观察界限。

## 只读离线诊断

使用同一固定镜像、原失败 PNG 和冻结助手：[离线脚本](audit/offline.py)、[完整 OCR 与结果](audit/ocr.json)。原始 OCR 和重新完整 OCR 均分类 unknown。单独执行既有严格 `battle_level_pixels` 得到 `LEVEL76/168XP`，符合原严格数字规则：该读取器原本允许 level 与当前 XP 的数字贴连，此结果只证明 HUD 数字签名，不能据此猜 level=76 或替换 T→7。

最小建议是只扩大固定 HUD 位置的 LEVEL…XP **重读候选**，仍由原严格数字像素 fullmatch 决定是否为战斗；不替换 OCR 字符、不猜等级、不改变已知遮罩优先级、不把所有 unknown 当 battle。此归档不宣称分类器已修复或新路线已通过。

[重启未启动证明](restart-not-started.json)：独立目录只有预先冻结的依赖与 snapshot，没有 command/runtime/process；管理脚本在 Forge 退出 1 时即停止，没有挂载 profile 启动重启。旧 86 包通过证据没有混入此次结果。
