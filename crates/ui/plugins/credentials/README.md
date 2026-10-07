# plugin-credentials: 凭据保险箱与密码学密钥对托管插件

> **模块定位**：Smalux 凭据与密码学生命周期中心（左侧活动栏第五位）。负责主机登录凭据（密码、RSA/Ed25519/ECDSA 私钥、SSH 证书与 Pageant/SSH Agent）的安全存储、指纹解析、绑定主机引用统计，以及集成纯 Rust 原生密钥对现场生成工作台 (`GenerateKeyPairModal`)。

---

## 一、 模块职责与着力点 (Focus & Scope)

1. **凭据保险箱主视图 (`views/credentials_center_view.slint`)**：
   - 卡片式网格与列表呈现所有安全凭据；
   - 实时展示凭据算法（Ed25519 / RSA 4096 / P-256）、SHA256 规范指纹 (`SHA256:...`)、最后修改时间以及关联的主机引用计数；
   - 支持一键导出公钥内容 (`id_ed25519.pub`) 与剪贴板极速复制。
2. **纯 Rust 原生密钥生成器 (`modals/generate-key-pair-modal.slint`)**：
   - 彻底脱离系统外部 `ssh-keygen` 工具依赖；
   - 支持交互式选择算法、设置密码口令（Passphrase）保护与自定义注释（Comment），纯内存输出 OpenSSH PEM 私钥与公钥。
3. **安全内存生命周期与权限控制**：
   - 依赖底层 `smagical-storage::VaultManager` 与 Argon2id，明文私钥仅在使用瞬间在内存中解密，使用完毕或会话关闭时自动执行等长 0 字节覆盖抹零 (Zeroize)。

---

## 二、 核心 Slint 组件与调用方法 (Slint Components & Usage)

### 1. `CredentialsCenterView` (全屏凭据中心大页面)
```slint
import { CredentialsCenterView } from "@plugin-credentials/credentials_plugin.slint";

CredentialsCenterView {
    horizontal-stretch: 1;
    back-to-terminal => {
        WindowBridge.main-view = "terminal";
    }
}
```

### 2. `GenerateKeyPairModal` (密钥生成高级模态框)
```slint
import { GenerateKeyPairModal } from "@plugin-credentials/credentials_plugin.slint";

GenerateKeyPairModal {
    is-open: CredentialsBridge.is-generate-key-modal-open;
    confirm(algorithm, passphrase, comment) => {
        CredentialsBridge.generate-key-pair-advanced(algorithm, passphrase, comment);
        CredentialsBridge.is-generate-key-modal-open = false;
    }
    close => { CredentialsBridge.is-generate-key-modal-open = false; }
}
```

### 3. `CredentialsDrawer` (左侧轻量凭据列表抽屉)
```slint
import { CredentialsDrawer } from "@plugin-credentials/credentials_plugin.slint";

CredentialsDrawer {
    width: 100%;
    height: 100%;
    collapse => { WindowBridge.is-left-drawer-open = false; }
}
```

---

## 三、 核心 Rust 驱动与 API 接口 (Rust Handlers & APIs)

在 `crates/smagical-ui/src/handlers/credential_handlers.rs` 中集中管理：

### 1. 现场生成高强度 SSH 密钥对
```rust
pub fn generate_keypair_native(
    algorithm: &str,
    passphrase: Option<&str>,
    comment: &str,
    keygen_svc: &NativeKeygenService,
) -> Result<GeneratedKeyPair>
```
- **参数**：
  - `algorithm`: 算法类型 (`"ed25519"`, `"rsa"`, `"ecdsa"`)；
  - `passphrase`: 可选的加密保护密码；
  - `comment`: 密钥注释（如 `"admin@smalux-prod"`）；
  - `keygen_svc`: 纯 Rust 密钥生成器。
- **说明**：基于 `ed25519-dalek` / `rsa` 纯内存计算，直接输出符合 RFC 4716 格式的 OpenSSH 字符串与指纹。

### 2. 凭据增删改查与安全落地
```rust
pub async fn save_credential_secure(
    record: CredentialRecord,
    storage: &Arc<dyn AppStorage>,
) -> Result<()>
```
- **参数**：待保存的凭据领域对象与仓储接口；
- **机制**：由 `SeaOrmStorage` 进行 AES-256-GCM 密文封包写入，数据库中绝对不落地明文密码或私钥。

---

## 四、 核心数据模型 (Data Models)

```rust
// 对应 Slint 中的 CredentialItemData
pub struct CredentialItemData {
    pub id: SharedString,
    pub name: SharedString,
    pub cred_type: SharedString, // "password" | "private_key" | "certificate" | "agent"
    pub username: SharedString,
    pub algorithm: SharedString, // "Ed25519", "RSA-4096"
    pub fingerprint: SharedString,
    pub public_key: SharedString,
    pub has_passphrase: bool,
    pub notes: SharedString,
    pub bound_host_count: i32,
    pub updated_at: SharedString,
}
```
