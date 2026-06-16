# smagicalssh

Rust 桌面 SSH 工作台。

## 当前仓库结构

当前仓库已经初始化为 workspace，并拆成两个顶层 crate：

- `crates/smagical-core`
  - 无 UI 依赖
  - 负责领域模型、核心状态、服务接口
- `crates/smagical-ui`
  - 依赖 `smagical-core`
  - 负责桌面入口、Slint 界面与展示层

## 当前模块分层

`smagical-core`：

- `domain/`
- `state/`
- `services/`

`smagical-ui`：

- `app/`
- `desktop/`
- `presentation/`
- `ui/`

当前只是第一版 workspace 骨架，后续新增功能默认继续往这两个 crate 内聚：

- 核心能力进 `smagical-core`
- 桌面装配、回调、展示进 `smagical-ui`

## 运行

```bash
cargo run -p smagical-ui
```

## i18n

当前 UI 文案使用 Slint 的 `@tr(...)` 标记，并使用 `slint-tr-extractor` 提取到 gettext `.po` 文件。

当前文案文件：

- [crates/smagical-ui/messages.po](/F:/code/rust/smagicalssh/crates/smagical-ui/messages.po)

当前提取工具版本：

- `slint-tr-extractor = 1.16.1`
