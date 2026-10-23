---
name: cetus-knowledge-network
description: Research, build, extend or audit the tech network (科技网络) of 鲸鱼娘历险记 / Cetus — domain taxonomy, node decomposition, layered prerequisites, capabilities, phase goals, the natives-support line and external resources.
---

# 科技网络调研与填充（鲸鱼娘历险记 / Cetus）

> **skill 版本**：v0.2（2026-10-23）· **对应指南版本**：v0.5.2
> 指南版本高于上面的数字时，先读指南的版本说明，确认本 skill 是否需要同步；发现冲突时以指南为准，并在调研报告中指出。
> v0.2 变更：`white_box` 改为 `provenance`；原住民在降落后第二天出场，脊柱调整；新增原住民支持线（原住民路线的主体）；节点新增 `consults`；调研时收集常见误解与历史上的错误理论。
>
> 本 skill 里的框架仍是**初稿**：领域分类、能力目录、第一阶段脊柱都需要你调研、验证和填充。你可以提出修改建议，但不要悄悄改动框架本身（见 §2 规则 R10）。

## 0. 你的任务与产出

你要为一个教育游戏搭建和填充它的**科技网络**：一张覆盖理工科与计算社会科学的有向知识网络。每个节点是一个“可检验的理解”，节点之间有先修和其他关系，节点为游戏世界提供“能力”，阶段目标由能力需求定义。

**产出**（全部是文本文件，放在内容仓库 `cetus-content/graph/` 下，格式见 §3）：
1. `graph/domains.yaml`：领域与子领域分类（在 §4.1 初稿的基础上验证和细化）。
2. `graph/capabilities/*.yaml`：能力目录（在 §4.2 初稿的基础上细化）。
3. `graph/goals/*.yaml`、`graph/spines/*.yaml`：阶段目标与主线脊柱（§4.3）。
4. `graph/nodes/<domain>/<subdomain>/*.yaml`：节点文件，每个文件一个节点，或者每个子领域一个文件（一个 YAML 列表）。
5. `graph/reports/<日期>-<批次>.md`：每批工作的调研报告（模板见 §8）。

**不属于你的任务**：编写仿真模型（module）、课程（lesson）、剧情台词、小故事、美术。这些由其他 skill 负责。你只负责网络本身。调研中遇到的常见误解和历史上的错误理论，简要记进 `research.notes`，详细整理交给 `research-misconceptions` skill。

## 1. 必读背景（最小上下文）

如果你能读到项目的实现指南（`docs/IMPLEMENTATION_GUIDE.md`），优先阅读 §1、§2、§3.6、§3.7、§7.11–§7.13、§9.3–§9.6。读不到时，以下是你需要知道的全部背景：

- **游戏**：一艘飞船降落在宜居星球时出了故障。鲸鱼娘机器人要探索星球、获取资源，修复通信和能源，并造出能驾驶飞船的子代理。第一阶段结局：子代理驾驶飞船返航，鲸鱼娘选择留下。第二阶段：星际通信，与原住民共同发展。
- **原住民**：Q 版，不一定是人形，体型小、寿命短、代际快（约 3–5 年一代）。鲸鱼娘降落后**第二天**就遇到他们，关系是合作互惠。原住民有**自己的科技发展**：在同一张科技网络上独立推进，玩家可以通过农业、健康、教育等方面的支持来加速他们。目前只设计一个部族，将来可能有多个。
- **学习方式**：有引导的探究。玩家用仪器观测“星球实测数据”，补全模型，预测，对照。学会的东西变成世界中真实运转的设施，也可以教给子代理和原住民。
- **初心**：“逆练”骗氪游戏的应用心理学，入门门槛要像大众手游一样低，上限达到大学水平。
- **四个表示层**（每个节点都按层设计）：

| 层 | 玩家做什么 | 前置知识 |
|---|---|---|
| L0 直觉层 | 直接操作世界中的对象，观察后果 | 无 |
| L1 因果层 | 画因果箭头，只标增加 / 减少 | 无 |
| L2 定量层 | 写带单位的公式、估计参数 | 初中到高中数学 |
| L3 推导 / 代码层 | 推导，或用 Python 写策略、算法 | 大学 |

