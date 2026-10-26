# 项目状态

> 更新：2026-10-24 · 对应指南 v0.5.2

## 已完成

| 里程碑 | 状态 |
|---|---|
| M0 基础设施 | 完成 |
| M1 内核 v0（量纲、表达式、SD） | 完成（`dopri5`/`ros2` 推迟到 M1.5/M6；核心抽象 K5/K6/K8–K12 的接口推迟到首次使用它们的里程碑，见 ADR-0004） |
| M2 网页实验台 v0 | 完成（真机性能待补测，见「M2 已知限制」） |
| M3 引导探究与验收 V0–V4 | 进行中（切片 1 完成：lesson schema + verify V0/V1 + CLI） |

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
| SIR 和太阳能板–蓄电池两个模型可以在网页中完整运行和编辑 | `apps/web/e2e/workbench.e2e.ts`（Playwright，CI 的 web 任务）：两个模型在浏览器中跑到终点，界面显示的状态哈希**等于黄金值**；播放/暂停/单步/重置；调参改变结果、恢复默认后回到黄金值；快照经内存与下载文件恢复；`pv_power` 编辑与 `charge` 初值 `0.5 * capacity`。另有 2026-10-24 人工检查通过 |
| 编辑出错时有可读的提示 | 同上：单位不匹配与解析错误给出说明并禁止应用；公式输入拒绝无底数的 `\log` 并提示 `\lg`/`\ln`。所有提示文案经 ICU 解析测试（`src/i18n/messages.test.ts`） |
| 10 万步运行时 UI 不卡顿 | 仿真全部在 Worker（主线程不调用 wasm）。吞吐量：`scripts/bench-wasm.cjs`（CI 的 determinism 任务），SIR dt=0.001、全部观测量带探针，100,000 个 RK4 宏步加读取探针，预算 1.5 s（本地实测约 31 ms，预算按 CI 共享机器放宽）。UI 状态更新节流到约 15 Hz，曲线渲染前降采样到 2400 点 |
| （确定性：分段推进） | 浏览器按 `step(n)`/`runUntil(t)` 分段推进，结果与一次跑完逐位相同：`cetus-wasm` 测试（分段 1/7/1000 与交替模式）与 `check-cross-target.sh` 的 wasm `step(997)` 路径，均对照黄金值 |

## M2 已知限制

1. **§5.13 未实现项**：`runAnalysis`、`enqueueAction`、`exprEquiv`、
   `checkDiscrimination` 尚未实现，按计划随 M3（分析层/判别实验）与 M8（推导）
   落地。
2. **编辑语义**：方程编辑通过 `editModel` 重新序列化 YAML，会丢失 YAML 注释；
   编辑成功后仿真重置（内容哈希变化，旧快照同时失效）。
3. **MathLive 子集**：只接受算术、`sqrt`、常用函数与 `\pi`；求和、积分、矩阵、
   下标、赋值会给出结构化错误，不会静默丢字符。对数必须写明底数：`\lg` 或
   `\log_{10}` → `log10`，`\ln` 或 `\log_e` → `ln`；无底数的 `\log` 报错（内核本身也
   没有含义模糊的 `log`）。`e^{x}` 中的 `e` 当作普通标识符，不会转换成 `exp`。
4. **图表**：最多两条带标签的 y 轴；超过两个单位分组时其余系列仍绘制但没有
   独立轴标签。
5. **主题**：当前为深色仪器面板；浅色主题尚未按 §10.4 设计与验证。
6. **真机性能**：只在桌面 Chromium 与 360–390px 视口模拟上验证；尚未在一台
   真实中端安卓机上跑首屏与帧率测试（§10.5）。
7. **WASM 体积**：核心包 gzip 约 **290 KB**（§5.13 预算 800 KB）；M2 引入的
   JSON 序列化是主要增量，仍有充足余量。
8. **浏览器解析 YAML**：实验台在浏览器中解析模型 YAML（含 `modelSource.ts` 的
   正则读取），这是 ADR-0002 的有期限例外，至 M5，见 ADR-0005。
9. **M1 遗留**：仅 `euler`/`rk4`；`delay1`/`delay3`/`smooth` 在编译期报
   E1901（延迟状态槽待 M4，不再静默求值为 0）；不变量无容差；YAML 字段级行号仍待
   serde span 支持；`content/` 仍暂驻引擎仓库。

## M3 切片 1 交付（2026-10-24）

