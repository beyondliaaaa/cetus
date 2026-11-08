# ADR-0006：K3 分析层与轨迹查询接口

- 状态：已接受（2026-10-26，人工评审通过；PR #11 合并）
- 相关：ADR-0004（K3/K4）、§5.2、§5.5、§5.13、§6.2、§6.3、§8.4、§12 M3
- 取代：无（本 ADR 定稿 ADR-0004 中推迟到 M3 的 K3；批准后同步更新 ADR-0004）

## 本次修订摘要

相对上一版草案，本版按两轮评审意见确定了以下事项：

1. `Transient` 和 `Sweep` 接受**带时间戳的操作列表**（第一版只支持参数变更），在宏步边界生效。
2. `SimFactory::make(plan, seed, &SimInit)`；`SimInit` 带存量初值覆盖（值或表达式），
   不再为每组设置重新编译 `plan`。理由是 V4 判别实验要搜 256 组设置，浏览器预算 200 ms。
3. 查询表达式有明确的**查询作用域**：`Dataset` 列、模型参数；`first_time` / `value_at` /
   `min` / `mean` / `max_where` 作为 `cetus-expr` 内置聚合函数，整个表达式先类型检查、
   再编译成 `CompiledQuery`、最后求值。
4. `first_time` 返回 `Option<QueryValue>`（SI 值 + 单位），与其他查询一致。
5. 每个分析提供**强类型解析函数**（`TransientCfg::parse` / `SweepCfg::parse`），
   遇到未知键就报错。
6. `AnalysisIo`、`AnalysisValue`、`QueryError` 加 `#[non_exhaustive]`。
7. 其余沿用上一版：`&Arc<CompiledPlan>`（在 ADR-0004 中记录）、`Sweep` 只输出最终值和
   `reduce` 结果、查询返回 SI 值、`AnalysisEvents` 只有进度。
8. `SimInit` 增加**参数覆盖**：先套用参数，再重算全部存量初值（模型自身的 initial
   表达式 + 课程覆盖）。这修复 `Sim::set_param` 不重算存量初值的问题；V2 扫描与
   实验台“开始运行前调参”都走这条路径。
9. `CompiledQuery` 支持**时间窗口**（`eval_in(dataset, window)`），供 `world_state`、
   `trajectory_fit` 的 `window` 使用。

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

另外两个需求推动了本版修订：

- **操作序列**：课程和世界会在运行中改变参数（玩家调参、事件、判别实验）。分析必须能
  表达“在 t 时刻把 load 改成 1200 W”，并且和内核一样在宏步边界生效（§5.5）。
- **初值覆盖不重编译**：V4 的判别实验要在 `setup_space` 内搜 256 组设置；如果每组都
  重新编译 `plan`，浏览器 200 ms 的预算不够。初值覆盖必须作用在已有 `plan` 和 `Sim` 上。
- **参数覆盖与初值（现有 bug）**：`Sim::set_param` 只重算辅助量，不重算存量初值。
  solar-battery 的 `initial: "0.5 * capacity"` 在创建 Sim 时按默认容量算成 10 kWh；
  V2 扫描到 `capacity = 1 kWh` 时仍从 10 kWh 起步，第一步被钳制回上限。结果是扫描
  测的场景与模型声明不一致，V2 却照样报 PASS。`SimInit` 必须先套用参数、再计算
  全部存量初值（模型 initial 表达式 + 课程覆盖）；V2 扫描和实验台开始前调参都走它。

## 选项

- **A. 放在 `cetus-kernel`**：K3/K4 都是内核之上的核心抽象；`Dataset` 已在
  `cetus-kernel::dataset`，查询层只依赖 `cetus-expr`，不引入新 crate。推荐。
- B. 新建 `cetus-analysis` crate：边界更清楚，但 M3 只有一个实现者和一个消费者；
  新增 crate 会让 `cetus-kernel` → `cetus-analysis` 的依赖方向需要重新论证。
- C. 放在 `cetus-verify`：拒绝。lesson-runtime、实验台和未来 UI 也要查询轨迹，
  `cetus-verify` 是验收工具，不应成为它们依赖的核心层。

## 决定

### 1. 代码位置

- `cetus-kernel::analysis`：`Analysis`、`AnalysisId`、`AnalysisCfg`、`AnalysisIo`、
  `SimFactory`、`SimInit`、`AnalysisEvents`，以及 `Transient`、`Sweep`。
- `cetus-kernel::query`：`CompiledQuery`、`DatasetQuery`、`QueryValue`、`QueryResult`、
  `QueryError`、`ParameterValues`。