- **主线必须能只用 L0–L1 走完**，更深的层带来效率和余量。
- **网络不是树**：先修可以是“与 / 或”组合，还有增强、应用、类比等软边。
- **能力**：世界可测量的本领（例如每日推进剂产量），分等级。节点在某层提供某项能力的某个等级。目标按能力需求定义，最小路径由程序计算。
- **来源（provenance）**：目标对每项能力可以限定来源。`player` = 玩家亲自掌握的节点（白盒）；`native_developed` = 原住民通过自己的研究发展出的技术。不限定时，黑盒（飞船残存系统等）也可以。第一阶段：**驾驶员只能是 `player`**；推进剂、推力、制导可以是 `player` 或 `native_developed`；其余不限。
- **原住民路线**：玩家亲自学得越少，就要等原住民发展越久。工作假设：主线约 24–50 个原住民年；只学驾驶员相关节点、但充分支持原住民约 60–100 年；最少投入约 2000 年（在实践中等不到）。所以**原住民支持线上的节点**（让原住民吃饱、健康、受教育、能做研究的那些理解）是第一阶段网络的正式组成部分。
- **黑盒默认供应**：世界永远有保底设施，学习是把黑盒换成白盒并改进，而不是从 0 到 1 的许可。

## 2. 硬规则

- **R1 不编造**：不得编造文献、网址、数值。没有核实过的内容标记 `TODO(expert)`，或写进 `research.todo_expert`。外部资源的网址必须是你实际访问并确认存在的；无法访问网络时，`url` 写 `"TODO"`，只写资源名称。
- **R2 可检验的理解**：`understanding` 用一句话写，以“能解释 / 能预测 / 能估算 / 能设计 / 能判断 / 能区分”开头，不写“了解”“熟悉”“掌握”。它必须能被一个检查器验证。
- **R3 粒度**：一个节点的每一层，都应该能在一次 10–15 分钟的会话中完成。超过就拆分；明显不到 5 分钟就合并。
- **R4 先修最小化**：`requires` 只写开始这一层**必需**的东西。“学了更好”的写进 `enhances`。L0 和 L1 的先修尽量为空，或只依赖其他节点的 L0–L1。
- **R5 稳定 ID**：`kg:<领域>.<子领域>.<短名>`，全小写，短名用连字符连接（例如 `kg:phys.mech.newton-third-law`）。发布后永不复用；改名时把旧 ID 放进 `aliases`。**创建节点之前，先搜索现有的 ID 和 aliases，避免重复。**
- **R6 无环**：每一层的 `requires` 构成有向无环图。发现环时，说明真正的依赖方向是什么；拆不开的，把其中一条改成 `enhances`。
- **R7 分区**：思辨性内容（假想学科）只能放在 `spec` 领域，`tier: speculative`，并且永远不能作为实证节点的先修。
- **R8 置信度**：每个节点都要填 `research.confidence`（`high` / `medium` / `low`）。`low` 的节点必须写明原因。
- **R9 语言**：`title` 和 `understanding` 必须有 `zh-CN`，鼓励同时写 `en`。
- **R10 框架变更走提案**：如果你认为领域分类、能力目录或脊柱需要修改，在调研报告的“框架修改提案”一节中写出理由，**不要直接修改**这些文件。
- **R11 能力之间的依赖不写成先修**：例如“电解制推进剂需要电力”，由世界模拟体现，不在网络中画边。
- **R12 不写教学内容**：不要写课程、题目、台词、小故事。可以在 `research.notes` 中记录对课程设计有用的发现，例如常见误解、历史上的错误理论、好的类比。
- **R13 原住民节点不写成“原住民专属”**：原住民和玩家共用同一张网络。不要为原住民另建一套节点；原住民支持线上的节点，是**玩家**要学的理解（例如“能预测粮食盈余如何随亩产和人口变化”），它们通过世界模型影响原住民。
- **R14 停下来问**：遇到以下问题时，写进报告的“需要人类决定的问题”，不要自行决定：能力等级的具体阈值（指南 Q19）；新手教程覆盖哪几节（Q31）；原住民的具体设定（Q11）；部族之间的关系（Q30）。

## 3. 数据格式

### 3.1 节点（`cetus.node/0.2`）

**完整节点示例**（调研充分后应达到这个程度）：

