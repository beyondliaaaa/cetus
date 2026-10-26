# ADR-0006：K3 分析层与轨迹查询接口

- 状态：草案（2026-10-26，待人工评审；未合并）
- 相关：ADR-0004（K3/K4）、§5.2、§5.5、§5.13、§6.2、§6.3、§8.4、§12 M3
- 取代：无（本 ADR 定稿 ADR-0004 中推迟到 M3 的 K3；批准后同步更新 ADR-0004 的 K3 行）

## 背景

M3 的下一段关键路径是轨迹查询层与 K3。两件事互相依赖：

1. **轨迹查询层**：V3 行为测试、课程检查器、L0 的 `world_state` 和 t4 的
   `first_time(charge_pct <= 0)` 都需要在一条轨迹上求
   `first_time`、`min`、`mean`、`max_where`。这些不是模型方程里的逐宏步表达式，
   而是**对 `Dataset` 的后处理**（§5.2 把它们列为“只在测试 / 检查器中可用”的时间查询）。
2. **K3 分析层**：ADR-0004 只记录了 trait 形状（`Analysis::run(plan, cfg, io) -> Dataset`），
   但 `AnalysisId`、`AnalysisCfg`、`AnalysisIo` 尚未定义；`cetus verify` 的 V2/V3 和
   CLI 的 sweep/transient 目前各自手写仿真循环。M3 要求把“模型是什么”和“要算什么”
   分开，并让检查器、行为测试、前端都面向同一个 `Dataset`（K4）。

本 ADR 给出这两层的**第一版接口**，只定稿接口，不改变仿真语义。V3 的“与文献对照”
仍然等到专家给出处；本接口不依赖它。

## 选项

- **A. 放在 `cetus-kernel`**：K3/K4 都是内核之上的核心抽象；`Dataset` 已在
  `cetus-kernel::dataset`，查询层只依赖 `cetus-expr`，不引入新 crate。推荐。
- B. 新建 `cetus-analysis` crate：边界更清楚，但 M3 只有一个实现者和一个消费者；
  新增 crate 会让 `cetus-kernel` → `cetus-analysis` 的依赖方向需要重新论证。
- C. 放在 `cetus-verify`：拒绝。lesson-runtime、实验台和未来 UI 也要查询轨迹，
  `cetus-verify` 是验收工具，不应成为它们依赖的核心层。

## 决定（草案）

### 1. 代码位置

- `cetus-kernel::analysis`：`Analysis`、`AnalysisId`、`AnalysisCfg`、`AnalysisIo`、
  `SimFactory`、`AnalysisEvents`，以及 `Transient`、`Sweep`。
- `cetus-kernel::query`：`DatasetQuery`、`QueryValue`、`QueryError`。
- `cetus-cli` / `cetus-wasm` 提供 `SimFactory` 的实现（当前是 `cetus-sd` 的
  `SdSubsystem` + `Sim`）；`cetus-kernel` 不依赖任何范式 crate。

