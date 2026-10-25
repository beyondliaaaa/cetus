# 误解调研报告：蓄电池夜间耗尽（2026-10-26）

> research-misconceptions skill v0.1 · 对应指南 v0.5.2 · 草稿，待人类审阅
> 条目文件：`misconceptions/eng/pv-basics.yaml`

## 概要

- 本批节点：`kg:eng.energy.pv-basics`（课程 `lesson.energy.battery-night`）。顺带涉及两个尚未建立的节点：存量与流量入门、蓄电池充放电（ID 暂记为 `kg:TODO.*`）。
- 新增条目：4 条（research_documented 2、historical_theory 0、folk_belief 1、hypothesized 1）
- 可写成补丁：yes 3、partial 0、no 1
- 原住民是对的候选：1 条（`mc:eng.energy.pv-better-when-cold`）
- L1 层条目：3 条（满足“至少 1 条 L1”）

## 对现有课程误解的建议

| 现有误解 | 调研结果 | 建议 |
|---|---|---|
| `m_square`（功率与辐照度的平方成正比） | 没有找到出处 | 移出误解列表。它本质上是一个函数形式的变异体，V4 的变异测试（§8.5 第 3 项）已经会生成和检验这类错误，不需要单独列为误解。`t2` 因此没有对应的误解是有意的：指南只要求每个误解让它对应的任务失败，不要求每个任务都配一个误解 |
| `m_constant_sun`（把辐照度当作日均值） | 没有找到出处 | 保留，出处标为 `hypothesized`（对应 `mc:eng.energy.average-sun`） |
| `m_load_helps`（L1：负荷越大充得越快） | 没有找到出处 | 用 `mc:phys.em.battery-constant-source` 替换，拆成 L1 因果补丁（`{ c2: irrelevant }`，让 `l1-1` 失败）和 L2 模型补丁（`flows.discharging.eq` 固定 800 W，让 `t4` 失败）；`teaching.holds` 只能引用模型补丁。`t4` 的 load 是 1200 W，必须与 800 W 不同 |
| （新增）| — | 用 `mc:meta.modeling.stock-follows-inflow` 作为课程主误解，并作为 `teach-natives-1` 原住民所持误解的首选 |

替换后，`must_fail` 的大致对应：存量跟随流入的误解让 `t4`（预测耗尽时间）和 `t5`（轨迹拟合）失败；电池恒定输出的 L1 因果补丁让 `l1-1` 失败，L2 模型补丁让 `t4` 失败（`t4` 的 load 是 1200 W，补丁常数是 800 W）。具体以 V4 的实际运行结果为准。

## 值得注意的发现

- **存量-流量混淆是本课最好的误解**：研究结论稳健（理工科研究生也常犯），能写成补丁，判别实验又很自然：看电量的最低点出现在什么时刻。误解认为在夜里，真实情况是在日出之后。负荷越接近光伏峰值功率，两个时刻相差越远。这也让 `setup_space` 里负荷的范围有了设计依据。
- **这个误解可以贯穿多个节点**：水箱与延迟（M4 第 3 课）、舱体保温、种群都可以用“存量的峰谷出现在净流量过零时”这一条复习。适合做成一条贯穿的原住民故事线：同一个原住民在不同季节反复遇到“为什么最满的时候不是流进来最多的时候”。
- **补丁打在观测量上**：存量跟随流入的误解模型，最自然的写法是改观测方程而不积分，这会让误解模型的“电量”与存量脱钩。自洽性检查可以接受：它把误解模型当作真模型，用同一个 patched observable 生成原住民的预测和观测。课程任务应读仪器可测量的量；`t4` 已改为 `first_time(charge_pct <= 0)`，所以这个补丁会让 `t4` 和 `t5` 都失败。
- **“原住民是对的”的候选在本课用不上**：冷天发电更多需要温度系数，当前模块没有温度变量。可以作为小故事候选，但在专家补上教材、标准或同行评议综述之前不得使用；`confidence` 已降为 `low`。
- **设计规则**：判别“认为某样东西怎样”的误解时，任务应读取仪器实际测量的量（观测量/仪器可读量），不要读模型内部存量。否则误解只改观测量时，任务仍读真值，判别会失效。`t4` 从 `charge` 改为 `charge_pct` 就是这条规则的第一个应用；后续应写进 `design-misconceptions` 技能。