- `cetus-expr`：新增 `Type::Series` 与聚合内置函数（`first_time` / `value_at` /
  `min` / `mean` / `max_where`）的查询语义。
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
    ) -> Result<Dataset, AnalysisError>;
}

#[non_exhaustive]
pub enum AnalysisError {
    Config(CfgError),
    Query(QueryError),
    Sim(SimError),
}
```

- `AnalysisId` 用 `&'static str` 而不是枚举，是为了让未来的分析可以来自新的 crate，
  不需要修改核心抽象（P13）。内置分析目前是 `transient`、`sweep`。
- `AnalysisError` 让“配置写错”“查询失败”和“仿真失败”分开；`Analysis::run` 接收的是
  原始 `AnalysisCfg`，内部调用该分析的强类型解析函数（见 §4）；`Sweep` 的 `reduce`
  查询失败走 `AnalysisError::Query`。

### 3. `SimInit` 与 `SimFactory`

```rust
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct SimInit {
    pub parameters: IndexMap<String, Quantity>,
    pub stock_initial: IndexMap<String, InitialValue>,
}

#[non_exhaustive]
pub enum InitialValue {
    /// A value in the declared unit.
    Value(Quantity),
    /// An expression evaluated against the selected parameters.
    Expression(String),
}

pub trait SimFactory {
    fn make(
        &self,
        plan: &Arc<CompiledPlan>,
        seed: u64,
        init: &SimInit,
    ) -> Result<Sim, SimError>;
}

#[non_exhaustive]
pub struct AnalysisIo<'a> {
    pub seed: u64,
    pub observations: Option<&'a Dataset>,
    pub sim_factory: &'a dyn SimFactory,
    pub events: &'a mut dyn AnalysisEvents,
}

impl<'a> AnalysisIo<'a> {
    pub fn new(
        seed: u64,
        sim_factory: &'a dyn SimFactory,
        events: &'a mut dyn AnalysisEvents,
    ) -> Self;

    pub fn with_observations(self, observations: &'a Dataset) -> Self;
}
```

- `SimFactory` 由调用方提供，捕获模型声明的积分器/范式（`cetus-kernel` 不能依赖
  `cetus-sd`）。同一个 `plan`、`seed` 与 `SimInit` 必须得到同一个初始 `Sim`。
- `AnalysisIo` 是 `#[non_exhaustive]`：字段只能通过 `new` / `with_observations`
  设置，后续增加字段不破坏调用方。
- `SimInit` 同时覆盖**参数**与**存量初值**；应用顺序固定为：
  1. 套用 `parameters`（按声明单位转 SI，校验存在与范围）；
  2. 用所选参数重新计算**模型自身的全部** `stocks[].initial` 表达式；
  3. 套用 `stock_initial` 覆盖（值或表达式）；
  4. 调用子系统的 `init`。
  这修复了 `Sim::set_param` 只重算辅助量、不重算存量初值的问题。编译器需要保留
  存量 initial 的 `CompiledExpr`（`CompiledStock.initial` 仍是默认值，供 metadata），
  才能在参数变化后重算。
- 全过程不重新编译 `plan`：工厂在已有的 `plan` 上创建 `Sim`。V4 搜索 256 组设置时
  只重复创建 `Sim`，不重复编译计划，落在浏览器 200 ms 预算内。
- `InitialValue::Expression` 只允许引用**模型参数**；引用存量/辅助变量是
  `CfgError::InvalidValue`，解析后的初值超出存量 `bounds` 是
  `SimError::InvalidInitial { stock, value }`（为此 `SimError` 增加
  `InvalidInitial` 变体，并加 `#[non_exhaustive]`）。不接受“由实现定义”的作用域。
- V2 扫描和实验台的“开始运行前调参”都走 `SimInit`；V2 的 `run_one` 不再先建 `Sim`
  再 `set_param`。
- `SimInit` 的键必须是存量；不是存量时返回配置/运行错误，而不是静默忽略。