```yaml
schema: cetus.node/0.2
id: "kg:eng.energy.pv-basics"
aliases: []
title: { zh-CN: "光伏发电基础", en: "Photovoltaics basics" }
understanding:
  zh-CN: "能解释并预测光伏功率如何随面积、效率和光照变化"
  en: "Can explain and predict how PV power varies with area, efficiency and irradiance"
tier: applied                 # foundation | applied | social | meta | speculative
domain: eng.energy
layers:
  L0: { minutes: 10, understanding: { zh-CN: "能通过摆放和增减太阳能板，让夜间电量不归零" } }
  L1: { minutes: 10, understanding: { zh-CN: "能判断面积、光照、负荷分别如何影响充电和电量" } }
  L2: { minutes: 15, understanding: { zh-CN: "能写出 P = ηAG，并从数据中估计效率 η" } }
requires:
  L0: []
  L1: []
  L2:
    all:
      - "kg:math.func.proportionality@L2"
      - any: ["kg:phys.energy.power@L2", "kg:phys.energy.power@L1"]
enhances: [{ node: "kg:eng.energy.pv-tilt", note: "倾角决定入射辐照度" }]
applies_to: ["kg:eng.aero.spacecraft-power"]
analog_of: []
provides:
  - { cap: "cap:power.generation", levels: { L1: 1, L2: 2 } }
paradigms: [sd]
consults: []                  # 本节点的请教事件，由其他 skill 填写；你只需保留字段
external_resources:
  - { title: "TODO", url: "TODO", kind: course, lang: en, level: intro, free: true }
status: skeleton
trust: draft
research:
  confidence: medium
  notes: "常见误解：以为光伏功率与光照的平方成正比；以为阴天完全不发电"
  todo_expert: ["L2 的效率合理范围需要文献来源"]
  references: []              # 你实际查阅过的课程标准、教材、网页（只写确实读过的）
```

**骨架节点**（广度铺开阶段，最少只需这些字段）：

```yaml
schema: cetus.node/0.2
id: "kg:phys.thermo.ideal-gas-law"
title: { zh-CN: "理想气体定律" }
understanding: { zh-CN: "能预测密闭气体的压强如何随温度和体积变化" }
tier: applied
domain: phys.thermo
layers: { L0: { minutes: 10 }, L1: { minutes: 10 }, L2: { minutes: 15 } }
requires:
  L2: { all: ["kg:phys.thermo.temperature@L1", "kg:math.func.proportionality@L2"] }
status: skeleton
trust: draft
research: { confidence: medium, notes: "", todo_expert: [] }
```

**字段说明**
- `layers`：这个节点提供哪些层。L0、L1 尽量都提供；没有合理 L0 的节点（例如纯数学概念），说明原因并给出最接近的 L1。
- `requires`：按层声明，高层自动包含低层的要求。引用格式 `节点ID@层`。组合用 `all:` 和 `any:`，可以嵌套。
- `provides`：只有能为世界带来可测量本领的节点才写。基础学科节点通常不写 `provides`，而是通过全局修正起作用（§4.1 表中的“全局修正”一列）。原住民支持线上的节点，提供的是 `cap:natives.*` 能力（§4.2）。
- `paradigms`：这个节点的课程大概需要哪种仿真范式（取值见 §6.3）。不确定就留空。
- `consults`：请教事件的引用，不是你的任务，保留空列表即可。
- `external_resources.kind`：`course`、`video`、`interactive`、`textbook`、`app`、`article`。

### 3.2 能力（`cetus.capability/0.1`）

```yaml
schema: cetus.capability/0.1
id: "cap:propellant.production"
title: { zh-CN: "推进剂生产" }
description: { zh-CN: "把水电解成氢和氧，并能稳定地每天产出一定量的推进剂" }
metric: { expr: "daily_output(propellant)", unit: "kg/day_local" }
levels:
  - { level: 1, min: "TODO(expert)", note: "够用，但要等很久" }
  - { level: 2, min: "TODO(expert)" }
defaults: [{ source: ship.fabricator, level: 0 }]
```

能力文件不写“哪个目标要求什么来源”；来源要求写在目标里（§3.3）。

### 3.3 目标与脊柱

```yaml
schema: cetus.goal/0.1
id: "goal:phase1.return-launch"
title: { zh-CN: "子代理驾驶飞船返航" }
requires:
  - { cap: "cap:propellant.production", min_level: 1, provenance: [player, native_developed] }
  - { cap: "cap:autonomy.pilot",        min_level: 1, provenance: [player] }
  - { cap: "cap:structure.airframe",    min_level: 1 }     # 不写 provenance = 任何来源都可以
  # ……见 §4.3
budget: { nodes: [50, 100], hours_L0_L1: [12, 25], weeks: [6, 10], native_years: [24, 50] }
routes:
  - { id: mainline,       player_learns: all,        native_support: any,  native_years: [24, 50] }
  - { id: native-support, player_learns: pilot_only, native_support: full, native_years: [60, 100] }
  - { id: minimal,        player_learns: pilot_only, native_support: none, native_years_min: 2000 }
capstone: "sim:launch-campaign"
spine: "spine:phase1"
```