## 需要专家确认的问题

1. `mc:meta.modeling.stock-follows-inflow`：Booth Sweeney & Sterman 的会议版（preliminary results）与期刊版（initial results，SDR 16:249–286）应为同一研究，请核对后定一个引用。Cronin et al. (2009) 我只读到了摘要和二手转述。
2. `mc:phys.em.battery-constant-source`：原研究针对电路中的电流。把它迁移到“电池放电速度与负荷无关”是本调研的推论，请确认是否成立，或者给出储能情境下的直接研究。Engelhardt & Beichner 的正式发表信息（疑为 AJP 72(1), 2004）请核对。
3. `mc:eng.energy.average-sun`：是否有易错题或教材记录了“日均发电量够用就不会断电”这类错误？没有的话保留 `hypothesized`。
4. `mc:eng.energy.pv-better-when-cold`：需要一份比课程讲义更权威的出处（教材或标准），并给出晶硅组件典型的功率温度系数。
5. 各条目的 `prevalence`：除 Booth Sweeney & Sterman 外都没有可引用的数字。

## 实际查阅过的资料

- Booth Sweeney, L., & Sterman, J. D. (2000). *Bathtub Dynamics: Preliminary Results of a Systems Thinking Inventory*. 2000 International System Dynamics Conference（会议论文 PDF，全文）。https://proceedings.systemdynamics.org/2000/PDFs/sweeney4.pdf
- Cronin, M. A., Gonzalez, C., & Sterman, J. D. (2009). Why don't well-educated adults understand accumulation? *OBHDP*, 108(1), 116–130（只读了摘要）。https://ideas.repec.org/a/eee/jobhdp/v108y2009i1p116-130.html
- Aşık, G., & Doğança Küçük, Z. (2021). Metacognition in action as a possible explanation for stock-flow failure. *System Dynamics Review*, 37(4), 253–282（全文）。https://mural.maynoothuniversity.ie/15116/1/sdr.1692%283%29.pdf
- Engelhardt, P. V., & Beichner, R. J. *Students' understanding of direct current resistive electrical circuits*（arXiv 全文）。https://arxiv.org/pdf/physics/0304040
- Wainwright, C. L. *Toward Learning and Understanding Electricity: Challenging Persistent Misconceptions*（全文）。https://fg.ed.pacificu.edu/wainwright/Publications/MisconceptionsArticle.06.pdf
- McDermott, L. C., & Shaffer, P. S. (1992). Research as a guide for curriculum development… Part I. *Am. J. Phys.* 60(11), 994–1003（只读到了摘要页，未在条目中作为证据使用）。https://www.compadre.org/portal/items/detail.cfm?ID=2722
- Penn State AE 868, *Temperature and PV Performance Optimization*（课程讲义）。https://courses.ems.psu.edu/ae868/book/export/html/878
- Emery, K. et al. (1996). Temperature Dependence of Photovoltaic Cells, Modules, and Systems. 25th IEEE PVSC（只读到了摘要，未给出系数，未作为证据使用）。https://research-hub.nrel.gov/en/publications/temperature-dependence-of-photovoltaic-cells-modules-and-systems-5/

## 自检（§8）

- [x] 每条都有实际读过的出处，或已标为 `hypothesized` 并写明理由。只读了摘要的文献已在条目中注明。
- [x] 没有编造论文、作者、年份、网址或流行率。唯一的数字来自 Booth Sweeney & Sterman 会议论文全文；未核实的发表信息标注为“待核对”。
- [x] 没有大段抄录原文。
- [x] 没有把错误理论归属到现实中的民族、地区或宗教。
- [x] `natives_right: true` 的条目写清了边界；但出处只有一份课程讲义，`confidence` 已降为 `low`，在专家补上教材、标准或综述前不得使用（专家问题 4）。
- [x] 节点至少 1 条 L1（共 3 条）。
- [x] 每条都写了 `regime.diverges_when`。
- [x] `patchable: no` 的条目写了原因。
- [x] ID 无重复；带 `TODO` 的节点 ID 等科技网络骨架产出后替换。