### 4. `AnalysisCfg` 与强类型解析

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
#[non_exhaustive]
pub enum AnalysisValue {
    Number(Quantity),
    Integer(i64),
    Bool(bool),
    Text(String),
    Texts(Vec<String>),
    Numbers(QuantityList),
    Axes(Vec<SweepAxis>),
    Parameters(IndexMap<String, Quantity>),
    Initial(IndexMap<String, InitialValue>),
    Operations(Vec<Operation>),
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum CfgError {
    UnknownKey(String),
    MissingKey(&'static str),
    WrongType { key: String, expected: &'static str },
    InvalidValue { key: String, message: String },
}
```

- `AnalysisCfg` 是**带单位的值映射**，不解析 YAML/JSON；CLI/wasm 负责把文本解析成它。
- `Quantity.value` 用声明单位；`si_value()` 才乘 `unit.factor()`。
- 每个分析定义自己的强类型配置，并逐键解析；**未知键直接报 `CfgError::UnknownKey`**，
  不允许静默忽略：

```rust
pub struct TransientCfg {
    pub until: Quantity,
    pub probes: Vec<String>,
    pub init: SimInit,
    pub operations: Vec<Operation>,
}

impl TransientCfg {
    pub fn parse(plan: &CompiledPlan, cfg: &AnalysisCfg) -> Result<Self, CfgError>;
}

pub struct SweepCfg {
    pub axes: Vec<SweepAxis>,
    pub until: Quantity,
    pub probes: Vec<String>,
    pub init: SimInit,
    pub operations: Vec<Operation>,
    pub reduce: Vec<String>,
}

impl SweepCfg {
    pub fn parse(plan: &CompiledPlan, cfg: &AnalysisCfg) -> Result<Self, CfgError>;
}
```

`Transient` 和 `Sweep` 的 `run` 内部先调用对应的 `parse`，把 `CfgError` 映射为
`AnalysisError::Config`。`AnalysisCfg` 的 `parameters` 与 `initial` 键都解析进同一个
`SimInit`（`TransientCfg.init` / `SweepCfg.init`）。

### 5. 操作序列

```rust
#[derive(Debug, Clone)]
pub struct Operation {
    /// Time in the model's display time unit.
    pub at: Quantity,
    pub action: OperationAction,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum OperationAction {
    SetParameter { name: String, value: Quantity },
}
```

- 第一版只支持参数变更；后续可以扩展为学生代码返回的 `Policy` 动作、世界事件等。
- 操作按 `(at.si_value(), 输入顺序)` 稳定排序；同一时刻的操作保持输入顺序。
- 操作在**宏步边界**生效：在第一个满足 `boundary_time >= at` 的边界处应用，参数值对
  之后的宏步有效。实现可以按操作时间切分 `run_until`，在边界调用 `sim.set_param`；
  这与 §5.5 的操作日志语义一致，不要求当前内核立即实现 `enqueueAction`。
- `Transient` 和 `Sweep` 都接受 `operations`；`Sweep` 的每个参数点使用同一组操作
  （时间相对每个点的时间零点）。

### 6. 查询作用域与 `CompiledQuery`

查询表达式有独立的作用域，不是模型方程的逐宏步作用域：

- `time`：数据集的 `Time` 轴，类型 `Series<Time>`。
- `Dataset` 的每一列：`Series<T>`，`T` 是列的单位/量纲。
- 模型参数：`T`（标量），值来自调用方提供的 `ParameterValues`（SI）；名字/量纲
  来自 `CompiledPlan`。
- 标量运算符、比较、`and`/`or`/`not` 在 `Series` 上逐元素提升。
- 聚合函数（`cetus-expr` 内置）：

```rust
min(Series<T>) -> T
mean(Series<T>) -> T
max_where(Series<T>, Series<Bool>) -> T
first_time(Series<Bool>) -> Time
value_at(Series<T>, Time) -> T
```

- `min` / `max` 保留现有的标量二元重载；查询编译器按参数类型/元数选择聚合版本。
- `Type::Series(Box<Type>)` 只对查询编译有效；模型方程里出现 `Series` 时报专门的
  **E1902**（新增诊断页），而不是笼统的类型错误。

查询先整体类型检查，再编译：

```rust
#[derive(Debug, Clone, Default)]
pub struct ParameterValues {
    values: IndexMap<String, f64>, // SI
    changed_by_operation: BTreeSet<String>,
}

impl ParameterValues {
    pub fn from_plan(plan: &CompiledPlan) -> Self;
    pub fn set(&mut self, name: &str, si_value: f64);
    pub fn mark_changed_by_operation(&mut self, name: &str);
}

pub struct TimeWindow {
    /// Inclusive start, in the dataset's display time unit.
    pub start: Quantity,
    /// Inclusive end, in the dataset's display time unit.
    pub end: Quantity,
}

pub struct CompiledQuery { /* compiled expression + bound columns/parameters */ }

impl CompiledQuery {
    pub fn compile(
        source: &str,
        plan: &CompiledPlan,
        parameters: &ParameterValues,
        dataset: &Dataset,
    ) -> Result<Self, QueryError>;

    pub fn eval(&self, dataset: &Dataset) -> Result<QueryResult, QueryError>;

    /// Restricts the dataset to `[start, end]` before evaluating the query.
    pub fn eval_in(
        &self,
        dataset: &Dataset,
        window: TimeWindow,
    ) -> Result<QueryResult, QueryError>;
}

#[non_exhaustive]
pub enum QueryResult {
    Scalar(QueryValue),
    Bool(bool),
    Optional(Option<QueryValue>),
}

pub struct QueryValue {
    /// Value in SI units.
    pub value: f64,
    pub unit: Unit,
}

impl QueryValue {
    pub fn display_value(&self) -> f64;
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum QueryError {
    NotTimeSeries,
    UnknownColumn(String),
    UnknownParameter(String),
    UnknownVariable(String),
    InvalidExpression(String),
    TypeMismatch(String),
    ParameterChangedByOperation(String),
    Empty,
}
```

- `"min(charge) > 0.05 * capacity"` 的类型是 `Bool`，`eval` 返回
  `QueryResult::Bool(...)`；`capacity` 来自 `ParameterValues`，量纲和 `CompiledPlan`
  一致。
- `"first_time(charge_pct <= 0)"` 的类型是 `Time`，`eval` 返回
  `QueryResult::Optional(Option<QueryValue>)`；`first_time` 不命中时是 `None`。
- `first_time` 返回 `Option<QueryValue>`（SI 值 + 时间单位），不是裸 `f64`；调用方
  用 `display_value()` 显示为小时等。
- `min` / `mean` / `value_at` 返回 `QueryValue`；`min` / `mean` 在空窗口（或空
  `Dataset`）时报 `QueryError::Empty`。
- `max_where` 在 API 层返回 `Option<QueryValue>`（无匹配行时 `None`）；在标量表达式
  里使用时，空集报 `QueryError::Empty`。
- `first_time` 在无匹配行时返回 `None`，不是 `Empty`。
- 被操作序列改过的参数在查询中引用时，报 `QueryError::ParameterChangedByOperation`
  （第一版；避免“查询用的是哪个时刻的参数”歧义）。
- `eval_in(dataset, window)` 先把 `Dataset` 截取到 `[start, end]`（含端点），再求值；
  `world_state` 与 `trajectory_fit` 的 `window` 直接映射到它。窗口为空时按上面的空集
  规则处理。
- 查询只支持**单 `Time` 轴**的 `Dataset`；多维 `Sweep` 输出不做切片查询，V3 的弹性/
  单调性用 `Sweep` 的 `reduce` 表达。多维切片留给后续 ADR。
- `CompiledQuery` 绑定 `Dataset` 的列顺序和 `ParameterValues`；数据集 schema 变化后
  必须重新编译，不能复用。

### 7. `DatasetQuery` 便利层

```rust
pub struct DatasetQuery<'a> { /* dataset + compiled query context */ }

impl<'a> DatasetQuery<'a> {
    pub fn new(
        dataset: &'a Dataset,
        plan: &CompiledPlan,
        parameters: &ParameterValues,
    ) -> Result<Self, QueryError>;

    pub fn compile(&self, source: &str) -> Result<CompiledQuery, QueryError>;

    pub fn first_time(&self, condition: &str) -> Result<Option<QueryValue>, QueryError>;
    pub fn min(&self, column: &str) -> Result<QueryValue, QueryError>;
    pub fn mean(&self, column: &str) -> Result<QueryValue, QueryError>;
    pub fn max_where(
        &self,
        column: &str,
        condition: &str,
    ) -> Result<Option<QueryValue>, QueryError>;
    pub fn value_at(&self, column: &str, time: Quantity) -> Result<QueryValue, QueryError>;
}
```

便利方法内部调用 `CompiledQuery`；重复查询时调用方应保存 `CompiledQuery`，不要每次
重新编译。

### 8. 单位、确定性与进度

- 查询值统一用 SI 存储，`QueryValue.unit` 是显示单位；`display_value()` 除以 `factor()`。
- 行顺序是 `Dataset` 的存储顺序；`first_time` 取第一个满足条件的行，`max_where` 取
  最大值（同值取第一行），`mean` 是算术平均。时间不进浮点累加，直接用 tick × quantum。
- `AnalysisEvents` 只有进度：

```rust
pub trait AnalysisEvents {
    fn progress(&mut self, done: u64, total: u64);
}
```

取消是 UI 的需求；未来加一个带默认实现的取消方法不算破坏性修改，所以现在不加，
也不在第一版增加 `Result`/取消状态。

### 9. `Sweep` 输出

- `SweepCfg.reduce` 为空时，输出每个参数点的各 probe 最终值；非空时，每个 `reduce`
  字符串是一个 `CompiledQuery` 表达式，对每个点的轨迹求值，输出该标量。
- 输出 `Dataset` 的维度是 `Parameter` 轴（最外层是第一个轴）；不输出
  `Time × Parameter` 全轨迹。
- V3 的弹性/单调性测试用 `reduce` 表达，例如 `mean(pv_power)`。

## 评审结论（2026-10-26）

以下 7 项已经人工确认：

| # | 结论 |
|---|---|
| 1 | `Analysis::run` 使用 `&Arc<CompiledPlan>`；已在 ADR-0004 记一笔。 |
| 2 | `AnalysisCfg` 保持带单位的值映射；每个分析提供强类型解析函数，未知键报错，不能静默忽略。 |
| 3 | `Sweep` 只输出每个参数点的最终值和 `reduce` 结果，V3 够用。 |
| 4 | **`first_time` 返回 `Option<QueryValue>`（SI 值 + 单位）**，不返回显示单位的裸数字；所有查询统一 SI + 单位。 |
| 5 | `min`/`mean`/`max_where`/`value_at` 返回 SI 值 + 单位。 |
| 6 | 采用 `CompiledQuery`：查询表达式先编译、再求值；判别实验要值 256 次，必须有这个能力。 |
| 7 | `AnalysisEvents` 第一版只有进度；未来加一个带默认实现的取消方法不算破坏性修改，现在不用加。 |

## 第二轮评审结论（2026-10-26）

| # | 结论 |
|---|---|
| 1 | 接受 `AnalysisError`，并新增 `Query(QueryError)` 变体。 |
| 2 | 接受 `Type::Series`；同步 §5.2 与 ADR-0004（注明只用于查询）；模型方程出现 `Series` 时报专门的 **E1902**，并补诊断页。 |
| 3 | 接受显式 `ParameterValues`；被操作序列改过的参数在查询中引用时，报 `QueryError`（第一版）。 |
| 4 | 接受：`max_where` 在 API 层返回 `None`，标量上下文报 `Empty`；`min`/`mean` 遇到空窗口也报 `Empty`。 |
| 5 | 初值表达式只允许引用参数；引用存量或初值超出 `bounds` 时报错。`SimInit` 增加参数覆盖，先套参数、再算全部存量初值；V2 扫描与实验台开始前调参走同一条路径，并补回归测试。 |
| 6 | 接受 `Sweep` 只输出最终值和 `reduce` 结果。 |
| + | `CompiledQuery` 支持时间窗口（`eval_in`），供 `world_state`、`trajectory_fit` 使用。 |

以上确认后无剩余接口问题；ADR 已接受。

## 后果

- 接口一旦批准，`Analysis`/`AnalysisCfg`/`AnalysisIo`/`SimInit`/`DatasetQuery` 的公开
  形状受 ADR-0004 的“修改需新 ADR”约束；后续分析（`ensemble`、`fit`、
  `steady_state`）只新增实现，不修改 trait。
- `cetus-verify` 的 V3 行为测试、lesson 检查器和未来 UI 都通过 `CompiledQuery` /
  `DatasetQuery` 读轨迹，不再各自写 `first_time`/`mean`。
- `SimInit` 让 V4 的 256 组判别实验在浏览器预算内复用同一份 `plan`。
- CLI 的 `sweep` 可以在后续 PR 中改为调用 `Sweep` 分析；V2 的扫描逻辑保持不变，
  直到 V3 需要共享输出格式。
- 第一版不引入新依赖；`cetus-kernel` 已有 `cetus-expr`、`cetus-compile`、`cetus-units`。

## 实施顺序（批准后）

1. **先修 `SimInit` 与初值 bug**：`SimInit` 增加参数覆盖；编译器保留存量 initial 的
   `CompiledExpr`；`SimFactory`/`Sim` 先套参数、再算全部存量初值；V2 扫描和实验台
   开始前调参改走 `SimInit`；补 `set_param` 不重算初值的回归测试。
2. `cetus-kernel::query` + `cetus-expr` 的 `Series`/聚合函数 + 单元测试
   （`first_time`/`min`/`mean`/`max_where`/`value_at`，含 `eval_in`）。
3. `cetus-kernel::analysis` + `Transient` + 操作序列 + 测试；CLI 的 `run` 改用它
   （或先并存）。
4. `Sweep` + `reduce` + 测试；`cetus-verify` V3 行为测试使用 `CompiledQuery`。
5. 更新 ADR-0004 的 K3 行与 STATUS。
