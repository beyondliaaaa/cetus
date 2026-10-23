# 项目状态

> 更新：2026-10-23 · 对应指南 v0.5.2

## 已完成

| 里程碑 | 状态 |
|---|---|
| M0 基础设施 | 完成 |
| M1 内核 v0（量纲、表达式、SD） | 完成（`dopri5`/`ros2` 推迟到 M1.5/M6） |
| M2 网页实验台 | 未开始 |

## M1 退出标准对照

| 退出标准（§12 M1） | 证据 |
|---|---|
| 8 个样板模型全部通过测试，有解析解的 nrmse < 1e-4 | `crates/cetus-sd/tests/sample_models.rs`：9 项测试通过；decay 1.6e-12、logistic 1.3e-10、RC 1.3e-11；SIR 守恒、Lotka–Volterra 不变量漂移 0、logistic 映射逐位对照 |
| `sweep` 以 `Dataset` 输出 CSV | `cetus-kernel::Dataset`（Time / Parameter / Ensemble 轴）；`cetus sweep` 实测 4×3 网格输出正确 |
| 未来范式 schema：解析/名称/类型推导通过，求值报 E1901 | `crates/cetus-compile` 的 `complex_model_typechecks_then_reports_e1901`；fixture 在 `content/fixtures/complex-fixture.yaml` |
| 每个样板模型在 native 与 WASM 下状态哈希相同 | `./scripts/check-cross-target.sh`：8 个模型全部 OK（见下） |
| 快照往返（proptest，1000 例） | `crates/cetus-sd/tests/snapshot.rs`，1000 例通过，约 4.3 s |
| 故意写错的单位得到 E1xxx 且定位准确 | `rejects_a_flow_with_the_wrong_dimension` 断言 `E1203` 且 `path == flows.charging.unit` |

### 跨目标哈希（2026-10-23）

```text
epidemic-sir          7b07b5374ea322c7139ebbbf69d979301cd1753a5dbdf62e61903c7cf2d594df
population-logistic   1739d2070d444fd01493ab0a6f310d12577f553de0bf3f214ddaa99dcfa57f7a
predator-prey         3f4788049c2a2ea259f2bb578fe9e3b52748d2938a261b1541f0775792850655
solar-battery         b897679e5ad4b1960fc5f1b40b94c884e2a4b106e00ef471950946c42c57aded
inventory-oscillation 4323626c8f52dc1af4811c6b9744c73d44d55d58950befee6bbed2fd72142aae
decay                 a89ac686532106ffef53c261958202748b1c4fa2e2bf5818a2ac09e5c76fb406
logistic-map          f09a82850098fe3c992eb730763af78a00e60885ebfba21806a471794c7f6c13
rc-circuit            d4918cb10516e21f3dad4d84042da346695fbb7b5106926b2d221897cc1d806e
```

hello digest（M0 witness）：`dcc2a51fe7fb09bc332796b79f3ad92abe892fe8bcf2a44480d777900cd63fd7`

以上哈希已锁定在 `crates/cetus-testkit/golden/hashes.txt`，CI 同时检查
native 与 wasm 一致、以及二者与黄金值一致（ADR-0001「黄金哈希」）。

### 规模

- Rust 测试：77 个通过（含 proptest）；
- wasm 包：570 KB（gzip 214 KB），在 §5.13 的 800 KB 预算内；
- 前端首屏构建：JS 230 KB / gzip 73 KB，CSS 3.4 KB / gzip 1.4 KB。

## M1 的已知限制

1. **积分器**：只实现 `euler` 与 `rk4`。模型声明 `dopri5` 或 `ros2` 时，`cetus run`
   会明确报错（不静默降级）。
2. **状态函数**：`delay1`/`delay3`/`smooth` 能解析与类型检查，但 SD 子系统还没有
   为它们分配状态槽，求值时返回 0。`lookup` 已可用。
3. **不变量**：`assert`/`bounds` 的语义是“表达式为 0 即违反”，没有容差。
   `conserved_sum` 等 kind 尚未实现；SIR 的守恒改由测试检查。
4. **离散模型**：`time.kind: discrete` 的 `euler` 语义已实现（flow 是每步增量），
   但 `regex`/`dt ≠ 1` 的组合尚未测试。
5. **诊断源码定位**：表达式内部用字节 span；YAML 字段用 `path`（如
   `flows.charging.unit`）。字段级行号需要 serde_yaml_ng 的 span 支持，M2 再做。
6. **内容仓库**：`content/` 暂驻引擎仓库，M5 拆分到 `cetus-content`。

## 下一步：M2 网页实验台 v0（§12 M2）

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
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./scripts/check-clippy-rules.sh      # clippy.toml 的确定性规则确实命中
cargo fmt --all -- --check
./scripts/check-cross-target.sh      # native = wasm = golden/hashes.txt
npm install && npm run build:wasm && npm run typecheck && npm run build --workspace @cetus/web
```
