# 鲸鱼娘历险记（Cetus）

一个网页上的、由**可运行的科学模型**构成的持久小世界。玩家通过有引导的探究亲手重建
科学知识，学到的东西会变成世界里真实运转的设施。

> **初心：逆练应用心理学。** 许多以「骗氪」为核心的页游、手游，本质上是应用心理学的
> 集大成，但它们把机制用在让人花更多钱、更多时间上。本项目要「逆练」：用同样深的
> 心理学功力，服务于让人真正学会，并且离开游戏后生活更好。
>
> 详见 [`docs/IMPLEMENTATION_GUIDE.md`](docs/IMPLEMENTATION_GUIDE.md) §1.1、§3.7。

**非营利导向。** 目标是惠及尽可能多的人。

## 当前状态

**M0 — 基础设施（完成）**

- Cargo workspace、锁定工具链、确定性 clippy 规则（由 `tools/clippy-probe` 持续验证）；
- WASM 构建管线（`wasm-bindgen` + `wasm-opt`，版本校验）；
- Vite + React + TypeScript 前端骨架，WASM 运行在 Web Worker 中（Comlink）；
- CI：fmt、clippy、clippy 规则探针、测试、跨目标确定性（native = wasm = 黄金值）、
  发布产物无 OS 随机源、web 构建与测试、素材许可证检查。

**M1 — 内核 v0（完成）**

- `cetus-units`：有理数指数的量纲、单位解析（`kW*h`、`W/m^2`、`mol/(L*s)`、
  分数指数）、SI 前缀、仿射单位（°C/°F）、角度标签、自定义量纲；proptest 覆盖
  往返与量纲代数。
- `cetus-expr`：Pratt 解析器、量纲推导、零字面量的量纲多态、栈机器求值、
  数值等价判定（返回反例）。`Complex` 等未来类型可以解析与类型推导，求值时按
  E1901 拒绝。
- `cetus-model`：`cetus.model/0.1` 的 SD 子集，严格拒绝未知字段。
- `cetus-compile`：名称/量纲/结构检查、代数环检测、流量量纲检查、拓扑排序、
  内容哈希；诊断码带路径与源码位置。
- `cetus-kernel`：状态布局、宏步推进、探针、不变量检查、边界钳制事件、
  具名随机子流、快照与状态哈希。
- `cetus-sd`：Euler 与 RK4 积分器；`dopri5`/`ros2` 尚未实现（M1.5/M6）。
- `cetus-cli`：`hello`、`check`、`run`、`sweep`、`explain`，全部支持 `--json`。
- **8 个样板模型**（`content/modules/`）：指数衰减、logistic 种群、SIR、
  捕食者–猎物、库存–订货延迟、RC 充放电、太阳能板–蓄电池、logistic 映射。
  有解析解的模型 nrmse 在 1e-10 量级（要求 < 1e-4）。
- **跨目标确定性**：8 个模型的 native、wasm 状态哈希与
  [`golden/hashes.txt`](crates/cetus-testkit/golden/hashes.txt) 三者一致。

下一个里程碑是 **M2 — 网页实验台**（在浏览器中加载模型、调参、运行、看图、
编辑方程）。里程碑与退出标准见指南 §12。

## 快速开始

依赖：

- Rust（`rust-toolchain.toml` 会固定版本）
- Node.js 22+
- `wasm-bindgen` CLI 0.2.129（必须与 crate 版本一致）
- `wasm-opt`（binaryen `version_123`；`scripts/build-wasm.sh` 会校验版本，不符即失败）

npm 默认走 npmmirror 镜像（`.npmrc`，方便国内的合作专家）。在其他地区，可以
`export npm_config_registry=https://registry.npmjs.org/` 改用官方源。`package-lock.json`
始终记录官方源地址；如果 `npm install` 把镜像地址写进了 lockfile，提交前运行
`npm run lockfile:fix`（CI 会检查）。

```bash
# 1. 构建 wasm 包并启动开发服务器
npm ci
npm run dev

# 2. 只跑引擎
cargo run -p cetus-cli -- hello
cargo run -p cetus-cli -- check content/modules/energy/solar-battery/module.yaml
cargo run -p cetus-cli -- run content/modules/energy/solar-battery/module.yaml \
    --until 72h --probe charge_pct --out /tmp/solar.csv
cargo run -p cetus-cli -- sweep content/modules/energy/solar-battery/module.yaml \
    --space "area=6..24:4,load=400..1600:3" --probe charge_pct --until 72h

# 3. 完整检查
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/check-clippy-rules.sh    # 60 条 std 确定性规则确实命中
./scripts/check-no-os-rng.sh       # 发布产物（cetus-wasm / cetus-cli）不含 getrandom
./scripts/check-cross-target.sh    # native = wasm = golden/hashes.txt
npm run lockfile:check
npm run build:wasm
npm run typecheck && npm test && npm run build --workspace @cetus/web
```

如果 `wasm-bindgen` 不在 PATH 中，`scripts/build-wasm.sh` 会直接报错并退出——这是
有意为之：CLI 与 crate 版本不一致会产出 ABI 不匹配的包。

## 仓库结构

```
crates/            Rust 引擎（M1 已实现：math、units、expr、model、compile、kernel、sd、
                   wasm、cli、testkit；其余为后续里程碑的占位 crate）
content/           内容，CC BY-SA 4.0（M5 前暂驻引擎仓库，之后拆分到 cetus-content）
apps/web/          Vite + React 前端
packages/          TypeScript 应用逻辑（目前只有 sim-client；lesson-runtime、mastery、
                   py-runner、ui-kit 随 M2–M8 加入）
docs/              实现指南、项目状态、ADR、诊断码说明
skills/            给 LLM 代理的技能包
assets/            美术与音频素材（每个文件都要在 LICENSES.yaml 登记）
scripts/           构建与检查脚本
tools/             CI 用的辅助 crate（clippy 规则探针）
```

## 设计文档

- 实现指南：[`docs/IMPLEMENTATION_GUIDE.md`](docs/IMPLEMENTATION_GUIDE.md)（**所有工作的
  最高依据**；§2 的硬约束优先级高于其他任何部分）
- 项目状态：[`docs/STATUS.md`](docs/STATUS.md)
- 诊断码索引：[`docs/errors/`](docs/errors/)
- 架构决策：[`docs/adr/`](docs/adr/)
- 确定性与内核规则：[`skills/cetus-determinism/SKILL.md`](skills/cetus-determinism/SKILL.md)
- 科技网络调研：[`skills/cetus-knowledge-network/SKILL.md`](skills/cetus-knowledge-network/SKILL.md)
- 误解调研：[`skills/research-misconceptions/SKILL.md`](skills/research-misconceptions/SKILL.md)
- 动机设计评审：[`skills/motivation-design-review/SKILL.md`](skills/motivation-design-review/SKILL.md)

## 许可证

- 代码：`MIT OR Apache-2.0`
- 内容（`content/`，M5 后为独立的 `cetus-content` 仓库）：`CC BY-SA 4.0`，见 [`content/LICENSE.md`](content/LICENSE.md)
- 素材：逐个记录在 [`assets/LICENSES.yaml`](assets/LICENSES.yaml)

见 [ADR-0003](docs/adr/0003-licenses.md)。
