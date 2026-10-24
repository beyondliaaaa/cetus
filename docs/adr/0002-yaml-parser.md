# ADR-0002：YAML 解析库

- 状态：已接受（2026-10-23，M0）
- 相关：§4.4、§5.3、§5.4
- 取代：无

## 背景

模型（`module.yaml`）、课程、世界、连接器、参数库全部是 YAML。解析库必须满足：

1. **仍在维护**，有安全更新；
2. 能保留映射的**声明顺序**（确定性 D1 要求：反序列化到 `IndexMap`，遍历顺序 = 文件顺序）；
3. 能给出**源码位置**（行、列），用于 §5.4 的诊断码与 `miette` 渲染；
4. 允许严格模式：未知字段报错（作者写错字段名时必须失败，而不是静默忽略）；
5. 能编译到 `wasm32`（作者预览在浏览器中运行）。

`serde_yaml` 已归档，排除。

## 选项

| 库 | 维护状态 | 顺序保留 | 位置信息 | 未知字段严格 | 备注 |
|---|---|---|---|---|---|
| `serde_yaml_ng` 0.10 | 活跃（`serde_yaml` 的社区 fork） | 是（`IndexMap`） | `Location`（行/列） | 是（`deny_unknown_fields`） | API 与 `serde_yaml` 兼容，迁移成本低 |
| `serde-saphyr` 2.0 | 活跃，较新 | 是 | 有 span | 是 | 设计更现代，但生态与文档较新 |
| `yaml-rust2` | 活跃 | 是 | 有 | 需自行实现 serde 层 | 不是 serde 原生，工作量大 |

## 决定

使用 **`serde_yaml_ng` 0.10**。

理由：

- 它是 `serde_yaml` 的直接维护分支，API 稳定，M1 的 `cetus-model` 可以直接用
  `#[serde(deny_unknown_fields)]` 与 `serde_yaml_ng::Value`；
- `Location` 足以支撑诊断码的 `span { line, col }`；
- 纯 Rust，可编译到 `wasm32`；
- 与 `indexmap` 的 serde 支持配合，能保留声明顺序。

`serde-saphyr` 作为备选记录在案：如果 M1 中发现 `serde_yaml_ng` 的 span 精度不足
（例如无法定位到嵌套映射的具体键），再评估迁移。

## 约束

- 反序列化的所有映射类型使用 `IndexMap`（模型顶层字段顺序会影响 IR 布局与黄金轨迹）。
- 所有 schema 结构体加 `#[serde(deny_unknown_fields)]`，除非该区块明确允许扩展字段。
- 解析错误必须带 `file`、`line`、`col`，转换为 §5.4 的诊断码 E0xxx。
- 不在运行时解析 YAML：YAML 只在构建内容包（`cetus bundle`）与作者预览时解析；
  玩家端加载的是预编译的内容包。M2 实验台在浏览器中解析 YAML，是一个有范围和期限
  （至 M5）的例外，见 [ADR-0005](0005-runtime-yaml-in-workbench.md)。

## 后果

- 需要为每个 schema 区块写一个「最小示例 + 故意写错字段」的测试，验证严格模式与
  错误位置。
- 未来若 YAML 规范出现安全问题（别名炸弹等），升级路径明确（本 ADR 更新 + 重新锁定）。
