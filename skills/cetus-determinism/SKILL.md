---
name: cetus-determinism
description: Rules, checks and test recipes for keeping the Cetus (鲸鱼娘历险记) simulation bit-for-bit deterministic across native x86_64, aarch64 and wasm32 (P2, §5.12). Use when writing or reviewing any code that touches simulation state, random streams, time, snapshots, world-time actions or the Python runner.
---

# 确定性（鲸鱼娘历险记 / Cetus）

> **skill 版本**：v0.2（2026-10-23）· **对应指南版本**：v0.5.2
> v0.2 变更：§4 补充当前 `clippy.toml` 的实际规模与持续验证方式（`tools/clippy-probe`、`scripts/check-clippy-rules.sh`、`scripts/check-no-os-rng.sh`）。
> 指南版本高于上面的数字时，先读指南 §5.5、§5.9、§5.11、§5.12、§5.7.5、§7.12 的最新版本；两者冲突时以指南为准，并在 PR 描述中指出。

## 0. 什么时候用

- 编写或修改 `crates/` 下任何会影响仿真状态的代码：内核、范式、教学层的检查器、世界层、快照。
- 编写跨目标确定性测试、快照往返测试、扰动测试。
- 评审别人的 PR。
- 新增一个范式（指南 §5.6.3 检查清单的第 4 条）。
- 编写 `packages/py-runner`（Pyodide）或 TS 层中会产生 `Action` 的代码（FSRS 调度、时间额度、世界时间推进）。

## 1. 目标是什么

**P2**：给定相同的模型文件、种子和玩家操作序列，仿真结果在所有平台（native x86_64 / aarch64 / wasm32）上**逐位一致**。

为什么这么严格：
- 验收用 native 内核，玩家用 WASM 内核（P4）。两者不一致，验收就不能代表玩家看到的东西。
- 存档、回放、平行星球、请教事件的复现，都依赖“同样的输入得到同样的状态”。
- 状态哈希是 CI 能自动发现问题的唯一手段。

确定性**不等于**数值正确。步长收敛、守恒、误差是 V2 的事；这里只管“每次都算出同样的位”。

## 2. 规则

每条规则后面是：为什么、怎么做、怎么检查。

### D1 不用无序容器参与任何会影响状态的迭代

- **为什么**：`HashMap` / `HashSet` 的迭代顺序取决于哈希种子，每次运行都可能不同。
- **怎么做**：用 `indexmap::IndexMap` / `IndexSet`（保持插入顺序）或 `BTreeMap` / `BTreeSet`（按键排序）。反序列化 YAML 的映射时也用 `IndexMap`，让声明顺序成为插入顺序。
- **怎么检查**：`clippy.toml` 的 `disallowed-types`（见 §4）。非仿真代码（例如 CLI 的参数处理）确实需要时，用 `#[allow(clippy::disallowed_types)]` 并写注释说明为什么不影响状态。

### D2 超越函数一律走 `cetus-math`

- **为什么**：`f64::sin` 等标准库方法在不同平台可能调用不同的数学库实现，最后一位可能不同。
- **怎么做**：`cetus_math::{sin, cos, tan, exp, ln, log10, pow, sqrt, atan2, ...}`，内部用 `libm`（纯 Rust 实现）。`sqrt` 在 IEEE 754 下本来就正确舍入，但为了规则简单，也走封装层。整数次幂 `powi` 同样走封装层（实现为确定的连乘或 `libm::pow`，在 `cetus-math` 中固定一种）。
- **怎么检查**：`clippy.toml` 的 `disallowed-methods`（见 §4）。

### D3 不用融合乘加和任何“快速数学”

- **为什么**：`mul_add` 只舍入一次，与先乘后加的结果不同；不同平台是否融合也不同。relaxed-simd、fast-math 允许编译器改变运算顺序。
- **怎么做**：不调用 `f64::mul_add`；不开启 WASM 的 relaxed-simd；`wasm-opt` 不使用会改变浮点语义的选项（例如 `--fast-math`）；不使用 `-C target-cpu=native`。
- **怎么检查**：`disallowed-methods` 禁用 `mul_add`；构建脚本中的编译选项写进 ADR-0001 并在 CI 中固定。

### D4 单次仿真单线程

- **为什么**：并行归约的求和顺序不确定，浮点加法不满足结合律。
- **怎么做**：一次仿真的推进只在一个线程里完成。并行只出现在“多次独立仿真”之间（批跑、平行星球、V2 扫描）。不在子系统内部使用 `rayon`。
- **怎么检查**：内核与范式 crate 不依赖 `rayon`（`cargo deny` 或 CI 中检查依赖树）。