```yaml
schema: cetus.spine/0.1
id: "spine:phase1"
acts:
  - id: act0
    title: { zh-CN: "降落与能源审计" }
    trigger: "start"
    suggested: ["kg:eng.energy.pv-basics"]
```

`routes` 中的原住民年数由程序模拟校验，不是你要算的；你要做的是确保每条路线在网络上**存在**（§4.3 第 5 步）。

## 4. 框架初稿（需要你验证、细化、填充）

### 4.1 领域分类

| 前缀 | 领域 | 子领域（初稿） | 主要 tier | 全局修正（基础学科） |
|---|---|---|---|---|
| `math` | 数学 | `arith` 数与运算、`algebra` 代数、`func` 函数与关系（含比例）、`geom` 几何与三角、`calc` 微积分、`linalg` 线性代数、`ode` 微分方程与动力系统、`prob` 概率、`stats` 统计、`discrete` 离散数学、`optim` 优化、`numerics` 数值方法 | foundation | 子代理与原住民的协同效率 |
| `meta` | 方法论 | `sci-method` 科学方法、`modeling` 建模、`measurement` 测量与误差、`dimensional` 量纲分析、`learning` 学习科学、`motivation` 动机设计（“拆解模式”） | foundation / meta | 提示质量、探究效率 |
| `phys` | 物理 | `units` 单位、`mech` 力学、`energy` 能量与功率、`thermo` 热学与统计力学、`em` 电磁学、`waves` 振动与波、`optics` 光学、`fluids` 流体、`orbital` 天体与轨道、`modern` 近代物理（含半导体、相对论、量子入门）、`nuclear` 原子核 | foundation（基础部分）/ applied | 子代理数量上限 |
| `chem` | 化学 | `structure` 物质结构、`stoich` 化学计量、`thermochem` 热化学、`kinetics` 反应动力学、`equilibrium` 化学平衡、`electrochem` 电化学、`materials` 材料化学、`organic` 有机化学基础、`analytical` 分析化学 | applied | — |
| `bio` | 生命科学 | `cell` 细胞与分子、`genetics` 遗传、`evolution` 进化、`physiology` 生理、`ecology` 生态、`micro` 微生物、`neuro` 神经科学、`agri` 农业与食品科学、`biomech` 生物力学与尺度律、`health` 公共卫生与营养 | applied | — |
| `earth` | 地球与行星科学 | `planetary` 行星科学、`atmosphere` 大气与气候、`hydro` 水文、`geo` 地质、`soil` 土壤 | applied | — |
| `cs` | 计算机科学 | `programming` 编程、`algo` 算法与数据结构、`theory` 计算理论与自动机、`arch` 体系结构与数字逻辑、`networks` 网络、`systems` 分布式系统、`data` 数据与数据库、`ml` 机器学习、`info` 信息论、`crypto` 密码学 | foundation（`programming`）/ applied | 子代理的自主能力 |
| `eng` | 工程 | `energy` 能源工程、`electrical` 电气、`mech` 机械与结构、`materials` 材料工程、`chemeng` 化工、`control` 控制、`aero` 航空航天、`comms` 通信、`manufacturing` 制造、`robotics` 机器人与自主系统 | applied | — |
| `social` | 社会科学 | `psych` 心理学（认知、发展、社会）、`ling` 语言学、`edu` 教育学、`econ` 经济学（含增长与知识生产）、`games` 博弈论与机制设计、`netsci` 网络科学与社会学、`polisci` 制度与投票、`law` 法学（可形式化的部分）、`anthro` 人类学、`demog` 人口学 | social | 与原住民的沟通、信任、合作效率 |
| `spec` | 假想实验室 | 例如 `energy-money` 能量货币、`thermo-game-law` 热力学与博弈论的法学、`edu-production` 教育生产函数 | speculative | 不提供（只在沙盒中生效） |

v0.2 新增的子领域：`bio.health`、`social.demog`，以及 `social.econ` 明确包含增长与知识生产。它们是原住民支持线的主要来源，你可以在报告中提议调整。

