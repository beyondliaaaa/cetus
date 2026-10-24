# ADR-0005：实验台在浏览器中解析 YAML（ADR-0002 的例外）

- 状态：已接受（2026-10-24，M2 之后补记；M2 审查确认）
- 相关：ADR-0002「约束」、§4.2、§5.13、§12 M2、§12 M5
- 取代：无（ADR-0002 仍然有效，本 ADR 只界定一个有期限的例外）

## 背景

ADR-0002 约定：

> 不在运行时解析 YAML：YAML 只在构建内容包（`cetus bundle`）与作者预览时解析；
> 玩家端加载的是预编译的内容包。

M2 的网页实验台没有遵守这一条，而且当时没有记录：

1. `apps/web/src/catalog.ts` 用 `import.meta.glob("…/content/modules/**/module.yaml",
   { query: "?raw" })` 把 YAML 原文打进前端包；
2. Worker 调用 `compileModel(yaml)`，由 wasm 内核中的 `serde_yaml_ng` 在浏览器里解析；
3. `editModel` 修改后重新序列化为 YAML，再交给 `compileModel`（因此会丢失注释）；
4. `apps/web/src/lib/modelSource.ts` 用正则从 YAML 原文读取元数据中没有的两个字段
   （观测变量的方程、存量初值的原始表达式），供编辑器回填。

原因很实际：`cetus bundle` 和内容包格式要到 M5 才定（作者底座），而 M2 的退出标准
要求「SIR 和太阳能板–蓄电池两个模型可以在网页中**编辑**」。编辑的产物本来就是 YAML，
实验台在这个意义上就是 ADR-0002 允许的「作者预览」。

## 选项

- **A. 承认例外，界定范围和期限**（本 ADR）。
- B. 现在就实现 `cetus bundle` 和二进制内容包，实验台从内容包加载、编辑时在内存中改 IR。
  这需要提前定下 M5 的内容包格式，违反「不跨里程碑」（§0.2）。
- C. 把实验台降级为只读。不满足 M2 退出标准。

## 决定

采用 **A**。

**允许的范围**：实验台与作者预览（即今天的 `apps/web`）可以在浏览器中解析 YAML，
包括 `compileModel(yaml)`、`editModel` 和 `modelSource.ts`。

**不允许的**：

- 面向玩家的课程、世界、存档加载（M3 起的 `lesson-runtime`、M4 的垂直切片）不得
  依赖浏览器解析 YAML。它们在内容包出现之前，也要经过一个可替换的加载接口，
  不能直接 `import …?raw`。
- 不再新增像 `modelSource.ts` 这样的 TS 侧 YAML 读取逻辑。需要的字段应该由内核放进
  `metaJson`。

**期限**：M5 实现 `cetus bundle` 时：

1. 玩家端改为加载内容包，ADR-0002 的约束对玩家端恢复完全有效；
2. 实验台可以继续解析 YAML，但必须明确归为「作者预览」入口；
3. `modelSource.ts` 删除：`ModelMeta` 补上观测变量的 `eq` 和存量初值的原始表达式。

## 后果

- **确定性不受影响**：解析与编译在浏览器和 native 中是同一份 Rust 代码
  （`Model::from_yaml` + `compile_model`）。`scripts/check-cross-target.sh` 在 wasm 中
  通过实验台实际使用的 `compileModel` → `start` → `step` 路径跑全部样板模型，并与黄金哈希比对。
- **体积**：`serde_yaml_ng` 留在玩家端 wasm 中，直到 M5。当前 wasm gzip 约 290 KB，
  在 800 KB 预算内。
- **安全**：浏览器解析的只有仓库自带的样板模型和用户自己编辑的结果，不加载第三方 YAML。
  M10 起若要加载他人分享的模型，必须先有内容包和签名/哈希校验，不能沿用本例外。
- **已知限制**：编辑会丢失 YAML 注释；`modelSource.ts` 只认识样板模型使用的单行写法
  （见其文件头注释）。两者都记录在 `docs/STATUS.md`。
