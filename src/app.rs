//! Slint 应用装配入口。
//!
//! 这里是当前桌面 UI 的最外层 Adapter。它知道 Slint，也持有桌面组合状态，
//! 但不直接实现 SSH、存储、终端或业务状态变更。
//!
//! 启动流程固定为：
//!
//! 1. `bootstrap::boot_state` 构造核心状态和本地依赖。
//! 2. `callbacks::bind` 把 Slint 事件翻译成核心 `Message`。
//! 3. `pump::start_backend_pump` 把后端事件送回核心状态。
//! 4. `projection::sync_window` 把核心状态投影到 Slint 属性。
//!
//! 如果未来重写 UI，新的 UI 只需要复用这条思路：持有 `CoreState` 或自己的
//! Adapter 状态，提交 `Message`，再按需复用 view model 或建立新的投影层。

mod bootstrap;
mod callbacks;
mod file_dialog;
mod ids;
mod projection;
mod pump;
mod state;
mod view_model;

use std::cell::RefCell;
use std::rc::Rc;

use uuid::Uuid;

use crate::config::{AppConfig, HostListModePreference};
use crate::model::{
    AgentSource, AuthProfile, CommandHistoryId, CommandHistoryItem, CredentialGroup,
    CredentialGroupId, CredentialId, CredentialInspection, CredentialKind, CredentialMetadata,
    GroupId, Host, HostGroup, HostId, HostNetworkSelection, KeyAlgorithm, KnownHostEntry,
    ProxyProfile, RecentConnection, SecretMaterialKind, SecretRecord, SecretRef, SftpBookmark,
    Snippet, SnippetArgument, SnippetGroup, SnippetGroupId, SnippetId, SnippetImplementation,
    SnippetImplementationId, SnippetScope, SnippetShell, SnippetSupportTarget,
    SnippetSupportTargetId, SnippetVariable, TunnelKind, TunnelRule,
};
use crate::storage::{SqliteStorage, StorageManager, ThemeProfileRecord};
use state::{AsDesktopStateView, DesktopAppState};

slint::include_modules!();

const SEED_PREVIEW_DATA_ARG: &str = "--seed-preview-data";

/// Slint 回调运行在单线程 UI 循环中，因此当前 Adapter 用 `Rc<RefCell<_>>`
/// 持有核心状态。
///
/// 这只是 Slint Adapter 的共享方式，不是核心层约束。其他 UI 可以改成
/// `Arc<Mutex<_>>`、通道或自己的状态容器，只要仍然通过核心/桌面状态接口即可。
type SharedAppState = Rc<RefCell<DesktopAppState>>;

/// 启动桌面应用。
pub fn run() -> anyhow::Result<()> {
    if std::env::args().any(|argument| argument == SEED_PREVIEW_DATA_ARG) {
        seed_preview_data()?;
        return Ok(());
    }

    select_window_backend()?;

    let state = Rc::new(RefCell::new(bootstrap::boot_state()));
    let window = AppWindow::new()?;

    callbacks::bind(&window, Rc::clone(&state));
    pump::start_backend_pump(&window, Rc::clone(&state));
    projection::sync_window(&window, state.borrow().as_desktop_state_view());

    window.run()?;
    Ok(())
}

fn select_window_backend() -> Result<(), slint::PlatformError> {
    slint::BackendSelector::new()
        .with_winit_window_attributes_hook(|attributes| {
            attributes.with_theme(Some(slint::winit_030::winit::window::Theme::Dark))
        })
        .select()
}

fn seed_preview_data() -> anyhow::Result<()> {
    let storage_backend = SqliteStorage::default_store()
        .ok_or_else(|| anyhow::anyhow!("当前平台没有可用的默认应用数据目录"))?;
    let storage = preview_storage();
    storage_backend.save(&storage)?;
    println!("已生成预览数据：{}", storage_backend.path().display());
    println!(
        "主机 {} 个，凭据 {} 个，片段 {} 个",
        storage.host_count(),
        storage.credential_count(),
        storage.snippet_count()
    );
    Ok(())
}

