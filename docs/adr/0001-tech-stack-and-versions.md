# ADR-0001：技术选型与版本锁定

- 状态：已接受（2026-10-23，M0）
- 相关：§4.4、§5.12、`skills/cetus-determinism`
- 取代：无

## 背景

M0 需要把技术栈固定下来，让后续所有 crate、前端、CI 和验收都在同一组版本上工作。
其中三项对**确定性（P2）**有直接影响，必须在 CI 中锁定：

1. 编译器与目标平台（`rustc` 版本、`wasm32-unknown-unknown`、优化选项）；
2. `wasm-bindgen` CLI 与 crate 版本必须完全一致，否则生成的绑定与运行时 ABI 不一致；
3. `wasm-opt` 的优化选项不能改变浮点语义（禁止 `--fast-math`、relaxed-simd）。

## 选项

### 编译器

- **A. 固定具体 patch 版本**（`rust-toolchain.toml` 写 `1.99.1`）
- B. 跟随 stable 通道（`channel = "stable"`，每次 CI 取最新）

### WASM 构建

- **A. `cargo build --target wasm32-unknown-unknown` + `wasm-bindgen` CLI + `wasm-opt`**
- B. `wasm-pack`
- C. Trunk / vite-plugin-wasm

### 浮点与依赖

- **A. 全部超越函数走 `libm`（纯 Rust），clippy 禁止标准库浮点方法**
- B. 使用标准库 `f64::sin`（平台数学库可能不同）
- C. 使用 `fast-math`（不同平台结果必然不同）

## 决定

采用 **A / A / A**。

### 锁定版本（M0）

| 组件 | 版本 | 位置 |
|---|---|---|
| Rust | `1.99.1`（2026-10-13） | `rust-toolchain.toml` |
| 目标 | `x86_64-unknown-linux-gnu`、`wasm32-unknown-unknown` | `rust-toolchain.toml` |
| wasm-bindgen | crate `0.2.129` / CLI `0.2.129` | `Cargo.toml`、`scripts/build-wasm.sh` |
| binaryen / wasm-opt | `version_123` | `scripts/build-wasm.sh` |
| serde | `1.0.229` | `Cargo.lock` |
| libm | `0.2.16` | `Cargo.lock` |
| blake3 | `1.8.7` | `Cargo.lock` |
| clap | `4.6.7` | `Cargo.lock` |
| indexmap | `2.14.2` | `Cargo.lock` |
| proptest | `1.12.0` | `Cargo.lock` |
| rand / rand_chacha | `0.10.3` / `0.10.0`（`rand` 关闭默认特性，只开 `std`） | `Cargo.lock` |
| postcard | `1.1.3` | `Cargo.lock` |
| lz4_flex | `0.14.0` | `Cargo.lock` |
| thiserror | `2.0.21` | `Cargo.lock` |
| schemars | `1.2.3` | 尚未使用（M5 导出 schema 时锁定） |
| miette | `7.6.0` | 尚未使用（M2 富诊断渲染时锁定） |
| Vite | `7.3.7` | `package-lock.json` |
| React | `19.2.x` | `package-lock.json` |
| TypeScript | `5.9.3` | `package-lock.json` |
| Comlink | `4.4.2` | `package-lock.json` |

版本升级规则：任何一项升级都必须更新本 ADR 的表格，并在 PR 中附上跨目标哈希对比结果
（`scripts/check-cross-target.sh` 通过）。

### 编译器固定到 patch 版本

`rust-toolchain.toml` 写死 `1.99.1`。理由：

- 浮点运算、优化器与 `libm` 的行为可能在 minor 升级中改变，从而改变状态哈希；
- 验收（native）与玩家（wasm）必须用同一份编译器，否则 P4 的“同一份内核”名存实亡；
- 升级是有意的动作，需要重跑全部黄金轨迹并记录理由。

升级流程：修改 `rust-toolchain.toml` → 重跑 `scripts/check-cross-target.sh` 与全部
`cargo test` → 在 PR 中说明黄金轨迹是否有变化 → 更新本 ADR。

### 不使用 wasm-pack

`wasm-pack` 已随 rustwasm 组织于 2025 年归档。`wasm-bindgen` 迁移到新组织继续维护。
`scripts/build-wasm.sh` 直接调用 CLI，并在构建前校验 crate 版本与 CLI 版本一致
（不一致直接失败，避免产出 ABI 不匹配的包）。

### wasm-opt 选项

只使用 `-O2`。禁止：

- `--fast-math`：允许重排浮点表达式；
- relaxed-simd / `-msimd128` 中会改变浮点语义的部分；
- `-C target-cpu=native`（native 侧也不使用）。

### clippy 确定性规则

`clippy.toml` 的 `disallowed-types` / `disallowed-methods` 已在 M0 实际验证生效：

- `std::collections::HashMap` / `HashSet` → 命中；
- `f64::sin`、`f64::mul_add` → 命中。

验证方法：临时在 `crates/cetus-testkit/tests/` 放一个使用上述 API 的文件，运行
`cargo clippy -p cetus-testkit --tests`，确认出现 `disallowed_types` /
`disallowed_methods` 警告后删除该文件。本 ADR 记录该验证已完成（2026-10-23）。

路径写法说明：`f64::sin` 这种「类型::方法」形式在 clippy 1.99 下可以直接命中
固有方法；未来 clippy 若改变路径解析规则，以当时的官方文档为准并更新本 ADR。

### `rand` 的默认特性被关闭

`rand` 0.10 的默认特性会引入 `getrandom`，而 `getrandom` 在
`wasm32-unknown-unknown` 上默认拒绝编译。内核不使用操作系统随机源（D5），
只用 `rand_chacha` 的 `ChaCha8Rng`，所以 workspace 中写：

```toml
rand = { version = "0.10", default-features = false, features = ["std"] }
```

M1 实测：这样可以在不启用 `getrandom` 的 `wasm_js` 特性的前提下编译到
`wasm32-unknown-unknown`。如果将来某处确实需要 `rand::rng()`，必须经过 ADR
评审，因为那会重新引入系统随机源。

### 溢出检查

`[profile.release] overflow-checks = true`。理由（D10）：debug 与 release 对整数
溢出的行为必须一致，否则 native 测试与 wasm 发布版可能走不同分支。

## 后果

- 构建时间略增（release 也做溢出检查）。
- 升级 Rust 或 wasm-bindgen 需要一个专门的 PR，并重跑黄金轨迹。
- CI 必须安装与 crate 版本一致的 wasm-bindgen CLI；`scripts/build-wasm.sh` 会在版本
  不一致时直接失败。
- 所有浮点超越函数必须经过 `cetus-math`；新代码若直接调用 `f64::sin` 会在 clippy
  阶段被拒绝。
