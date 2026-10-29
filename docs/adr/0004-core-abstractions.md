# ADR-0004：核心抽象 K1–K14

- 状态：已接受（2026-10-23，接口设计定稿；实现状态见文末「实现状态」）
- 相关：§5.2、§5.5、§5.6.3、§5.8、§5.9、§5.11–§5.13、P13
- 取代：无

## 背景

P13 要求：新增一种仿真范式时，只能新增 crate，**不得修改**核心抽象。这意味着
K1–K14 必须在 M1 就按完整形态设计接口，即使大部分实现要等到 M6–M9。

本 ADR 记录接口决定，以及 M1 实现到什么程度。任何后续修改都需要新的 ADR。

## 决定

### K1 值类型

```rust
pub enum Type {
    Scalar(Dim),
    Int(Dim),
    Bool,
    Enum(EnumId),
    Complex(Dim),
    Vector(Box<Type>, usize),
    Matrix(Box<Type>, usize, usize),
    Array(Box<Type>),
    Field(Box<Type>, GridId),
    Table(AgentTypeId),
    Graph(GraphId),
    Geometry(GeomKind),
    Dataset(DatasetSchemaId),
}
```

- `Type` 定义在 `cetus-expr`，`Dim` 来自 `cetus-units`。
- M1 实现 `Scalar`、`Int`、`Bool`、`Enum` 的解析、名称解析与类型推导；
  其余分支在求值阶段报 E1901「类型尚未支持」。
- 关键要求（M1 的退出标准之一）：一个带 `Complex` 变量的模型必须能通过解析、
  名称解析和类型推导，只在求值时报 E1901。这条测试证明接口是通用的。

### K2 时间模型

```rust
pub enum TimeKind {
    Continuous,
    Discrete,
    None,
}

pub struct TimeBase {
    pub kind: TimeKind,
    pub quantum: f64,   // SI seconds per tick; 0 for None
    pub unit: Unit,     // display unit (h, day_local, ...)
    pub start_tick: u64,
}
```

- 时间一律用 `u64` 刻度表示，`t = start_tick * quantum + tick as f64 * quantum`。
  不做浮点累加（D6）。
- 多速率：子系统声明 `period_ticks: u64`，必须整除宏步的 tick 数；编译器检查。

### K3 分析层

```rust
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

- 模型描述「系统是什么」，分析描述「要算什么」。
- M1 实现 `transient` 与 `sweep`；`ensemble`、`fit` 在 M3。
- `AnalysisCfg` 是带单位的值映射，不解析 YAML。
- **M3 修订（ADR-0006，2026-10-26）**：`plan` 参数定为 `&Arc<CompiledPlan>`，
  以便 sweep/判别实验复用同一份计划；ADR-0006 同时定义 `AnalysisId`、`AnalysisCfg`、
  `AnalysisIo`、`SimInit`、`CompiledQuery`，其余接口仍在评审。

### K4 数据集

```rust
pub struct Dataset {
    pub schema: DatasetSchema,
    pub dims: Vec<DimAxis>,
    pub columns: Vec<Column>,
}

pub struct Column {
    pub name: String,
    pub unit: Unit,
    pub values: Vec<f64>,   // row-major over dims
}