fn preview_storage() -> StorageManager {
    let mut storage = StorageManager {
        app_config: preview_app_config(),
        ..StorageManager::default()
    };

    let prod_group = group_id(1);
    let staging_group = group_id(2);
    let db_group = group_id(3);
    storage.upsert_group(HostGroup {
        id: prod_group,
        name: "生产环境".to_owned(),
        parent_id: None,
    });
    storage.upsert_group(HostGroup {
        id: staging_group,
        name: "预发布环境".to_owned(),
        parent_id: None,
    });
    storage.upsert_group(HostGroup {
        id: db_group,
        name: "数据库".to_owned(),
        parent_id: Some(prod_group),
    });

    let jump_host = host_id(1);
    let api_host = host_id(2);
    let db_host = host_id(3);
    let windows_host = host_id(4);
    for host in [
        preview_host(
            jump_host,
            "跳板机",
            Some(prod_group),
            "gateway",
            "jump.prod.internal",
            22,
            AuthProfile::Agent {
                username: "ops".to_owned(),
                source: AgentSource::Auto,
                key_hint: Some("id_ed25519".to_owned()),
            },
            &["prod", "bastion", "linux"],
        ),
        preview_host(
            api_host,
            "生产 API",
            Some(prod_group),
            "cloud",
            "api.prod.internal",
            2222,
            AuthProfile::Key {
                username: "deploy".to_owned(),
                key: secret_ref("keys/deploy-ed25519"),
                passphrase: Some(secret_ref("passphrases/deploy-ed25519")),
            },
            &["prod", "linux", "api"],
        ),
        preview_host(
            db_host,
            "主数据库",
            Some(db_group),
            "database",
            "10.20.0.15",
            22,
            AuthProfile::Password {
                username: "dba".to_owned(),
                secret: secret_ref("passwords/db-admin"),
            },
            &["prod", "postgres", "critical"],
        ),
        preview_host(
            windows_host,
            "构建 Windows",
            Some(staging_group),
            "windows",
            "win-build.staging.internal",
            22,
            AuthProfile::Agent {
                username: "builder".to_owned(),
                source: AgentSource::OpenSsh,
                key_hint: Some("builder_ed25519".to_owned()),
            },
            &["staging", "windows", "build"],
        ),
    ] {
        storage.upsert_host(host);
    }
    if let Some(host) = storage.hosts.iter_mut().find(|host| host.id == api_host) {
        host.proxies = vec![ProxyProfile::Socks5 {
            host: "127.0.0.1".to_owned(),
            port: 1080,
            auth: crate::model::ProxyAuth::None,
            remote_dns: true,
        }];
        host.jumps = vec![crate::model::JumpProfile {
            host_id: jump_host,
            username_override: None,
            port_override: None,
            alias: None,
        }];
    }

    seed_credentials(&mut storage);
    seed_history(&mut storage, api_host, db_host, windows_host);
    seed_snippets(&mut storage, api_host);
    seed_extensions(&mut storage, api_host, db_host);
    seed_themes(&mut storage);

    storage
}

fn preview_app_config() -> AppConfig {
    let mut config = AppConfig::default();
    config.workspace.host_list_mode = HostListModePreference::Tree;
    config
}