### 2. K3 类型与 trait

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisId(&'static str);

impl AnalysisId {
    pub const fn new(id: &'static str) -> Self;
    pub const fn as_str(self) -> &'static str;
}

pub trait Analysis {
    fn id(&self) -> AnalysisId;

    fn run(
        &self,
        plan: &Arc<CompiledPlan>,
        cfg: &AnalysisCfg,
        io: &mut AnalysisIo<'_>,
    ) -> Result<Dataset, SimError>;
}
```

`AnalysisId` 用 `&'static str` 而不是枚举，是为了让未来的分析可以来自新的 crate，
不需要修改核心抽象（P13）。内置分析目前是 `transient`、`sweep`。

### 3. `SimFactory` 与 `AnalysisIo`

```rust
pub trait SimFactory {
    fn make(&self, plan: &Arc<CompiledPlan>, seed: u64) -> Result<Sim, SimError>;
}

pub trait AnalysisEvents {
    fn progress(&mut self, done: u64, total: u64);
}

pub struct AnalysisIo<'a> {
    pub seed: u64,
    pub observations: Option<&'a Dataset>,
    pub sim_factory: &'a dyn SimFactory,
    pub events: &'a mut dyn AnalysisEvents,
}
```

- `SimFactory` 由调用方提供，捕获模型声明的积分器/范式（`cetus-kernel` 不能依赖
  `cetus-sd`）。同一个 `plan` 与 `seed` 必须得到同一个初始 `Sim`。
- `observations` 供 `fit`/对照类分析使用；`transient`/`sweep` 忽略它。
- `AnalysisEvents` 目前只有进度；取消是 UI 的需求，等 lesson-runtime 落地时再写
  ADR 扩展，不在第一版接口里增加 `Result`/取消状态。

### 4. `AnalysisCfg`

```rust
#[derive(Debug, Clone, Default)]
pub struct AnalysisCfg {
    values: IndexMap<String, AnalysisValue>,
}

#[derive(Debug, Clone)]
pub struct Quantity {
    /// Value in `unit` (not SI).
    pub value: f64,
    pub unit: Unit,
}

impl Quantity {
    pub fn si_value(&self) -> f64;
}

#[derive(Debug, Clone)]
pub struct QuantityList {
    pub values: Vec<f64>,
    pub unit: Unit,
}

#[derive(Debug, Clone)]
pub struct SweepAxis {
    pub parameter: String,
    pub values: Vec<f64>,
    pub unit: Unit,
}

#[derive(Debug, Clone)]
pub enum AnalysisValue {
    Number(Quantity),
    Integer(i64),
    Bool(bool),
    Text(String),
    Texts(Vec<String>),
    Numbers(QuantityList),
    Axes(Vec<SweepAxis>),
    Parameters(IndexMap<String, Quantity>),
}

impl AnalysisCfg {
    pub fn insert(&mut self, key: impl Into<String>, value: AnalysisValue) -> Option<AnalysisValue>;
    pub fn get(&self, key: &str) -> Option<&AnalysisValue>;
    pub fn quantity(&self, key: &str) -> Option<&Quantity>;
    pub fn integer(&self, key: &str) -> Option<i64>;
    pub fn bool(&self, key: &str) -> Option<bool>;
    pub fn text(&self, key: &str) -> Option<&str>;
    pub fn texts(&self, key: &str) -> Option<&[String]>;
    pub fn axes(&self, key: &str) -> Option<&[SweepAxis]>;
    pub fn parameters(&self, key: &str) -> Option<&IndexMap<String, Quantity>>;
}
```

- `AnalysisCfg` 是**带单位的值映射**，不解析 YAML/JSON；CLI/wasm 负责把文本解析成它。
- `Quantity.value` 用声明单位，`si_value()` 才乘 `unit.factor()`；这样作者写
  `until: 72[h]`，调用方不需要先换算。
- `parameters` 用于 `transient` 的参数覆盖（`sim.set_param`）；库存初值覆盖由调用方
  先编译一个变体 `plan`，不在 `AnalysisCfg` 里表达，因为 `Sim` 没有“改初始存量”的接口，
  而 V3 行为测试本来就要为每个 setup 编译一次。
- `IndexMap` 保留插入顺序，进度和输出列顺序可复现。

### 5. `Transient` 与 `Sweep`

`Transient` 配置键：

| 键 | 类型 | 必填 | 含义 |
|---|---|---|---|
| `until` | `Number(Quantity)` | 是 | 仿真到多久（时间单位） |
| `probes` | `Texts` | 否 | 观测名；缺省为全部 observable |
| `parameters` | `Parameters` | 否 | 运行前覆盖的参数（值用声明单位） |

输出：一个 `Time` 轴的 `Dataset`（K4）。`until` 必填，因为 `CompiledPlan` 不保存
`Model.time.horizon`。

`Sweep` 配置键：

| 键 | 类型 | 必填 | 含义 |
|---|---|---|---|
| `axes` | `Axes` | 是 | 每个参数的值列表（声明单位） |
| `until` | `Number(Quantity)` | 是 | 每个点仿真到多久 |
| `probes` | `Texts` | 否 | 观测名；缺省为全部 observable |
| `parameters` | `Parameters` | 否 | 基础参数覆盖 |
| `reduce` | `Texts` | 否 | 每个点上的轨迹查询，例如 `mean(pv_power)`；缺省保留最终值 |

第一版 `Sweep` 输出**每个参数点的最终值**（若给 `reduce` 则输出查询结果），
维度是 `Parameter` 轴（最外层是第一个轴）。完整 `Time × Parameter` 轨迹留给后续
`Sweep` 扩展；V3 的弹性/单调性测试可以用 `reduce` 表达，不需要在查询层做多维切片。

### 6. `DatasetQuery`

```rust
pub struct QueryValue {
    /// Value in SI units.
    pub value: f64,
    pub unit: Unit,
}

impl QueryValue {
    pub fn display_value(&self) -> f64;
}

pub enum QueryError {
    NotTimeSeries,
    UnknownColumn(String),
    UnknownVariable(String),
    InvalidExpression(String),
    TypeMismatch(String),
    Empty,
}

pub struct DatasetQuery<'a> {
    // Dataset must have exactly one Time axis.
}

impl<'a> DatasetQuery<'a> {
    pub fn new(dataset: &'a Dataset) -> Result<Self, QueryError>;

    /// First time the boolean condition is true, in the time axis's display unit.
    pub fn first_time(&self, condition: &str) -> Result<Option<f64>, QueryError>;

    /// Value at the sample nearest to `time` (display unit).
    pub fn value_at(&self, column: &str, time: f64) -> Result<QueryValue, QueryError>;

    pub fn min(&self, column: &str) -> Result<QueryValue, QueryError>;
    pub fn mean(&self, column: &str) -> Result<QueryValue, QueryError>;

    /// Maximum of `column` over rows where `condition` is true.
    pub fn max_where(
        &self,
        column: &str,
        condition: &str,
    ) -> Result<Option<QueryValue>, QueryError>;
}
```

- 第一版只支持**单 `Time` 轴**的 `Dataset`；多维 `Sweep` 输出不做切片查询，
  避免在接口里提前引入 `SliceSelection`。`DatasetQuery::new` 对多维数据返回
  `NotTimeSeries`。
- `first_time` 返回显示单位的时间（与课程里的 `first_time(charge_pct <= 0)` 和
  `abs_tol: "1[h]"` 对齐），不返回 `QueryValue`。
- `min`/`mean`/`max_where`/`value_at` 返回 `QueryValue`（SI 值 + 显示单位）；
  调用方用 `display_value()` 显示。
- 条件表达式沿用 `cetus-expr` 的解析/类型检查/求值：`DatasetResolver` 把列名映射为
  `Type::scalar(column.unit.dim())`，列槽位就是列索引。条件必须是 `Bool`。
- 一次查询一个条件时直接传字符串；重复查询时后续可加 `CompiledQuery`（编译一次、
  多次求值），第一版不暴露，避免接口过早膨胀。
- `integral` 暂不进入第一版：它需要“列单位 × 时间单位”的派生单位，而
  `cetus-units::Unit` 目前没有公开的乘法构造；等 V3 需要时再定。

### 7. 错误与确定性

- 查询错误不返回 `Diagnostic`；它们是编程/内容错误，由调用方映射成 E7xxx/E5xxx。
- 行顺序固定为 `Dataset` 的存储顺序；`first_time` 取第一个满足条件的行，
  `max_where` 取最大值（同值取第一行）。时间不进浮点累加，直接用 tick × quantum。
- 所有查询只读 `Dataset`，不创建 `Sim`，因此天然可复现。

## 待评审的问题

1. **`Analysis::run` 的 plan 参数**：ADR-0004 记录的是 `&CompiledPlan`；本草案改成
   `&Arc<CompiledPlan>`，这样 sweep 的每个点共享同一个计划，不用克隆。是否接受？
2. **`AnalysisCfg` 形状**：带单位的值映射（本草案）还是 `transient`/`sweep` 各一个
   具体 struct？映射更符合指南“带单位的值映射”，但键是字符串，缺少编译期检查。
3. **`Sweep` 输出**：第一版只输出每个点的最终值/`reduce` 结果，还是直接输出
   `Time × Parameter` 轨迹？前者简单、够 V3 的弹性/单调性；后者更通用，但需要
   多维切片查询。
4. **`first_time` 返回**：`Option<f64>`（显示单位）还是 `QueryValue`（带单位）？
   本草案选前者，与课程答案对齐。
5. **`min`/`mean`/`max_where` 的单位**：返回 SI 值 + 显示单位（本草案），还是返回
   显示值？本草案选前者，和 `Dataset::Column.values` 的 SI 存储一致。
6. **查询条件的编译**：第一版每次传字符串（调用方无需管理生命周期）；是否现在
   就加 `CompiledQuery`？
7. **`AnalysisEvents`**：第一版只有进度、没有取消；取消等 lesson-runtime 落地时
   再写 ADR，是否同意？

## 后果

- 接口一旦批准，`Analysis`/`AnalysisCfg`/`AnalysisIo`/`DatasetQuery` 的公开形状
  受 ADR-0004 的“修改需新 ADR”约束；后续分析（`ensemble`、`fit`、`steady_state`）
  只新增实现，不修改 trait。
- `cetus-verify` 的 V3 行为测试、lesson 检查器和未来 UI 都通过 `DatasetQuery` 读
  轨迹，不再各自写 `first_time`/`mean`。
- CLI 的 `sweep` 可以在后续 PR 中改为调用 `Sweep` 分析；V2 的扫描逻辑保持不变，
  直到 V3 需要共享输出格式。
- 第一版不引入新依赖；`cetus-kernel` 已有 `cetus-expr`、`cetus-compile`、`cetus-units`。

## 实施顺序（批准后）

1. `cetus-kernel::query` + 单元测试（单 Time 轴；`first_time`/`min`/`mean`/`max_where`/
   `value_at`）。
2. `cetus-kernel::analysis` + `Transient` + 测试；CLI 的 `run` 改用它（或先并存）。
3. `Sweep` + 测试；`cetus-verify` V3 行为测试使用 `DatasetQuery`。
4. 更新 ADR-0004 的 K3 行与 STATUS。