### D5 内核里没有真实时间和系统随机源

- **为什么**：真实时间和操作系统随机数每次都不同。
- **怎么做**：内核不调用 `SystemTime::now`、`Instant::now`、`rand::thread_rng`、`rand::random`、`getrandom`。真实时间只存在于 TS 层（FSRS 调度、时间额度、活跃在线时间计算）。TS 层要影响世界时，只能生成带时间戳的 `Action`（例如 `advance_world { seasons: 0.25 }`、注入请教事件），由内核在宏步边界执行并写入操作日志。
- **怎么检查**：`disallowed-methods`；评审时检查任何新增的 `Action` 是否由 TS 层产生、是否进了操作日志。

### D6 时间用整数刻度

- **为什么**：`t += dt` 累加会积累舍入误差，并且不同的执行路径（例如快照后恢复）会得到不同的 `t`。
- **怎么做**：`t = t0 + (tick as f64) * quantum`，每次用乘法算出。`tick` 用 `u64`。多速率子系统的周期与宏步长之比必须是有理数，换算到公共的整数刻度。
- **怎么检查**：评审时搜索对时间变量的 `+=`。

### D7 随机数：每个实体一条具名子流

- **为什么**：如果所有实体共享一条随机流，新增一个实体、或某个实体多抽一次，就会改变其他所有实体之后的随机数。
- **怎么做**：子流种子由 `blake3` 派生：`derive(global_seed, subsystem_id, purpose, entity_id)`，每条子流是 `ChaCha8Rng`。`purpose` 用指南 §5.9 中登记的名字（`activation`、`rule`、`init`、`observation_noise`、`breakthrough`、`story`、`consult_variant` 等）；需要新的 `purpose` 时，先在指南中登记。洗牌（例如 `random_sequential` 激活）用本宏步的 `activation` 子流做 Fisher–Yates。
- **怎么检查**：扰动测试（§3.3）。

### D8 迭代顺序和排序必须是全序

- **为什么**：按浮点值排序时遇到相等或 NaN，结果依赖排序算法的细节。
- **怎么做**：agent 按稳定 id 迭代；排序键用 `(主键, 稳定 id)` 这样的复合键，浮点主键用 `f64::total_cmp`。事件队列按 K8 排序键 `(tick, sub_time, delta, priority, seq)` 排序，`seq` 是插入序号，保证没有平局。物体、组件的插入顺序由模型 schema 的声明顺序决定。
- **怎么检查**：评审时检查所有 `sort_by` / `sort_unstable_by` 的比较函数；同时事件测试（指南 M6）。

### D9 状态哈希只看规范化的位

- **为什么**：`-0.0` 与 `0.0` 数值相等但位不同；WebAssembly 中 NaN 的载荷位不保证一致。
- **怎么做**：状态哈希对规范化状态计算：浮点数用 `to_bits()`，计算前把 `-0.0` 规范化为 `0.0`，所有 NaN 规范化为同一个位模式。仿真中出现 NaN 本身就是错误（E4001），应当报错，而不是被哈希掩盖。仪器缓冲区不参与哈希。
- **怎么检查**：哈希函数有单元测试，覆盖 `-0.0` 和多种 NaN。

### D10 整数溢出行为要明确

- **为什么**：Rust 在 debug 构建中整数溢出会 panic，在 release 构建中默认回绕，native 测试和 WASM 发布版可能走不同的分支。
- **怎么做**：仿真中可能溢出的整数运算显式使用 `checked_*`（溢出即报错）或 `wrapping_*`（确实需要回绕时）。或在 workspace 的 `Cargo.toml` 中对 release 也开启 `overflow-checks = true`，写进 ADR。
- **怎么检查**：评审时检查计数器、刻度、id 分配。

### D11 浮点数的解析与打印用标准库

- **为什么**：自己写的解析和打印很难做到正确舍入。
- **怎么做**：用 Rust 标准库的 `str::parse::<f64>` 和 `Display`（最短往返表示）。不要用格式化后的字符串做比较或哈希。

### D12 快照必须完整

