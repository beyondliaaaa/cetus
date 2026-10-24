# 项目状态

> 更新：2026-10-23 · 对应指南 v0.5.2

## 已完成

| 里程碑 | 状态 |
|---|---|
| M0 基础设施 | 完成 |
| M1 内核 v0（量纲、表达式、SD） | 完成（`dopri5`/`ros2` 推迟到 M1.5/M6；核心抽象 K5/K6/K8–K12 的接口推迟到首次使用它们的里程碑，见 ADR-0004） |
| M2 网页实验台 v0 | 完成（真机性能待补测，见「M2 已知限制」） |
| M3 引导探究与验收 V0–V4 | 未开始 |

## M2 交付

- **K12 视图契约**：`cetus-model::ViewKind`（13 种）与 `ParadigmId::views()`；
  SD / 离散映射声明 `series` 与 `phase`，前端 `views/registry.ts` 按 `views` 选择已实现的视图组件。
- **`cetus-wasm`（§5.13 子集）**：
  `compileModel`、`CompiledModel`（`metaJson` / `contentHash` / `start` / `restore` /
  `parseExpression` / `editModel`）、`Sim`（`statusJson` / `step` / `runUntil` /
  `setParam` / `addProbe` / `readProbe` / `query` / `snapshot` / `stateHash` /
  `tick` / `displayTime`）。元数据与诊断以 JSON 跨边界传递，探针以
  `Float64Array` 增量返回。
- **`@cetus/sim-client`**：完整的 TypeScript 契约与 Comlink Worker 封装；
  Worker 是唯一的 WASM 持有者，主线程从不直接调用内核（§10.2）。
- **实验台 UI**：
  - 8 个样板模型的模型选择；
  - 参数滑块 / 数值输入（显示声明单位，防抖提交）与固定参数折叠区；
  - 播放、暂停、单步、重置、调速（1×/10×/60×/300× 模型时间单位/现实秒）；
  - uPlot 时序图：按单位分组最多两条 y 轴、图例开关、2400 点降采样、
    ResizeObserver 与「最新值」无障碍表格；
  - 快照：内存保存/恢复、`.cetussnap` 下载/文件恢复，绑定内容哈希；
  - 诊断面板：编译诊断（码、路径、消息、帮助）与运行时错误；
  - xyflow 存量-流量画布：存量/辅助变量/系统外边界、流量箭头、信息连接、
    实时值（约 4 Hz）、鼠标与键盘选择、无障碍元素列表；
  - 方程编辑：ASCII 输入 + MathLive 公式输入（按需懒加载），实时
    `parseExpression` 量纲反馈、错误 caret 定位、编辑后经 `editModel` 重编译并
    重置仿真；失败时保留旧仿真；
  - i18next + ICU（zh-CN / en），所有面向玩家的文本走翻译键；
  - 手机竖屏优先的响应式布局（360–390px 无横向溢出，触控目标 ≥ 44px）。
- **内容**：SIR 的 `transmission` / `recovery` 标记为 `role: control`（原有的
  `range` 早已声明），使默认模型可直接调参；`source: param:TODO-expert` 保留，
  发布前仍需专家给出真实来源（P6 / V3）。

## M2 退出标准对照

| 退出标准（§12 M2） | 证据 |
|---|---|
| SIR 和太阳能板–蓄电池两个模型可以在网页中完整运行和编辑 | Playwright（Chromium）实测：加载、播放/暂停/单步/重置、调参、画布选择、方程编辑、重编译与重置均通过；SIR 编辑 `infection`，太阳能编辑 `pv_power` 与 `charge` 初值 `0.5 * capacity` |
| 编辑出错时有可读的提示 | 实时量纲反馈（例如 `m^2` vs `W`、`1/day` vs `person/day`）；解析错误带 caret；编译错误（如 E3101 代数环）同时出现在编辑器内联与全局诊断面板 |
| 10 万步运行时 UI 不卡顿 | 仿真全部在 Worker；wasm 版 SIR（dt=0.001）连续 100,000 个 RK4 宏步耗时约 **31 ms**，读取 100,000 对探针约 **1.5 ms**；UI 状态更新节流到约 15 Hz，曲线渲染前降采样到 2400 点 |

## M2 已知限制

1. **§5.13 未实现项**：`runAnalysis`、`enqueueAction`、`exprEquiv`、
   `checkDiscrimination` 尚未实现，按计划随 M3（分析层/判别实验）与 M8（推导）
   落地。
2. **编辑语义**：方程编辑通过 `editModel` 重新序列化 YAML，会丢失 YAML 注释；
   编辑成功后仿真重置（内容哈希变化，旧快照同时失效）。