fn seed_credentials(storage: &mut StorageManager) {
    let keys_group = credential_group_id(1);
    let password_group = credential_group_id(2);
    let cert_group = credential_group_id(3);

    for group in [
        CredentialGroup {
            id: keys_group,
            name: "私钥".to_owned(),
            kind: CredentialKind::PrivateKey,
            parent_id: None,
            sort_order: 0,
        },
        CredentialGroup {
            id: password_group,
            name: "密码".to_owned(),
            kind: CredentialKind::Password,
            parent_id: None,
            sort_order: 1,
        },
        CredentialGroup {
            id: cert_group,
            name: "证书".to_owned(),
            kind: CredentialKind::Certificate,
            parent_id: None,
            sort_order: 2,
        },
    ] {
        storage.upsert_credential_group(group);
    }

    let deploy_key = credential_id(1);
    upsert_credential_with_secret(
        storage,
        deploy_key,
        "deploy-ed25519",
        CredentialKind::PrivateKey,
        Some(keys_group),
        Some("deploy"),
        secret_ref("keys/deploy-ed25519"),
        SecretMaterialKind::PrivateKey,
        preview_private_key("deploy-ed25519").as_bytes().to_vec(),
        Some(KeyAlgorithm::Ed25519),
        Some("SHA256:preview-deploy-key"),
    );
    upsert_credential_with_secret(
        storage,
        credential_id(2),
        "db-admin",
        CredentialKind::Password,
        Some(password_group),
        Some("dba"),
        secret_ref("passwords/db-admin"),
        SecretMaterialKind::Password,
        b"preview-password-ChangeMe!".to_vec(),
        None,
        None,
    );
    upsert_credential_with_secret(
        storage,
        credential_id(3),
        "deploy-cert",
        CredentialKind::Certificate,
        Some(cert_group),
        Some("deploy"),
        secret_ref("certs/deploy-cert"),
        SecretMaterialKind::Certificate,
        preview_certificate("deploy-cert").as_bytes().to_vec(),
        Some(KeyAlgorithm::Ed25519),
        Some("SHA256:preview-deploy-cert"),
    );
    storage.upsert_secret(SecretRecord::local_plaintext(
        secret_ref("passphrases/deploy-ed25519"),
        SecretMaterialKind::Passphrase,
        b"preview-passphrase".to_vec(),
    ));

    storage.upsert_known_host(KnownHostEntry {
        host: "api.prod.internal".to_owned(),
        port: 2222,
        key_algorithm: KeyAlgorithm::Ed25519,
        fingerprint: "SHA256:host-api-preview".to_owned(),
        trusted: true,
    });
    storage.upsert_known_host(KnownHostEntry {
        host: "10.20.0.15".to_owned(),
        port: 22,
        key_algorithm: KeyAlgorithm::Rsa,
        fingerprint: "SHA256:host-db-preview".to_owned(),
        trusted: false,
    });
}

fn seed_history(
    storage: &mut StorageManager,
    api_host: HostId,
    db_host: HostId,
    windows_host: HostId,
) {
    for (offset, host_id, label) in [
        (0, api_host, "生产 API"),
        (1, db_host, "主数据库"),
        (2, windows_host, "构建 Windows"),
    ] {
        storage.record_recent_connection(RecentConnection {
            host_id,
            label: label.to_owned(),
            connected_at_unix_secs: 1_780_000_000 + offset,
        });
    }

    for (index, host_id, command, directory) in [
        (
            1,
            Some(api_host),
            "systemctl status smagical-api",
            Some("/srv/smagical"),
        ),
        (
            2,
            Some(api_host),
            "journalctl -u smagical-api -n 80",
            Some("/srv/smagical"),
        ),
        (
            3,
            Some(db_host),
            "psql -c \"select now()\"",
            Some("/var/lib/postgresql"),
        ),
        (4, Some(windows_host), "Get-Service ssh-agent", None),
        (5, None, "ssh -V", None),
    ] {
        storage.add_command_history(CommandHistoryItem {
            id: command_history_id(index),
            host_id,
            command: command.to_owned(),
            working_directory: directory.map(str::to_owned),
            exit_code: Some(0),
            started_at_unix_secs: 1_780_000_100 + index,
            duration_ms: Some(40 + index),
        });
    }
}