- **为什么**：快照漏掉任何一样东西，恢复后的仿真就会分叉。
- **怎么做**：快照包含内核版本、schema 版本、各模型 `id@version` 与内容哈希、当前刻度、各子系统状态、**随机子流状态**、事件队列（含 `seq` 计数器）、待执行操作队列、世界层状态（含原住民节点集合、研究进度、时间额度相关的世界侧状态）。格式为 `postcard` + `lz4_flex` + 头部（魔数、格式版本、校验和）。
- **怎么检查**：快照往返测试（§3.2）。

### D13 Python（Pyodide）

- **为什么**：学习者代码、CI 验收都要在同一版本的 Python 环境中得到同样的结果。
- **怎么做**：Pyodide 版本锁定；CI 中用同一版本的 Pyodide（在 Node 中运行），不用本机 CPython；加载时 `PYTHONHASHSEED=0`；`random` 由内核下发的种子初始化；`time`、`datetime` 中依赖真实时间的函数替换为确定性的桩函数；Python 只在宏步边界被调用（K10）。
- **怎么检查**：同一段玩家代码在浏览器和 CI 中的轨迹哈希相同（指南 M8 退出标准）。

## 3. 测试配方

### 3.1 跨目标哈希一致

每个样板模型和已发布模块：用 native 和 WASM（Node 或 wasmtime）分别运行固定步数，比较状态哈希。

```rust
// crates/cetus-kernel/tests/cross_target.rs（示意）
// 同一份测试代码分别编译为 native 和 wasm32 运行，各自把哈希写到输出，
// 由 CI 脚本比较两边的输出文件是否完全相同。
#[test]
fn golden_hashes() {
    for case in cetus_testkit::sample_models() {
        let plan = cetus_testkit::compile(&case.path).expect("compile");
        let mut sim = cetus_kernel::Sim::new(&plan, case.seed);
        sim.step(case.steps).expect("run");
        println!("{} {}", case.id, sim.state_hash());
    }
}
```

CI 比较 native 输出和 WASM 输出；不一致时打印第一个分歧的模型 id。

### 3.2 快照往返

快照 → 恢复 → 再跑 N 步，必须与不打断直接跑 N 步逐位一致。用 proptest 随机选择打断点和 N。

```rust
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn snapshot_roundtrip(split in 0u64..500, rest in 1u64..500, seed in any::<u64>()) {
        let plan = cetus_testkit::compile_sample("energy.solar-battery").unwrap();

        let mut a = cetus_kernel::Sim::new(&plan, seed);
        a.step(split + rest).unwrap();

        let mut b = cetus_kernel::Sim::new(&plan, seed);
        b.step(split).unwrap();
        let bytes = b.snapshot();
        let mut c = cetus_kernel::Sim::restore(&plan, &bytes).unwrap();
        c.step(rest).unwrap();

        prop_assert_eq!(a.state_hash(), c.state_hash());
    }
}
```

### 3.3 扰动测试（随机子流独立性）

在模型中加入一个与其他实体无关的 agent（或子系统），其他实体的轨迹必须逐位不变。

```rust
#[test]
fn unrelated_agent_does_not_perturb_others() {
    let base = cetus_testkit::compile_sample("abm.village").unwrap();
    let extra = cetus_testkit::with_extra_isolated_agent(&base);

    let traj_base = cetus_testkit::trajectories(&base, 42, 300);
    let traj_extra = cetus_testkit::trajectories(&extra, 42, 300);

    for (id, t) in &traj_base {
        assert_eq!(Some(t), traj_extra.get(id), "agent {id} was perturbed");
    }
}
```

### 3.4 操作日志回放

记录一段带玩家操作和世界时间推进 `Action` 的运行，从头回放，最终状态哈希必须相同。这同时验证 TS 层产生的 `Action`（请教事件注入、`advance_world`）都进了日志。

### 3.5 同时事件

构造在同一 `tick`、同一 `sub_time` 的多个事件，确认处理顺序只由 `(delta, priority, seq)` 决定，并且在 native 和 WASM 下相同。

（`cetus_testkit` 是建议的测试辅助 crate，名称和接口在 M1 中确定。）

## 4. `clippy.toml` 起点

以下是起点，**路径写法以 M0 时 clippy 的官方文档为准**，确认后写进 ADR-0001。特别是浮点固有方法在 `disallowed-methods` 中的路径写法，需要实际验证能否命中。