**你要做的**：确认每个领域有 5–15 个子领域，每个子领域写一行范围说明，并标出与其他领域的主要交界（例如 `bio.biomech` 与 `phys.mech`）。

### 4.2 能力目录初稿

**第一阶段**（★ = 目标要求非黑盒来源；☆ = 只能是 `player`）

| 能力 ID | 名称 | 黑盒默认 | 目标中的来源要求 | 备注 |
|---|---|---|---|---|
| `cap:power.generation` | 发电 | 飞船备用太阳能板（等级 1） | 不限 | M4 垂直切片的起点 |
| `cap:power.storage` | 储能 | 飞船蓄电池（等级 1，已老化） | 不限 | |
| `cap:power.distribution` | 配电与负荷管理 | 飞船配电（等级 0–1） | 不限 | 第二阶段接非因果电路 |
| `cap:thermal.management` | 热管理（保温、散热） | 飞船舱体（等级 1） | 不限 | 也影响推进剂的低温储存 |
| `cap:water.supply` | 供水与净化 | 飞船滤水器（等级 1） | 不限 | 电解的原料 |
| `cap:food.supply` | 食物（白饭） | 飞船储备米（有限） | 不限 | 不在主线上，是鲸鱼娘的增益 |
| `cap:fabrication.parts` | 零件制造 | 飞船制造机（等级 1） | 不限 | |
| `cap:exploration.survey` | 勘探与测绘 | 无 | 不限 | 发现资源 |
| ★ `cap:propellant.production` | 推进剂生产 | 无 | player 或 native_developed | 电解水、化学计量 |
| `cap:propellant.storage` | 推进剂储存 | 无 | 不限 | 气体定律、低温、蒸发损失 |
| ★ `cap:propulsion.delta-v` | 推力与速度增量 | 无 | player 或 native_developed | 动量守恒、火箭方程 |
| `cap:structure.airframe` | 箭体结构 | 受损船体（等级 0） | 不限 | |
| ★ `cap:control.attitude` | 制导与姿态控制 | 无 | player 或 native_developed | 反馈、PID |
| `cap:navigation.launch-window` | 导航与发射窗口 | 无 | 不限 | 轨道入门 |
| `cap:comms.link` | 通信链路 | 受损通信系统（等级 0） | 不限 | 电磁波、天线 |
| ☆ `cap:autonomy.pilot` | 子代理驾驶员 | 无 | 只能 player | 传感、状态机、编程；原住民寿命太短，无法完成星际航行 |
| `cap:natives.communication` | 与原住民的沟通 | 无 | — | 语言学；第一次相遇起就需要 |
| `cap:natives.trust` | 与原住民的信任与互惠 | 无 | — | 心理学、博弈论 |
| `cap:natives.food-security` | 原住民的粮食盈余 | 原住民传统农业（低） | — | 原住民支持线：盈余释放出研究劳动力 |
| `cap:natives.health` | 原住民的健康 | 原住民传统医疗（低） | — | 原住民支持线：卫生、营养、传染病 |
| `cap:natives.education` | 原住民的教育 | 无 | — | 原住民支持线：教学方法、知识保存与传承 |
| `cap:natives.research` | 原住民的研究能力 | 无 | — | 原住民支持线：记录、测量、实验方法；直接影响原住民的研究速率 |

`cap:natives.*` 不在返航目标的 `requires` 里，但它们决定原住民研究的速度，是“充分支持原住民”路线的含义所在。

**第二阶段**（方向草案，只需列出，不必细化）：`cap:comms.interstellar`（星际通信）、`cap:power.grid`（电网，非因果电路）、`cap:power.thermal-plant`（热电厂，朗肯循环）、`cap:natives.governance`、`cap:space.orbital-industry`、`cap:energy.megascale`（戴森群）。

**你要做的**：为第一阶段每项能力设计 2–3 个等级。等级的阈值通常需要专家给出，写 `TODO(expert)`，但要附上你对量级的推理（例如“返航所需推进剂约为干重的 N 倍，按 X 天备齐，推出每日产量的量级”），并注明假设。`cap:natives.*` 的等级用原住民世界状态中可测量的量来定义（例如粮食盈余占比、识字或“能记录测量结果”的人口比例），具体由人类决定模型变量，你给出候选即可。

### 4.3 第一阶段目标与脊柱初稿

