# 项目状态

> 更新：2026-10-23 · 对应指南 v0.5.2

## 已完成

| 里程碑 | 状态 |
|---|---|
| M0 基础设施 | 完成 |
| M1 内核 v0（量纲、表达式、SD） | 完成（`dopri5`/`ros2` 推迟到 M1.5/M6；核心抽象 K5/K6/K8–K12 的接口推迟到首次使用它们的里程碑，见 ADR-0004） |
| M2 网页实验台 | 未开始 |

## M1 退出标准对照

| 退出标准（§12 M1） | 证据 |
|---|---|
| 8 个样板模型全部通过测试，有解析解的 nrmse < 1e-4 | `crates/cetus-sd/tests/sample_models.rs`：9 项测试通过；decay 1.6e-12、logistic 1.3e-10、RC 1.3e-11；SIR 守恒、Lotka–Volterra 不变量漂移 0、logistic 映射逐位对照 |
| `sweep` 以 `Dataset` 输出 CSV | `cetus-kernel::Dataset`（Time / Parameter / Ensemble 轴）；`cetus sweep` 实测 4×3 网格输出正确 |
| 未来范式 schema：解析/名称/类型推导通过，求值报 E1901 | `crates/cetus-compile` 的 `complex_model_typechecks_then_reports_e1901`；fixture 在 `content/fixtures/complex-fixture.yaml` |
| 每个样板模型在 native 与 WASM 下状态哈希相同 | `./scripts/check-cross-target.sh`：8 个模型全部 OK，并与 `golden/hashes.txt` 一致（见下） |
| 快照往返（proptest，1000 例） | `crates/cetus-sd/tests/snapshot.rs`，1000 例通过，约 4.3 s |
| 故意写错的单位得到 E1xxx 且定位准确 | `rejects_a_flow_with_the_wrong_dimension` 断言 `E1203` 且 `path == flows.charging.unit` |

### 跨目标哈希

唯一的真相源是 [`crates/cetus-testkit/golden/hashes.txt`](../crates/cetus-testkit/golden/hashes.txt)
（hello 指纹 + 8 个样板模型，`--seed 1`）。本页不再抄录数值，避免两处不同步。
CI 检查 native、wasm 与该文件三者一致（ADR-0001「黄金哈希」）。

### 规模

- Rust 测试：78 个通过（含 proptest）；
- wasm 包：570 KB（gzip 213 KB），在 §5.13 的 800 KB 预算内；
- 前端首屏构建：JS 230 KB / gzip 73 KB，CSS 3.4 KB / gzip 1.4 KB。

## M1 的已知限制

1. **积分器**：只实现 `euler` 与 `rk4`。模型声明 `dopri5` 或 `ros2` 时，`cetus run`
   会明确报错（不静默降级）。
2. **状态函数**：`delay1`/`delay3`/`smooth` 能解析与类型检查，但 SD 子系统还没有
   为它们分配状态槽，求值时返回 0。`lookup` 已可用。
3. **不变量**：`assert`/`bounds` 的语义是“表达式为 0 即违反”，没有容差。
   `conserved_sum` 等 kind 尚未实现；SIR 的守恒改由测试检查。
4. **离散模型**：`time.kind: discrete` 的 `euler` 语义已实现（flow 是每步增量），
   但 `dt ≠ 1` 的组合尚未测试。
5. **诊断源码定位**：表达式内部用字节 span；YAML 字段用 `path`（如
   `flows.charging.unit`）。字段级行号需要 serde_yaml_ng 的 span 支持，M2 再做。
6. **内容仓库**：`content/` 暂驻引擎仓库，M5 拆分到 `cetus-content`。
7. **核心抽象接口**：K1/K2/K4/K7/K13/K14 已落地，K3（`Analysis`）、K5、K6、K8–K12
   仍是占位；它们的定稿时间见 ADR-0004「实现状态」。

## 下一步：M2 网页实验台 v0（§12 M2）

0. 先定稿 K12 `ViewKind`（ADR-0004），再实现前端视图注册表；
1. `cetus-wasm` 的 `compile` / `Sim` / `setParam` / `readProbes` / `snapshot` 接口，
   以及 `packages/sim-client` 的 TS 类型；
2. uPlot 时序图、参数滑块（显示单位）、播放/暂停/步进/调速；
3. xyflow 模型画布（存量-流量图），公式编辑 + 实时 `parseExpr` 量纲反馈；
4. MathLive 公式输入与 ASCII 输入；
5. i18n 正式接入（i18next + ICU），替换当前的玩具加载器；
6. 响应式布局与中端安卓机性能测试；
7. 快照保存/加载（内存 + 文件下载）。

## 复现验证

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/check-clippy-rules.sh      # clippy.toml 的 60 条 std 规则确实命中
./scripts/check-no-os-rng.sh         # cetus-wasm / cetus-cli 的产物依赖不含 getrandom
./scripts/check-cross-target.sh      # native = wasm = golden/hashes.txt
npm ci && npm run lockfile:check && npm run build:wasm
npm run typecheck && npm test && npm run build --workspace @cetus/web
```