```toml
disallowed-types = [
  { path = "std::collections::HashMap", reason = "nondeterministic iteration; use IndexMap or BTreeMap (P2, D1)" },
  { path = "std::collections::HashSet", reason = "nondeterministic iteration; use IndexSet or BTreeSet (P2, D1)" },
]

disallowed-methods = [
  { path = "f64::sin",     reason = "use cetus_math (D2)" },
  { path = "f64::cos",     reason = "use cetus_math (D2)" },
  { path = "f64::tan",     reason = "use cetus_math (D2)" },
  { path = "f64::exp",     reason = "use cetus_math (D2)" },
  { path = "f64::ln",      reason = "use cetus_math (D2)" },
  { path = "f64::log10",   reason = "use cetus_math (D2)" },
  { path = "f64::log2",    reason = "use cetus_math (D2)" },
  { path = "f64::powf",    reason = "use cetus_math (D2)" },
  { path = "f64::powi",    reason = "use cetus_math (D2)" },
  { path = "f64::sqrt",    reason = "use cetus_math (D2)" },
  { path = "f64::atan2",   reason = "use cetus_math (D2)" },
  { path = "f64::mul_add", reason = "fused multiply-add changes rounding (D3)" },
  { path = "std::time::SystemTime::now", reason = "no wall clock in the kernel (D5)" },
  { path = "std::time::Instant::now",    reason = "no wall clock in the kernel (D5)" },
  { path = "rand::thread_rng",           reason = "use derived ChaCha8 substreams (D5, D7)" },
  { path = "rand::random",               reason = "use derived ChaCha8 substreams (D5, D7)" },
]
```

完整的超越函数列表（`asin`、`acos`、`atan`、`sinh`、`cosh`、`tanh`、`exp2`、
`exp_m1`、`ln_1p`、`cbrt`、`hypot` 等）已在 M0 补齐。当前实际清单见仓库根
`clippy.toml`：77 条中 60 条 `std` 规则由 `tools/clippy-probe` +
`scripts/check-clippy-rules.sh` 持续验证；`rand` / `getrandom` 的 17 条带
`allow-invalid = true`，其依赖层面的防线是 `scripts/check-no-os-rng.sh`。

## 5. PR 检查清单

提交或评审涉及仿真状态的 PR 时，逐条确认：

- [ ] 没有新增 `HashMap` / `HashSet`；如有 `allow`，有注释说明不影响状态。
- [ ] 超越函数都走 `cetus-math`；没有 `mul_add`。
- [ ] 没有在仿真推进中引入并行。
- [ ] 内核中没有真实时间和系统随机源；TS 层对世界的影响都通过 `Action` 并写入操作日志。
- [ ] 时间由整数刻度算出，没有累加。
- [ ] 新的随机用途有具名子流，`purpose` 已在指南中登记。
- [ ] 所有排序是全序（`total_cmp` + 稳定 id）。
- [ ] 新增的状态已加入快照和状态哈希。
- [ ] 跨目标哈希、快照往返、扰动测试都通过；新增模型已加入跨目标测试列表。
- [ ] 编译选项没有变化；如有变化，已写 ADR。

## 6. 常见错误

| 写法 | 问题 | 改法 |
|---|---|---|
| `for (k, v) in &hash_map { total += v }` | 迭代顺序不定，浮点求和顺序不定 | `IndexMap` 或 `BTreeMap` |
| `x.sin()` | 平台数学库可能不同 | `cetus_math::sin(x)` |
| `a * b + c` 被改写成 `a.mul_add(b, c)` “提升精度” | 舍入不同 | 保持 `a * b + c` |
| `self.t += self.dt` | 舍入误差累积，快照恢复后不一致 | `t = t0 + tick as f64 * quantum` |
| 所有 agent 共用一个 `rng` | 新增 agent 会扰动所有人 | 每个 agent 一条派生子流 |
| `v.sort_by(\|a, b\| a.x.partial_cmp(&b.x).unwrap())` | 遇到 NaN 会 panic，相等时顺序依赖算法 | `v.sort_by(\|a, b\| a.x.total_cmp(&b.x).then(a.id.cmp(&b.id)))` |
| 快照里没存随机子流状态 | 恢复后随机序列从头开始 | 子流状态必须进快照 |
| 在 TS 层直接修改世界状态（例如“玩家回来了，给原住民加点研究进度”） | 绕过操作日志，回放不一致；也违反 P11 和 §3.7 | 只能通过 `Action`，并且世界侧规则决定其效果 |
| 用 `format!("{:.6}", x)` 的结果做比较或哈希 | 丢失精度，掩盖差异 | 比较 `to_bits()` |
| CI 用本机 CPython 跑 Python 课程的验收 | 与浏览器中的 Pyodide 结果可能不同 | 用同一版本的 Pyodide |