**目标**：`goal:phase1.return-launch`，需要的能力：
- 只能 `player`：`autonomy.pilot`，等级 1；
- `player` 或 `native_developed`：`propellant.production`、`propulsion.delta-v`、`control.attitude`，均为等级 1；
- 不限：`structure.airframe`、`navigation.launch-window`、`comms.link`，均为等级 1。

**预算**（工作假设）：50–100 个节点；L0–L1 共约 12–25 小时；6–10 周；主线期间原住民经历 24–50 年。

**脊柱草案**（每一幕列出**候选节点簇**，你要把它们拆成符合 R3 粒度的节点，设计按层先修，并核对预算）：

| 幕 | 标题 | 主要提供的能力 | 候选节点簇（待调研拆分） |
|---|---|---|---|
| 0 | 降落与能源审计（序章） | power.generation、power.storage | 测量与单位；能量与功率；存量与流量；光伏基础；蓄电池充放电 |
| 1 | 第二天：初遇原住民（序章） | natives.communication | 观察与描述；用演示代替语言（指物、动作、示范实验）；最初的词汇（命名博弈）；尺度律（他们为什么长这样） |
| 2 | 能源自给 | power.distribution、thermal.management | 负荷与优先级；功率预算；保温与散热（牛顿冷却）；太阳能板倾角（三角函数入门） |
| 3 | 水与食物 | water.supply（food.supply） | 水循环；过滤与净化；水箱与延迟；作物生长（logistic）；煮饭的能量（比热容、汽化潜热） |
| 4 | 推进剂 ★ | propellant.production、propellant.storage | 电解水；物质的量与化学计量（氢氧质量比）；化学能；理想气体定律；低温储存与蒸发损失 |
| 5 | 火箭原理 ★ | propulsion.delta-v | 牛顿第三定律；动量守恒；质量比与速度增量（火箭方程的 L1：边际递减）；推重比；多级火箭 |
| 6 | 箭体 | structure.airframe | 材料强度；压力容器；质量预算 |
| 7 | 控制与导航 ★ | control.attitude、navigation.launch-window | 反馈回路；过冲与振荡；PID 的直觉；重力与圆周运动；发射窗口 |
| 8 | 通信 | comms.link | 电磁波；天线与方向性；信号与噪声；链路预算的直觉 |
| 9 | 子代理 ☆ | autonomy.pilot | 传感器；状态机；流程与程序（L1 流程图，L3 Python）；不确定条件下的决策；测试与故障 |
| 终 | 结局演练 | 全部 | （不是节点：发射模拟） |

**原住民支持线**（不是一幕，贯穿第 1 幕之后的整个第一阶段；原住民路线的主体）：

| 方向 | 主要提供的能力 | 候选节点簇（待调研拆分） |
|---|---|---|
| 粮食 | natives.food-security | 亩产与投入；轮作与土壤养分；储粮与损耗；人口与承载力；盈余与分工 |
| 健康 | natives.health | 卫生与水源；营养；传染病的传播（SIR 直觉）；接种与群体免疫的直觉 |
| 教育 | natives.education | 演示与模仿；提取练习与间隔复习；知识的记录与传承；知识在社交网络上的扩散 |
| 研究 | natives.research | 计数与记录；测量与标准单位；控制变量的实验；知识积累的复利（增长理论的直觉） |
| 关系 | natives.trust | 互惠与重复博弈；公平分配；信任的建立与破坏 |

注意第 1 幕与新手教程的边界（哪几节属于按天推进的序章）还没有定（指南 Q31），你只需在报告中给出建议。

**你要做的**：
1. 从每项能力倒推：要在 L0–L1 达到等级 1，玩家**最少**需要理解什么？
2. 拆成节点，写出每层的 `requires`。
3. 估算主线最小路径的节点数和时长，与预算对照。超出预算时，提出取舍方案：哪些可以降到黑盒，哪些可以合并。
4. 标出主线之外、但能显著提升效率或余量的节点（`enhances` 指向主线节点）。
5. **路线存在性**：确认“只学驾驶员相关节点 + 原住民支持线”这条路线在网络上存在：驾驶员所需的节点有一条只用 L0–L1 的路径，并且不依赖推进剂、推力、制导这三项的白盒节点作为硬先修。如果驾驶员确实需要某些控制知识，把它们列出来，说明它们算“驾驶员相关节点”，并估算这条路线玩家要学的节点数和时长。
6. 估算原住民支持线的节点数和时长（L0–L1），写进报告。

### 4.4 跨领域类比（`analog_of`）初稿