3. **MathLive 子集**：只接受算术、`sqrt`、常用函数与 `\pi`；求和、积分、矩阵、
   下标、赋值会给出结构化错误，不会静默丢字符。
4. **图表**：最多两条带标签的 y 轴；超过两个单位分组时其余系列仍绘制但没有
   独立轴标签。
5. **主题**：当前为深色仪器面板；浅色主题尚未按 §10.4 设计与验证。
6. **真机性能**：只在桌面 Chromium 与 360–390px 视口模拟上验证；尚未在一台
   真实中端安卓机上跑首屏与帧率测试（§10.5）。
7. **WASM 体积**：核心包 gzip 约 **290 KB**（§5.13 预算 800 KB）；M2 引入的
   JSON 序列化是主要增量，仍有充足余量。
8. **M1 遗留**：仅 `euler`/`rk4`；`delay1`/`delay3`/`smooth` 求值为 0；
   不变量无容差；YAML 字段级行号仍待 serde span 支持；`content/` 仍暂驻引擎仓库。

## M1 退出标准对照

| 退出标准（§12 M1） | 证据 |
|---|---|
| 8 个样板模型全部通过测试，有解析解的 nrmse < 1e-4 | `crates/cetus-sd/tests/sample_models.rs`：9 项测试通过；decay 1.6e-12、logistic 1.3e-10、RC 1.3e-11；SIR 守恒、Lotka–Volterra 不变量漂移 0、logistic 映射逐位对照 |
| `sweep` 以 `Dataset` 输出 CSV | `cetus-kernel::Dataset`（Time / Parameter / Ensemble 轴）；`cetus sweep` 实测 4×3 网格输出正确 |
| 未来范式 schema：解析/名称/类型推导通过，求值报 E1901 | `crates/cetus-compile` 的 `complex_model_typechecks_then_reports_e1901`；fixture 在 `content/fixtures/complex-fixture.yaml` |
| 每个样板模型在 native 与 WASM 下状态哈希相同 | `./scripts/check-cross-target.sh`：8 个模型全部 OK，并与 `golden/hashes.txt` 一致 |
| 快照往返（proptest，1000 例） | `crates/cetus-sd/tests/snapshot.rs`，1000 例通过 |
| 故意写错的单位得到 E1xxx 且定位准确 | `rejects_a_flow_with_the_wrong_dimension` 断言 `E1203` 且 `path == flows.charging.unit` |

### 跨目标哈希

唯一的真相源是 [`crates/cetus-testkit/golden/hashes.txt`](../crates/cetus-testkit/golden/hashes.txt)
（hello 指纹 + 8 个样板模型，`--seed 1`）。本页不再抄录数值，避免两处不同步。
CI 检查 native、wasm 与该文件三者一致（ADR-0001「黄金哈希」）。

### 规模

- Rust 测试：**89** 个通过（含 proptest）；
- 前端测试：**43** 个通过（Vitest，7 个纯函数测试文件）；
- wasm 包：739 KB，gzip **290 KB**（§5.13 预算 800 KB）；
- 前端构建：主 JS 644 KB / gzip 212 KB，CSS 44 KB / gzip 8.5 KB；
  MathLive 及其 CSS 是独立的按需 chunk（JS 823 KB / gzip 229 KB），只在用户展开
  公式输入时加载。

## 下一步：M3 引导探究与验收 V0–V4（§12 M3）

1. `cetus-lesson`：`cetus.lesson/0.1` schema、仪器与观测生成、检查器、误解补丁；
2. `packages/lesson-runtime`：任务状态机、提示阶梯、反例驱动提示；
3. 实验台 UI：仪器面板、观测数据表、预测/对照图、任务列表；
4. `cetus-verify`：V0–V4 与 Markdown/JSON 报告；
5. 分析层 `ensemble` / `fit`，定稿 K9 `Decision`；
6. 表示层 `layers`、L0/L1 检查器、因果箭头画布；
7. 教学任务与 `checkDiscrimination` WASM 接口、判别实验台；
8. 手写 3 节课并通过 V4。

## 复现验证

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/check-clippy-rules.sh      # clippy.toml 的 60 条 std 规则确实命中
./scripts/check-no-os-rng.sh         # cetus-wasm / cetus-cli 的产物依赖不含 getrandom
./scripts/check-cross-target.sh      # native = wasm = golden/hashes.txt
npm ci && npm run lockfile:check && npm run build:wasm
npm run typecheck && npm test && npm run build --workspace @cetus/web
```

浏览器验收（开发服务器）：

```bash
npm run dev --workspace @cetus/web
# 默认打开 SIR；切换到「太阳能板与蓄电池」；播放/暂停/单步/重置；
# 拖动参数；点击画布节点/边编辑方程；应用后确认仿真重置。
```
