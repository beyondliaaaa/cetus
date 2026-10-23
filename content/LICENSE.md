# 内容许可证 / Content licence

SPDX-License-Identifier: CC-BY-SA-4.0

`content/` 目录下的所有文件（模块、课程、fixture、文本、世界定义、数据集等）采用
**知识共享 署名-相同方式共享 4.0 国际许可协议（CC BY-SA 4.0）** 授权：

- 协议摘要：<https://creativecommons.org/licenses/by-sa/4.0/deed.zh-hans>
- 法律文本：<https://creativecommons.org/licenses/by-sa/4.0/legalcode.zh-hans>

这与仓库其余部分不同：引擎代码（`crates/`、`packages/`、`apps/`、`scripts/`、
`tools/`）采用 `MIT OR Apache-2.0`，见仓库根目录的 `LICENSE-MIT` 与
`LICENSE-APACHE`。决定与理由见 [ADR-0003](../docs/adr/0003-licenses.md)。

## 署名

署名为「The Cetus contributors」，并附上本仓库地址
<https://github.com/beyondliaaaa/cetus>。单个模块如有具体作者或审核专家，以该模块
`meta` 中的作者与来源字段为准（指南 §5.3）。

## 第三方材料

模块中引用的参数、数据与文献，必须在模块内注明来源（指南 §0.2、§9.2）。第三方数据集
的许可证必须与 CC BY-SA 4.0 兼容；不兼容的只能引用数值，不能随内容一起分发
（ADR-0003）。

## 迁移

`content/` 在 M5 拆分到独立的 `cetus-content` 仓库（指南 §4.3、§9.1）。届时该仓库
根目录放 CC BY-SA 4.0 全文，本文件随之迁移。

---

Everything under `content/` is licensed under **CC BY-SA 4.0**
(<https://creativecommons.org/licenses/by-sa/4.0/legalcode>), unlike the
engine code, which is `MIT OR Apache-2.0`. See ADR-0003.
