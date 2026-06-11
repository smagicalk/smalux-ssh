# smagicalssh

Rust 跨平台桌面 SSH 工作台，目标是参考并整合 XTerminal、Termora、Termius 的核心能力，提供一个以终端为中心的连接管理与远程运维工具。

## 项目定位

- 面向开发者和运维人员
- 跨平台桌面应用
- 以 SSH 为核心，扩展到 SFTP、端口转发、隧道、工作区和资产管理
- 优先采用纯 Rust 技术栈

## 当前状态

项目已经从“需求草案”进入“核心能力持续收敛”阶段，当前重点不是继续扩散功能点，而是把领域模型、存储结构、页面 view model 和 Slint 装配边界做稳。

已稳定接入的核心链路：

- SSH 连接、认证、交互式 shell、本地终端
- 远程命令执行、命令历史、最近连接
- SFTP 浏览、刷新、书签、传输任务
- 隧道规则保存、启动、停止、运行态管理
- Known Hosts 校验与信任管理
- 凭据元数据、本地秘密存储、证书/私钥/密码
- 主题、背景、设置页导入导出、SQLite 备份与快照
- 网络资产内核、运行隧道展示、代理资产和跳板链资产展示

## 当前网络模型

网络相关内核已经从“单代理”收敛到“两层结构”，纯模型集中在 `crates/smagical-core/src/network.rs`：

- 主机内联链路：
  - `Host.proxies: Vec<ProxyProfile>`
  - `Host.jumps: Vec<JumpProfile>`
- 可复用资产层：
  - `ProxyAsset`
  - `JumpChainAsset`
  - `ForwardAsset`
- 主机引用层：
  - `Host.network.proxy_ids`
  - `Host.network.jump_chain_ids`
  - `Host.network.forward_ids`

当前这两层都已经落到核心、内存存储、快照和 SQLite：

- `proxy_assets`
- `jump_chain_assets`
- `jump_chain_steps`
- `forward_assets`
- `host_network_proxies`
- `host_network_jump_chains`
- `host_network_forwards`

兼容策略：

- 旧 `proxy` 单值字段仍可反序列化为 `Host.proxies`
- 旧 SQLite `host_proxy` 单行结构会在连接迁移时自动修复为多行结构
- 删除主机时会同步清理跳板链资产中的失效主机引用，避免外键保存失败

当前边界：

- 主机已经具备网络资源引用字段，后续主机页需要补选择控件
- 网络页已有第一版资源库展示，可查看代理、跳板链、转发资产和运行中隧道
- 网络资产 CRUD、主机页选择入口、网络页全量 i18n 仍是后续工作

## 仓库结构

当前仓库已经是 workspace 形式，重点 crate 如下：

- `crates/smagical-core`: 纯领域模型
- `crates/smagical-storage`: SQLite / redb / 快照持久化
- `crates/smagical-backend-core`: 后端执行命令与事件协议
- `crates/smagical-local-backend`: 本地终端与本地执行
- `crates/smagical-ssh-client-core`: SSH 客户端执行层
- `src/app`: UI adapter、callback、projection、view model
- `ui/*.slint`: Slint 页面与组件

分层原则：

- `core` 只放纯数据模型和规则
- `storage` 只管落盘与导入导出
- `backend` 只管执行协议和运行态
- `app/view_model/projection` 负责把核心状态投影到 Slint
- `ui` 只负责页面与交互，不承载业务逻辑

## 已实现重点

### 主机与工作区

- 主机树、分组、搜索、复制、删除、编辑
- 本地终端与远程终端都支持多开
- 工作区标签、分屏和恢复链路已打通

### 凭据

- 私钥、证书、密码的创建、导入、导出、编辑、查看
- 凭据和主机解耦，主机只保存引用
- 凭据详情、分组树和本地秘密存储已落地

### 片段

- 已重构为“逻辑片段 + 脚本实现 + 支持目标”
- 一个逻辑片段可以有多个目标
- 多个目标可以共享一份脚本，也可以拆分独立实现

### 存储

- 正式主存储为 SQLite
- 支持 redb 旧数据导入
- 支持备份、快照导出、快照导入
- 已处理部分旧 schema 自动修复
- 网络资产已进入 SQLite 持久化和快照链路

### 网络

- 代理和跳板链已拆成不同概念
- 代理资产可以独立复用
- 跳板链资产保存有序主机链路
- 转发资产进入资源库，可以被主机多选引用
- 运行态隧道只暴露停止动作，已保存隧道模板暂不假装可直接启动
- 主机高级连接配置仍应回到主机级设置，不放进网络资产层

## 开发原则

- 模块化、功能化、单一职责
- 优先稳定核心，再做 UI 壳层
- 文案统一走 i18n 配置
- 不把业务规则写进 Slint
- 存储升级必须兼容旧数据

## 近期路线

1. 稳定网络引用模型：
   当前已经采用“主机引用资产 ID”的主路径，下一步补主机页选择代理、跳板链和转发资产。
2. 完善网络页功能：
   在现有资源库展示基础上补 CRUD、i18n 文案、空状态、选择态和引用占用提示。
3. 收敛主机页高级连接：
   保活、重试、连接复用放回主机级配置。
4. 继续减少 UI 与核心耦合：
   维持 `core -> storage -> view model -> projection -> slint` 的单向边界。

## 常用命令

```powershell
cargo fmt
cargo check --color never
cargo test --color never
cargo build --color never
```