fn seed_snippets(storage: &mut StorageManager, api_host: HostId) {
    let ops_group = snippet_group_id(1);
    let deploy_group = snippet_group_id(2);
    storage.upsert_snippet_group(SnippetGroup {
        id: ops_group,
        name: "运维检查".to_owned(),
        parent_id: None,
        sort_order: 0,
    });
    storage.upsert_snippet_group(SnippetGroup {
        id: deploy_group,
        name: "部署".to_owned(),
        parent_id: Some(ops_group),
        sort_order: 1,
    });

    storage.upsert_snippet(multi_target_snippet(
        snippet_id(1),
        "查看系统负载",
        "Linux 通用脚本同时支持 Debian 和 RHEL，Windows 使用 PowerShell 变体",
        SnippetScope::Global,
        Some(ops_group),
        "uptime && free -h",
        "Get-CimInstance Win32_OperatingSystem | Select-Object Caption,FreePhysicalMemory",
    ));

    let mut restart = Snippet::with_default_implementation(
        snippet_id(2),
        "重启服务".to_owned(),
        Some("重启指定 systemd 服务并查看状态".to_owned()),
        SnippetScope::Host(api_host),
        Some(deploy_group),
        "sudo systemctl restart {{service}} && systemctl status {{service}} --no-pager".to_owned(),
    );
    if let Some(variable) = restart
        .variables
        .iter_mut()
        .find(|variable| variable.name == "service")
    {
        variable.default_value = Some("smagical-api".to_owned());
    }
    if let Some(implementation) = restart.default_implementation_mut() {
        implementation.name = "Linux systemd".to_owned();
        implementation.last_arguments = vec![SnippetArgument {
            name: "service".to_owned(),
            value: "smagical-api".to_owned(),
        }];
    }
    storage.upsert_snippet(restart);
}