pub enum DimAxis {
    Time { unit: Unit, ticks: Vec<u64> },
    Parameter { id: ParamId, values: Vec<f64>, unit: Unit },
    Ensemble { seed: u64 },
    Grid { nx: u32, ny: u32 },
    Frequency { unit: Unit, values: Vec<f64> },
}
```

- 时间序列是 `DimAxis::Time` 的特例。检查器和前端都面向 `Dataset`。
- `Dataset` 必须能序列化为 CSV（M1）与二进制（M2 的探针增量传输）。

### K5 求解服务

```rust
pub trait SolveService {
    fn solve(&self, problem: &SolveProblem) -> Result<SolveSolution, SolveError>;
}
```

- M1 只定义 trait 与 `SolveProblem` 的结构，不实现。
- 求解器必须确定性：固定迭代上限、固定容差推导规则、禁止依赖真实时间。

### K6 连接器语义

```rust
pub enum FieldKind {
    Signal,
    Resource,
    Across,
    Through,
}
```

- M1 只做 schema 定义与校验；`signal` + `resource` 在 M4 实现，非因果在波次 B。
- `resource` 字段的分配算法在世界层实现，宏步边界执行，守恒由测试保证。

### K7 随机性

```rust
pub fn derive_seed(global: u64, subsystem: &str, purpose: &str, entity: u64) -> [u8; 32];
```

- 用 BLAKE3 的 keyed hash 派生，再取前 32 字节作为 `ChaCha8Rng` 的种子。
- `purpose` 必须是登记过的字符串（§5.9）；新增用途先改指南。
- M1 定义 API 并实现 `derive_seed`；M1 的确定性测试只覆盖「同种子两次运行一致」。

### K8 事件排序键

```rust
pub struct EventKey {
    pub tick: u64,
    pub sub_time: u64,
    pub delta: u32,
    pub priority: i32,
    pub seq: u64,
}
```

- `Ord` 按字段顺序比较；`seq` 单调递增，保证全序。
- M1 定义结构；M6 的事件队列使用它。

### K9 玩家决策点

```rust
pub struct Decision { pub id: DecisionId, pub subsystem: SubsystemId, pub tick: u64, pub options: Vec<ActionSchema> }
```

- M1 只定义结构；M3 定义 `Decision` 事件接口；波次 B 广泛使用。

### K10 Python 边界

```rust
pub trait Policy {
    fn observe(&mut self, obs: &Dataset) -> Result<(), PolicyError>;
    fn act(&mut self, t: f64) -> Result<Vec<Action>, PolicyError>;
}
```

- 只在宏步边界调用；M1 只定义 trait，M8 实现 Pyodide 版本。

### K11 几何

- `cetus-geom` 定义 `Point2`、`Segment2`、`Polygon2`、`Transform2`、`Aabb2`。
- M1 只建类型与基础运算；M7 起 PDE 区域与地图使用。

### K12 视图契约

```rust
pub enum ViewKind {
    Series, Phase, Field2D, VectorField, Network, Particles, Scene2D,
    Tree, Matrix, BlochSphere, Circuit, Automaton, Trace,
}
```

- 每个范式在 `paradigms()` 中声明自己能产出的 `ViewKind`。
- 前端维护 `ViewKind -> component` 注册表；M2 起使用。

### K13 尺度

- 每个变量可以有 `scale: Option<f64>`（SI 单位下的典型量级）。
- 求解器的绝对容差 = `scale * rel_tol`；未声明时从 `initial` 与 `range` 推断。
- M1 的 `rk4` 固定步长不需要容差，但 `dopri5` 使用该规则。

### K14 单位扩展

- 对数单位：`dB`、`pH`、`Np`、`mag` 只作为输入/显示变换，内部用线性量。
- 角度：`rad` 无量纲但带 `angle` 标签；与普通无量纲相加给 W1301。
- 计数/货币/信息：`person`、`item`、`currency`、`bit` 是自定义基本量纲，不隐式
  与无量纲混用。
- M1 实现 `Dim` 的自定义量纲扩展与角度标签；对数单位在 M6 引入（需要 `log`
  单位变换的完整测试）。

## 实现状态（2026-10-23）

M1 的**退出标准**（§12）聚焦于量纲、表达式、SD、跨目标确定性与快照；下列接口中
只有被 M1 实际用到的部分已经落地。未落地的抽象在**首次使用它的里程碑之前**必须
定稿（更新本 ADR 或写新 ADR），不得在实现时顺手发明。

| 抽象 | 实际状态 | 未完成部分推迟到 |
|---|---|---|
| K1 | ✅ 标量类完整；`Complex` 等分支可解析、可类型推导、求值报 E1901 | — |
| K2 | ✅ `continuous` + `discrete`；整数刻度 | 多速率结构体：M6 |
| K3 | ⏳ 接口由 ADR-0006 定稿中：`&Arc<CompiledPlan>`、`SimInit`、`CompiledQuery` 已确认；实现待 M3 | — |
| K4 | ✅ `Dataset`（Time / Parameter / Ensemble 轴）+ CSV | — |
| K5 | ⏳ `cetus-solve` 是占位 crate，trait 与 `SolveProblem` 未定义 | M6 |
| K6 | ⏳ 连接器 schema 与 `FieldKind` 未定义 | M4（`signal` + `resource`） |
| K7 | ✅ `derive_seed` + `RngStreams` | — |
| K8 | ⏳ `EventKey` 未定义 | M6（DES 事件队列） |
| K9 | ⏳ `Decision` 未定义 | M3 |
| K10 | ⏳ `Policy` trait 未定义 | M8 |
| K11 | ⏳ `cetus-geom` 是占位 crate | M7 |
| K12 | ✅ `ViewKind` + `ParadigmId::views` 已实现；前端视图注册表随 M2 落地 | — |
| K13 | ⏳ 变量的 `scale:` 字段已解析；容差推导未实现（依赖 `dopri5`） | M1.5 |
| K14 | ✅ 自定义量纲 + 角度标签 | 对数单位：M6 |

## 后果

- M1 的接口评审必须对照本 ADR；任何「顺手改一下类型」都会破坏 P13。
- 未来范式的 PR 如果修改了本文档中已定稿的类型（`Type`、`TimeKind`、`Dataset`，
  以及后续定稿的 `EventKey`、`Analysis` 等），CI 应能通过 `git diff` 发现；
  评审时直接拒绝并要求新 ADR。
- M1 的测试包含一个「未来范式 schema」的编译测试（`content/fixtures/complex-fixture.yaml`），
  验证 K1 的接口通用性。
- 上表的推迟项是 M1 的范围修正，不是设计变更。K12 已在 M2 定稿
  （`ViewKind` + `ParadigmId::views()`，前端 `views/registry.ts` 随 M2 落地）；
  K3 的 `&Arc<CompiledPlan>` 已由 ADR-0006 确认（其余接口仍在评审）；K9（`Decision`）
  仍待 M3，依此类推，定稿时更新本 ADR。
