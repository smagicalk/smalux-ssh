# 🔐 08. 凭据保险库与安全认证中心 (Credentials Vault & Security Center)

---

## 📌 模块概述

凭据保险库（Credentials Vault）是 `smalux-ssh` 的核心安全资产管理中枢，负责统一管理 SSH 密钥对、登录口令密码、SSH Agent 代理转发通道与 X.509 证书。系统采用多层防护设计，严格贯彻**物理脱敏、敏感操作审计与防侧信道泄露**原则。

---

## 🏛️ 1. 双模式 UI 视图架构

```text
┌───────────────────────────────────────────────────────────────────────────┐
│                           凭据系统双模式视图体系                          │
├─────────────────────────────────────┬─────────────────────────────────────┤
│ 1. 侧边栏凭据抽屉 (CredentialsDrawer)│ 2. 全屏独立管理中心 (CredentialsView)│
│ • 常驻左侧活动栏二级抽屉 (快捷唤起) │ • 全屏主从双栏架构 (Master-Detail)  │
│ • 支持类型快速切换过滤 (密钥/密码等)│ • 深度表单编辑与多算法密钥生成器    │
│ • 敏感机密一键安全复制 (带审计追踪) │ • 强随机口令密码生成器与指纹计算    │
│ • 资产卡片类型徽标与用户名信息提示  │ • 敏感机密显隐切换 (防窥探设计)     │
└─────────────────────────────────────┴─────────────────────────────────────┘
```

---

## 🧩 2. UI 组件与文件结构

- **全屏管理视图**：[`crates/smagical-ui/ui/views/credentials_view.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/views/credentials_view.slint)
- **侧边栏抽屉组件**：[`crates/smagical-ui/ui/views/left_drawers/credentials_drawer.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/views/left_drawers/credentials_drawer.slint)
- **调试工作台选项卡**：[`crates/smagical-ui/ui/components/debug_workbench/debug-credentials-tab.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/debug_workbench/debug-credentials-tab.slint)
- **UI 路由与交互处理器**：[`crates/smagical-ui/src/handlers/credential_handlers.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/src/handlers/credential_handlers.rs)
- **核心领域模型**：[`crates/smagical-core/src/domain/credential.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-core/src/domain/credential.rs)

---

## 📐 3. 核心数据契约 (Data Contracts)

### 3.1 Slint UI 展示模型 (`CredentialItemData`)
```slint
export struct CredentialItemData {
    id: string,                 // 凭据全局唯一 ID (如 "cred-prod-root")
    name: string,               // 凭据可读名称 (如 "生产集群 Root 私钥")
    cred_type: string,          // 类型: "key" | "password" | "agent" | "certificate"
    cred_type_label: string,    // 类型可读徽标: "SSH 密钥" | "密码口令" | "SSH Agent" | "证书"
    username: string,           // 默认登录用户名 (如 "root", "ubuntu")
    algorithm: string,          // 算法规格 (如 "Ed25519", "RSA-4096", "ECDSA P-256")
    fingerprint: string,        // 公钥 SHA-256 指纹 (如 "SHA256:4a8b...f9e2")
    notes: string,              // 备注说明
    has_secret: bool,           // 是否包含私钥 PEM 或口令明文
    is_favorite: bool,          // 是否星标置顶
    updated_at: string,         // 最后修改时间戳 (ISO 8601)
}
```

### 3.2 凭据类别枚举 (`CredentialType`)
| 类型枚举 | 标识字符串 | 适用场景与安全特性 |
| :--- | :--- | :--- |
| **`CredentialType::Key`** | `"key"` | SSH 密钥对（公钥 + 私钥 PEM），支持 passphrase 保护 |
| **`CredentialType::Password`** | `"password"` | SSH 密码口令，加密持久化于系统凭据库 |
| **`CredentialType::Agent`** | `"agent"` | 本地 Pageant / OpenSSH Agent 命名管道转发通道 |
| **`CredentialType::Certificate`** | `"certificate"` | X.509 或 OpenSSH CA 签发的短期证书 |

---

## ⚡ 4. 核心安全特性与功能

### 4.1 密钥生成器 (Key Generator)
- 支持主流安全算法标准：
  - **Ed25519**（默认推荐，性能高且抗侧信道攻击）；
  - **RSA-2048 / RSA-4096**（传统兼容模式）；
  - **ECDSA (NIST P-256 / P-384)**；
- 自动计算并展示公钥指纹（格式：`SHA256:xxxx`）；
- 一键导出标准 OpenSSH `authorized_keys` 公钥格式。

### 4.2 随机密码生成器 (Password Generator)
- 采用强密码学伪随机发生器；
- 默认生成 20 位包含大小写字母、数字与特殊符号的高强度口令，杜绝弱口令风险；
- 严格遵循**只进不出物理脱敏**原则，日志流绝不记录明文口令内容。

### 4.3 敏感操作安全审计 (Security Audit Logging)
当用户在界面上执行公钥复制、私钥复制或密码提取时，系统自动分发 `CredentialSecretCopiedEvent`：
- **公钥复制**：标记为低敏感常规操作（`is_sensitive = false`，记录 `INFO` 日志）；
- **私钥 PEM 复制 / 密码提取**：标记为高危敏感操作（`is_sensitive = true`，以 `WARN` 级别记录安全审计日志，留存安全追溯轨迹）。

---

## 🌐 5. 事件分发系统集成 (Event Bus Integration)

凭据模块全面接入 `smagical-core` 的 `EventDispatcher` 事件总线：

```rust
// 1. 凭据持久化保存事件
core_state.events().dispatch(&CredentialSavedEvent {
    cred_id: record.id.clone(),
    name: record.name.clone(),
    cred_type: record.cred_type,
    algorithm: record.algorithm.clone(),
    username: record.username.clone(),
    fingerprint: record.fingerprint.clone(),
    is_new,
});

// 2. 敏感机密提取审计事件
core_state.events().dispatch(&CredentialSecretCopiedEvent {
    cred_id: cred_id.to_string(),
    name: name.to_string(),
    copy_type: CredentialCopyType::PrivateKey,
    is_sensitive: true,
});

// 3. 凭据删除事件
core_state.events().dispatch(&CredentialDeletedEvent {
    cred_id: cred_id.to_string(),
});
```

---

## 🔗 6. 主机联动与凭据引用设计 (Host Association)

在后续主机资产创建与编辑视图中，主机表单通过 `auth_type` 与 `credential_id` 与本模块完全打通：
1. **引用模式**：主机记录中仅存储 `credential_id: Option<String>`，解耦资产信息与认证机密；
2. **凭据选择器**：创建/编辑主机时弹出 `HostPickerList` 风格的凭据选择弹窗，实时筛选匹配对应主机的密钥或口令；
3. **级联安全检查**：当尝试删除某项凭据时，自动校验是否有主机正在引用，提供防误删安全提示。