- **`cetus-lesson`**：`cetus.lesson/0.1` 严格 schema（未知字段拒绝）；模型路径补丁
  （`parameters.*.value`、`stocks.*.initial`、`auxiliaries.*.eq`、`flows.*.eq`、
  `observables.*.eq`）；真模型覆盖；V1 静态检查（`E2101` / `E5203` / `E5401`），
  覆盖洞目标、误解补丁、L0–L3、教学任务的 `holds` / `setup_space` / 变式。
- **`cetus-verify`**：累积等级 `V0` → `V1`；模块与课程两种目标；Markdown 与 JSON
  报告；等级未实现时明确拒绝（不伪造通过）。
- **`cetus verify <path> --level V0|V1 --report md|json`**：正例退出 0，反例退出 1，
  未实现等级退出 2。
- **课程 fixture**：`content/lessons/energy.battery-night/`（正例）与
  `energy.battery-night-broken/`（E5203 + E5401 反例）。
- **诊断页**：`docs/errors/E5203.md`、`docs/errors/E5401.md`。
- **测试**：`cetus-lesson` 42、`cetus-verify` 11、`cetus-cli` 5。

### M3 切片 1 已知限制

1. **V1 的内容库引用尚未解析**：模块编译只检查 `fixed` 参数是否声明了 `source`，
   尚未到 `content/params/`、`content/sources/` 中核对来源是否存在；误解
   `source.ref` 的 `src:<id>` / `mc:<id>` 也只校验格式，不检查 id 是否存在（指南 §8.2）。
2. **`cetus verify` 只实现 V0/V1**：V2–V4（扫描、步长收敛、确定性、行为测试、
   变异杀伤率、判别实验、噪声稳健）尚未实现。
3. **课程 schema 尚无运行时**：`layers` 与 `teaching` 可解析并通过 V1，但检查器
   求值、观测生成、`lesson-runtime`、实验台 UI 尚未实现。
4. **容差单位换算未实现**：`RelTol` / `ParamWithin.rel_tol` 现在只接受无量纲数字
   （0 < tol < 1）；检查器求值时把相对容差换算到模型单位、以及 `AbsTol` 的量纲解析，
   尚未实现。
5. **V1 不校验仪器时间与噪声参数**：`instruments[].every`、
   `setup_space.duration.max` 的单位，以及 `noise.sigma` / `resolution` 的正值，
   留到观测生成切片一并实现。
6. **`truth.dataset` 只做 schema 接受**：尚未校验 `content/datasets/` 中的条目、
   来源、许可证与预处理脚本；数据集真值是 V3 数据集完整性检查的范围。
7. **K13 尺度推导未实现**：`EquivOptions.scale` 需要调用方从目标变量的 `scale:`
   取值；未提供时使用纯相对误差。`SlotInfo.scale` 已改为 `Option<f64>`，不再默认
   `1.0`；从 `initial`/`range` 推断尺度仍待 M1.5。
8. **`must_fail` 尚未经 V4 验证**：battery-night 修订后的 `must_fail`
   （`m_stock_follows_inflow` → `t4`/`t5`、`m_battery_constant_discharge` → `t4`、
   `m_load_irrelevant` → `l1-1`）都是按模型推算的，V4 的变异与区分力检查尚未实现；
   V4 落地后应把 battery-night 作为第一门复核课程。

### M3 教学 schema 修订（进行中）

- **误解来源（PR1）**：`misconceptions[]` 与 `layers.*.misconceptions[]` 新增必填
  `source: { kind, ref, note? }`；`research_documented` / `exam_item` /
  `historical_theory` / `folk_belief` 必须给出 `src:<id>` 或 `mc:<id>`（E5204），
  `hypothesized` 可省略 `ref` 但产生 W5201；`teaching[].holds` 引用 hypothesized
  条目时产生 W5202。V6 的专家签字规则已写入指南 §8.7，暂不自动检查。
- battery-night 已按调研修订：删除 `m_square`（改由 V4 变异测试覆盖 `t2`）；
  `m_stock_follows_inflow` 与拆分的 `m_battery_constant_discharge` /
  `m_load_irrelevant` 引用误解目录（`mc:`）；`m_constant_sun` 保留
  `hypothesized`（对应 `mc:eng.energy.average-sun`）。
- **提示阶梯（PR2）**：`holes[].hints` 改为对象 `{ q0?, h1, h2, h3 }`；`q0` 省略时由
  检查器按 §6.4 生成，`h1`–`h3` 必填；V1 用 E0003 校验 key 非空。