fn seed_extensions(storage: &mut StorageManager, api_host: HostId, db_host: HostId) {
    storage.upsert_sftp_bookmark(SftpBookmark {
        host_id: api_host,
        label: "应用目录".to_owned(),
        remote_path: "/srv/smagical".to_owned(),
    });
    storage.upsert_sftp_bookmark(SftpBookmark {
        host_id: db_host,
        label: "备份目录".to_owned(),
        remote_path: "/var/backups/postgresql".to_owned(),
    });
    storage.upsert_tunnel_rule(TunnelRule {
        name: "本地访问 PostgreSQL".to_owned(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.20.0.15".to_owned(),
        target_port: 5432,
        auto_start: false,
        exit_on_failure: false,
    });
    storage.upsert_tunnel_rule(TunnelRule {
        name: "动态代理".to_owned(),
        kind: TunnelKind::Dynamic,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 1080,
        target_host: String::new(),
        target_port: 0,
        auto_start: false,
        exit_on_failure: false,
    });
}

fn seed_themes(storage: &mut StorageManager) {
    storage.upsert_theme(ThemeProfileRecord {
        name: "预览暗色".to_owned(),
        profile_toml: "name = \"预览暗色\"\nfont_family = \"JetBrains Mono\"\nfont_size = 14.0\n"
            .to_owned(),
        builtin: false,
    });
}

fn upsert_credential_with_secret(
    storage: &mut StorageManager,
    id: CredentialId,
    name: &str,
    kind: CredentialKind,
    group_id: Option<CredentialGroupId>,
    username: Option<&str>,
    secret_ref: SecretRef,
    secret_kind: SecretMaterialKind,
    payload: Vec<u8>,
    algorithm: Option<KeyAlgorithm>,
    fingerprint: Option<&str>,
) {
    storage.upsert_secret(SecretRecord::local_plaintext(
        secret_ref.clone(),
        secret_kind,
        payload.clone(),
    ));
    storage.upsert_credential(CredentialMetadata {
        id,
        name: name.to_owned(),
        kind: kind.clone(),
        group_id,
        username: username.map(str::to_owned),
        secret: Some(secret_ref),
        key_algorithm: algorithm.clone(),
        fingerprint: fingerprint.map(str::to_owned),
    });
    storage.upsert_credential_inspection(CredentialInspection {
        credential_id: id,
        kind,
        payload_hash: format!("preview-{:08x}", payload.len()),
        parser_version: 1,
        parse_error: None,
        algorithm,
        fingerprint: fingerprint.map(str::to_owned),
        public_key: None,
        comment: Some(name.to_owned()),
        encrypted: Some(false),
        password_length: None,
        certificate: None,
    });
}

fn preview_host(
    id: HostId,
    name: &str,
    group_id: Option<GroupId>,
    icon_key: &str,
    address: &str,
    port: u16,
    auth: AuthProfile,
    tags: &[&str],
) -> Host {
    Host {
        id,
        name: name.to_owned(),
        group_id,
        icon_key: icon_key.to_owned(),
        tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        address: address.to_owned(),
        port,
        auth,
        network: HostNetworkSelection::default(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    }
}

fn multi_target_snippet(
    id: SnippetId,
    name: &str,
    description: &str,
    scope: SnippetScope,
    group_id: Option<SnippetGroupId>,
    linux_command: &str,
    windows_command: &str,
) -> Snippet {
    let linux_impl = SnippetImplementationId(uuid_from_u128(301));
    let windows_impl = SnippetImplementationId(uuid_from_u128(302));
    Snippet {
        id,
        name: name.to_owned(),
        description: Some(description.to_owned()),
        scope,
        group_id,
        variables: Vec::<SnippetVariable>::new(),
        implementations: vec![
            SnippetImplementation {
                id: linux_impl,
                snippet_id: id,
                name: "Linux 通用".to_owned(),
                shell: SnippetShell::Bash,
                command_template: linux_command.to_owned(),
                notes: Some("Linux 发行版目标共享这份脚本".to_owned()),
                last_arguments: Vec::new(),
                sort_order: 0,
            },
            SnippetImplementation {
                id: windows_impl,
                snippet_id: id,
                name: "Windows PowerShell".to_owned(),
                shell: SnippetShell::PowerShell,
                command_template: windows_command.to_owned(),
                notes: None,
                last_arguments: Vec::new(),
                sort_order: 1,
            },
        ],
        support_targets: vec![
            snippet_target(id, 1, "linux", "Linux", linux_impl, 0),
            snippet_target(id, 2, "debian-ubuntu", "Debian / Ubuntu", linux_impl, 1),
            snippet_target(id, 3, "rhel-centos", "RHEL / CentOS", linux_impl, 2),
            snippet_target(id, 5, "alpine", "Alpine", linux_impl, 3),
            snippet_target(id, 6, "fedora", "Fedora", linux_impl, 4),
            snippet_target(id, 7, "arch", "Arch", linux_impl, 5),
            snippet_target(id, 8, "suse", "SUSE / openSUSE", linux_impl, 6),
            snippet_target(id, 9, "freebsd", "FreeBSD", linux_impl, 7),
            snippet_target(
                id,
                4,
                "windows-powershell",
                "Windows PowerShell",
                windows_impl,
                8,
            ),
        ],
    }
}

fn snippet_target(
    snippet_id: SnippetId,
    seed: u128,
    target_key: &str,
    display_name: &str,
    implementation_id: SnippetImplementationId,
    sort_order: i32,
) -> SnippetSupportTarget {
    SnippetSupportTarget {
        id: SnippetSupportTargetId(uuid_from_u128(400 + seed)),
        snippet_id,
        target_key: target_key.to_owned(),
        display_name: display_name.to_owned(),
        implementation_id,
        sort_order,
    }
}

fn preview_private_key(label: &str) -> String {
    format!(
        "-----BEGIN OPENSSH PRIVATE KEY-----\npreview-{label}\n-----END OPENSSH PRIVATE KEY-----\n"
    )
}

fn preview_certificate(label: &str) -> String {
    format!("ssh-ed25519-cert-v01@openssh.com preview-{label} {label}@smagicalssh\n")
}

fn secret_ref(value: &str) -> SecretRef {
    SecretRef(value.to_owned())
}

fn host_id(seed: u128) -> HostId {
    HostId(uuid_from_u128(seed))
}

fn group_id(seed: u128) -> GroupId {
    GroupId(uuid_from_u128(100 + seed))
}

fn credential_id(seed: u128) -> CredentialId {
    CredentialId(uuid_from_u128(200 + seed))
}

fn credential_group_id(seed: u128) -> CredentialGroupId {
    CredentialGroupId(uuid_from_u128(220 + seed))
}

fn command_history_id(seed: u64) -> CommandHistoryId {
    CommandHistoryId(uuid_from_u128(500 + u128::from(seed)))
}

fn snippet_id(seed: u128) -> SnippetId {
    SnippetId(uuid_from_u128(600 + seed))
}

fn snippet_group_id(seed: u128) -> SnippetGroupId {
    SnippetGroupId(uuid_from_u128(620 + seed))
}

fn uuid_from_u128(seed: u128) -> Uuid {
    Uuid::from_u128(0x9000_0000_0000_0000_0000_0000_0000_0000 + seed)
}