这些类比是“宽墙壁”的关键：学会一个，另一个就有了迁移提示。请验证并扩充：
- 一阶系统：RC 电路充放电、牛顿冷却、药物代谢、蓄电池自放电、水箱泄流
- logistic 增长：种群、流行病早期、技术扩散、作物生长
- 振荡与反馈延迟：弹簧振子、LC 电路、库存–订货振荡、捕食者–被捕食者、蛛网模型
- 扩散：热传导、分子扩散、谣言与知识在网络上的传播、地下水
- 守恒与流：能量守恒、质量守恒、电荷守恒、货币存量与流量
- 最优化：最小作用量、市场均衡、最短路径、自然选择（作为类比，注意其局限）
- 复利与积累：存款利息、知识积累与研究速率、人口增长

## 5. 工作流程

按阶段进行。每完成一批，就写一份调研报告（§8）。一批的规模建议为：一个领域，或第一阶段的一幕，或原住民支持线的一个方向。

| 阶段 | 内容 | 产出 | 完成标准 |
|---|---|---|---|
| A 领域骨架 | 验证并细化 §4.1 | `domains.yaml` | 每个领域 5–15 个子领域，各有一行范围说明和交界标注 |
| B 第一阶段最小路径（**最优先**） | 按 §4.3 第 1–6 步，包括原住民支持线 | 第一阶段主线与原住民支持线的节点（完整格式）、能力等级草案、预算核对、路线存在性 | 每项能力都有至少一条只用 L0–L1 的路径；路线存在性已确认；预算核对结果写入报告 |
| C 第一阶段支线 | 效率提升、替代路线、食物线 | 节点（完整格式） | 每个支线节点都说明它增强哪个主线节点或能力 |
| D 全领域广度骨架 | 每个领域铺 30–80 个骨架节点，覆盖到大学入门课程的深度 | 骨架节点 | 每个子领域至少 3 个节点；跨领域先修已连上 |
| E 软边 | `enhances`、`applies_to`、`analog_of` | 更新节点文件 | §4.4 的每组类比都已落到具体节点 |
| F 外部资源 | 每个节点 1–3 个经过核实的资源 | 更新节点文件 | 网址都经过实际访问（R1）；优先选择免费、开放许可的资源 |
| G 自检与报告 | §7 清单 | 报告 | 清单全部通过，或在报告中说明例外 |

阶段 B 完成后先停下，等人类或专家审阅，再继续 C 之后的阶段。

## 6. 调研方法

### 6.1 参考框架（用于确定先修结构和覆盖范围；使用前请自行核实，只引用你确实读过的）

- 美国科学促进会（AAAS）2061 计划的《科学素养导图》（Atlas of Science Literacy）：用“概念发展链”画出科学概念之间的先修关系，非常接近本网络的形式。
- 美国《新一代科学标准》（NGSS）：学科核心概念与跨学科概念（例如“系统与系统模型”“能量与物质的流动”）。
- 中国《普通高中课程标准》中的数学、物理、化学、生物、信息技术等学科标准。
- ACM/IEEE-CS 的计算机科学课程体系（CS2023）。
- 大学入门课程的公开教学大纲（例如 MIT OpenCourseWare），以及标准教材的目录。
- 学科教育研究中的常见误解文献，例如物理教育研究中的“力的概念量表”（Force Concept Inventory）相关研究。
- 航天方面：航天机构面向公众的教育材料（例如 NASA 的火箭入门资料）。
- 原住民支持线：公共卫生、农业推广、发展经济学与教育学的入门材料。注意区分实证结论与政策主张。

### 6.2 拆分节点的方法

1. 从“玩家在世界中要能做到什么”出发（能力），倒推“需要理解什么”。
2. 对每个理解，先问：它的 L0 是什么（玩家能动手做什么）？L1 是什么（因果方向是什么）？想不出 L0 和 L1 的，通常说明节点太抽象，需要拆分或换个切入点。
3. 用 R3 检查粒度，用 R4 检查先修。
4. 对每条先修边，问自己：“没有它，玩家真的无法开始这一层吗？”答案是否定的，就改成 `enhances`。

### 6.3 范式取值（`paradigms` 字段）