- **容差（PR3）**：`Check::RelTol` 改为 `tol: f64`；`RelTol` 与
  `ParamWithin.rel_tol` 由 V1 校验有限且 `0 < tol < 1`（E0003）。
- **任务读取仪器量（PR5）**：battery-night 的 `t4` 从 `first_time(charge <= 0)` 改为
  `first_time(charge_pct <= 0)`；判别类任务读仪器量，不读内部存量。规则写入指南
  §6.3 与 M5 的 `design-misconceptions` 要求，并在 `research-misconceptions` 中记为
  M8。
- **来源引用（PR6）**：`source.ref` 允许 `src:<id>` 或 `mc:<id>`；`mc:` 指向误解目录
  条目，文献证据在目录里维护一次，课程只引用目录。V1 仍只校验格式。

### M3 评审修复（2026-10-24）

- **`delay1` / `delay3` / `smooth` 不再静默求值为 0**：编译期报 `E1901`；
  `E3101` 的帮助文本与 `docs/errors/E3101.md` 不再推荐这些尚未实现的函数。
- **`expr_equiv` 采样修正**：
  - `time` 必须通过 `EquivOptions::time` 给出范围并逐点采样，不再固定为 0；
  - `dt` 必须通过 `EquivOptions::dt` 给出值；
  - 未覆盖的变量返回 `IncompleteDomain`，不再按 0 代入；
  - `lookup` 必须提供表数据（`TableLookup` / `equivalent_with_tables`），
    缺失时返回 `Undefined`，不再按 0 代入；
  - 相对误差分母改为 `max(|l|, |r|, scale)`，`scale` 来自输出量的 K13 量级；
    未声明时退化为纯相对误差，去掉固定的 `1.0` 下限；
  - 有状态函数在 `equivalent` 中直接拒绝比较。
- 新增 9 个等价性回归测试与 2 个编译期回归测试；`cetus-expr` 的等价性测试从
  3 个增至 12 个。

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

- Rust 测试：**97** 个通过（含 proptest）；
- 前端测试：**51** 个通过（Vitest，8 个测试文件）；
- 端到端测试：**12** 个通过（Playwright，生产构建，CI 的 web 任务）；
- wasm 包：758 KB，gzip **290 KB**（§5.13 预算 800 KB）；
- 前端构建：主 JS 646 KB / gzip 213 KB，CSS 44 KB / gzip 8.5 KB；
  MathLive 及其 CSS 是独立的按需 chunk（JS 823 KB / gzip 229 KB），只在用户展开
  公式输入时加载。

## 下一步：M3 剩余工作（§12 M3）

切片 1（schema + V0/V1 + CLI）已完成；剩余：

1. `cetus-lesson`：仪器与观测生成（噪声子流）、洞的应用、检查器
   （`expr_equiv`、`param_within`、`abs_tol`、`trajectory_fit`、`structure`）；
2. `packages/lesson-runtime`：任务状态机、提示阶梯、反例驱动提示；
3. 实验台 UI：仪器面板、观测数据表、预测/对照图、任务列表；
4. `cetus-verify`：V2（扫描、步长收敛、确定性）、V3（行为测试）、V4（可解性、
   区分力、变异测试、泄露检查、噪声稳健）；
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
cargo run -p cetus-cli -- verify content/lessons/energy.battery-night/lesson.yaml --level V1 --report md
cargo run -p cetus-cli -- verify content/lessons/energy.battery-night-broken/lesson.yaml --level V1 --report json  # 预期退出 1：E5203 + E5401
./scripts/check-clippy-rules.sh      # clippy.toml 的 60 条 std 规则确实命中
./scripts/check-no-os-rng.sh         # cetus-wasm / cetus-cli 的产物依赖不含 getrandom
./scripts/check-cross-target.sh      # native = wasm（一次跑完与分段推进）= golden/hashes.txt
node scripts/bench-wasm.cjs target/wasm-pkg/nodejs/cetus.js   # 10 万步吞吐（上一步已构建 nodejs 包）
npm ci && npm run lockfile:check && npm run build:wasm
npm run typecheck && npm test && npm run build --workspace @cetus/web
npx playwright install chromium && npm run test:e2e --workspace @cetus/web   # 12 个端到端测试
```

浏览器验收（开发服务器）：

```bash
npm run dev --workspace @cetus/web
# 默认打开 SIR；切换到「太阳能板与蓄电池」；播放/暂停/单步/重置；
# 拖动参数；点击画布节点/边编辑方程；应用后确认仿真重置。
```
