# 模拟器独立素材输入

只给开发模拟器增加可选 `ROGUE_SIM_ASSET_ROOT`；未设置时仍使用原项目素材路径。启动时检查目录存在，输出实际使用路径；不是目录则在创建游戏前明确拒绝。没有修改游戏逻辑、数值、玩家设置或增加依赖。

用于后续描述性平衡采样：复制全部 185 个已追踪素材文件（含 52 个原模型），逐字节核对后只读保存。未接受的魔窟候选不在快照中。各次模拟可使用同一独立素材目录，不受开发目录中的新美术迭代影响。[素材 SHA 清单](asset-manifest.json) 记录可从 Git 重建的原始版本与每个文件；本归档不重复存储 26 MB 素材或开发 ELF。

## 实际检查

- [模拟器现有单元测试](bin-tests.log)：6 通过，0 失败。
- [严格全目标 Clippy](clippy.log)及[优化构建](build.log)退出 0。
- [前](source-before.json)／[后](source-after.json)全部游戏 Rust 与 Cargo 文件哈希一致，二进制 SHA 见[binary.json](binary.json)。城门逻辑的独立 687 项库测试属于[上一源码检查](../gate-pass-state-2026-11-06/README.md)，不冒充本次重复执行全库。
- [真实固定目录启动](snapshot-root-smoke.log)：使用全部 185 个字节一致素材，正常完成 1,200 次更新；最后记录游戏时间 19.617 秒，未暂停、未结束，无 ERROR 或 panic。这是有限启动检查，不能算通关。
- [无效目录](missing-root-rejected.log)：明确指定不存在路径，预期退出 101，错误文本与路径校验一致；没有进入游戏。不是生产崩溃通过。
- 初次误用了项目未声明的 `serde_json`，编译退出 101，保留在[初次失败](initial-compile-failure/)。随后改用普通日志输出；未加入新依赖，最终全部阶段通过。

`python3 docs/verification/sim-asset-root-2026-11-06/verify.py` 检查归档哈希与实际阶段结果，不启动游戏。`run.py` 保留本机原始执行命令。使用示例：

```sh
ROGUE_SIM_ASSET_ROOT=/absolute/frozen/assets \
  target/release/balance_sim active A11CE 2 behind 120000
```

这是 Nix 开发环境中的 headless 生产 GamePlugin 模拟器，不是独立 SDK 发行包、正常存档解锁流程、硬件性能或真实玩家体验验收。精确逐帧重复门槛仍独立开放；后续多种子与重复样本只作描述性结果，不据此宣称因果调参成立。