`sd`（系统动力学 / ODE）、`reactions`（化学反应网络）、`blocks`（方块图与控制）、`acausal`（非因果物理网络：电路、热网络）、`discrete_map`（离散时间映射）、`sde`（随机微分方程）、`ssa`（离散随机模拟）、`lattice_mc`（格点蒙特卡洛）、`des`（离散事件）、`abm`（基于主体）、`ca`（元胞自动机）、`network`（网络动力学）、`pde`（网格场）、`lbm`（格子 Boltzmann）、`fem`、`wave_complex`（复数场与量子）、`rigid`（刚体）、`particles`（粒子与分子动力学）、`solve`（平衡与求解）、`optim`（优化与拟合）、`game`（博弈）、`geometry`（几何与光线）、`structure`（分子与晶体结构）、`algo`（算法轨迹）、`logic`（数字逻辑）、`automata`（自动机与形式语言）、`quantum_circuit`（量子线路）、`derivation`（推导与符号）、`proof`（证明）、`data`（统计与数据分析）、`human_subject`（玩家被试实验）。

### 6.4 顺手收集误解与历史上的错误理论

调研时遇到以下材料，简要记进节点的 `research.notes`（一句话 + 出处），供 `research-misconceptions` skill 后续整理：
- 学科教育研究中记录的常见误解；
- 历史上真实出现过、在当时的证据下合理的错误理论（例如热质说、燃素说、“自然厌恶真空”、自然发生说）；
- 民间做法中后来被证实**正确**的例子（只记录有可靠出处的）。

## 7. 自检清单

- [ ] 所有 ID 符合 R5，没有重复（包括与 aliases 重复）。
- [ ] 每一层的 `requires` 无环（R6）。可以写一个小脚本做拓扑排序来检查。
- [ ] 主线节点的 L0–L1 不依赖任何节点的 L2 及以上层。
- [ ] 每个 `applied` 节点至少有一条通向 `foundation` 节点的路径。
- [ ] 没有 `speculative` 节点被实证节点依赖（R7）。
- [ ] 被超过 30 个节点直接依赖的枢纽节点已在报告中列出，并给出是否拆分的建议。
- [ ] 第一阶段每项能力至少有一条只用 L0–L1 的路径。
- [ ] 驾驶员能力的路径不依赖推进剂、推力、制导这三项的白盒节点作为硬先修（或已在报告中说明例外）。
- [ ] 原住民支持线的四个方向（粮食、健康、教育、研究）都有节点，并标明各自提供的 `cap:natives.*` 能力。
- [ ] 第一阶段最小路径与“只学驾驶员 + 原住民支持线”路线的节点数和时长都已估算，并与预算对照。
- [ ] 每个节点都有 `understanding`（符合 R2）、`confidence`（R8）。
- [ ] 所有外部资源的网址都经过实际访问，或标记为 `"TODO"`（R1）。
- [ ] 没有编造任何文献或数值（R1）。

## 8. 调研报告模板

```markdown
# 科技网络调研报告：<批次名称>（<日期>）

## 概要
- 本批范围：
- 新增节点：N 个（完整 a 个，骨架 b 个）；修改节点：M 个
- 第一阶段最小路径估算（如涉及）：x 个节点，约 y 小时（L0–L1）；预算对照：
- “只学驾驶员 + 原住民支持线”路线估算（如涉及）：玩家要学 x 个节点，约 y 小时

## 主要发现
- 先修结构上的重要发现
- 粒度调整（拆分、合并）及理由

## 不确定之处与需要专家回答的问题
1. ……（指出涉及的节点 ID）

## 需要人类决定的问题（R14）
1. ……（例如对 Q31 新手教程范围的建议）

## 框架修改提案（R10）
- 提案：……；理由：……；影响：……

## 自检结果
- §7 清单逐项结果；例外及原因

## 实际查阅过的参考资料
- ……
```

## 9. 常见错误

- 把一整门课写成一个节点（例如“热力学”）。应该拆成“温度与热量的区别”“热平衡”“比热容”这样一次会话能完成的理解。
- 把“学了更好”写成硬先修，导致 L0–L1 的门槛被抬高。
- L0 写成“看一段动画”。L0 必须是玩家**亲手操作**并看到后果。
- 为了让网络“看起来完整”，编造外部资源或文献。宁可留 `TODO`。
- 把能力之间的依赖（例如“推进剂需要电”）画成先修边（违反 R11）。
- 擅自修改领域分类或脊柱（违反 R10）。
- 为原住民另建一套节点，或把原住民支持线当成可有可无的支线（违反 R13；它是原住民路线的主体）。
- 还在用旧版的 `white_box: true`。v0.2 起统一用 `provenance`。
