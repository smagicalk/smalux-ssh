//! 基于纯内存与预设种子的 Mock 存储实现。

pub mod host_repo;
pub mod group_repo;
pub mod history_repo;
pub mod credential_repo;
pub mod snippet_repo;
pub mod tunnel_repo;
pub mod config_repo;
pub mod seed_data;

pub use host_repo::MockHostRepository;
pub use group_repo::MockGroupRepository;
pub use history_repo::MockHistoryRepository;
pub use credential_repo::MockCredentialRepository;
pub use snippet_repo::MockSnippetRepository;
pub use tunnel_repo::MockTunnelRepository;
pub use config_repo::MockConfigRepository;

use smagical_core::storage::{
    AppStorage, ConfigRepository, CredentialRepository, GroupRepository, HistoryRepository,
    HostRepository, SnippetRepository, StorageResult, TunnelRepository,
};

/// 聚合内存存储实现 (MockStorage)
#[derive(Debug, Clone)]
pub struct MockStorage {
    hosts_repo: MockHostRepository,
    groups_repo: MockGroupRepository,
    history_repo: MockHistoryRepository,
    credentials_repo: MockCredentialRepository,
    snippets_repo: MockSnippetRepository,
    tunnels_repo: MockTunnelRepository,
    config_repo: MockConfigRepository,
}

impl Default for MockStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MockStorage {
    /// 创建空的 MockStorage
    pub fn new() -> Self {
        let hosts_repo = MockHostRepository::new();
        let credentials_repo = MockCredentialRepository::with_hosts(hosts_repo.hosts_raw());
        Self {
            hosts_repo,
            groups_repo: MockGroupRepository::new(),
            history_repo: MockHistoryRepository::new(),
            credentials_repo,
            snippets_repo: MockSnippetRepository::new(),
            tunnels_repo: MockTunnelRepository::new(),
            config_repo: MockConfigRepository::new(),
        }
    }

    /// 创建内置完整集群与丰富主机演示资产的 MockStorage (开箱即用种子引擎)
    pub fn new_seeded() -> Self {
        let seed = seed_data::generate_seed_data();
        let hosts_repo = MockHostRepository::with_hosts(seed.hosts);
        let credentials_repo = MockCredentialRepository::with_credentials_and_hosts(seed.credentials, hosts_repo.hosts_raw());
        Self {
            hosts_repo,
            groups_repo: MockGroupRepository::with_groups(seed.groups),
            history_repo: MockHistoryRepository::with_history_and_snapshots(seed.history, seed.snapshots),
            credentials_repo,
            snippets_repo: MockSnippetRepository::with_data(seed.snippets, seed.snippet_groups),
            tunnels_repo: MockTunnelRepository::with_tunnels(seed.tunnels),
            config_repo: MockConfigRepository::new(),
        }
    }
}

impl AppStorage for MockStorage {

    fn hosts(&self) -> &dyn HostRepository {
        &self.hosts_repo
    }

    fn groups(&self) -> &dyn GroupRepository {
        &self.groups_repo
    }

    fn history(&self) -> &dyn HistoryRepository {
        &self.history_repo
    }

    fn credentials(&self) -> &dyn CredentialRepository {
        &self.credentials_repo
    }

    fn snippets(&self) -> &dyn SnippetRepository {
        &self.snippets_repo
    }

    fn tunnels(&self) -> &dyn TunnelRepository {
        &self.tunnels_repo
    }

    fn config(&self) -> &dyn ConfigRepository {
        &self.config_repo
    }

    fn reload(&self) -> StorageResult<()> {
        tracing::debug!(target: "smagical_core::storage", "MockStorage 内存重新加载请求 (无操作)");
        Ok(())
    }

    fn flush(&self) -> StorageResult<()> {
        tracing::debug!(target: "smagical_core::storage", "MockStorage 内存刷盘请求 (无操作)");
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::domain::{
        credential::{CredentialRecord, CredentialType},
        history::HistoryRecord,
        host::HostRecord,
        snippet::{SnippetGroupRecord, SnippetRecord},
        tunnel::TunnelType,
    };

    #[test]
    fn test_mock_storage_seeded_data() {
        let storage = MockStorage::new_seeded();
        
        let groups = storage.groups().list_all().unwrap();
        assert_eq!(groups.len(), 6);
        assert_eq!(groups[0].name, "生产集群 (Production)");

        let hosts = storage.hosts().list_all().unwrap();
        assert_eq!(hosts.len(), 10);
        assert_eq!(hosts[0].name, "prod-server-01");
    }

    #[test]
    fn test_mock_storage_host_crud() {
        let storage = MockStorage::new();
        assert_eq!(storage.hosts().list_all().unwrap().len(), 0);

        let new_host = HostRecord::new("h1", "Host 1", "127.0.0.1", 22);
        storage.hosts().save(&new_host).unwrap();

        assert_eq!(storage.hosts().list_all().unwrap().len(), 1);
        let found = storage.hosts().get_by_id("h1").unwrap().unwrap();
        assert_eq!(found.name, "Host 1");

        let deleted = storage.hosts().delete("h1").unwrap();
        assert!(deleted);
        assert_eq!(storage.hosts().list_all().unwrap().len(), 0);
    }

    #[test]
    fn test_mock_storage_list_reordering() {
        let storage = MockStorage::new();
        storage.hosts().save(&HostRecord::new("1", "H1", "1.1.1.1", 22)).unwrap();
        storage.hosts().save(&HostRecord::new("2", "H2", "2.2.2.2", 22)).unwrap();
        storage.hosts().save(&HostRecord::new("3", "H3", "3.3.3.3", 22)).unwrap();

        // 调整顺序为: 3, 1, 2
        storage.hosts().update_list_order(&["3".to_string(), "1".to_string(), "2".to_string()]).unwrap();

        let hosts = storage.hosts().list_all().unwrap();
        assert_eq!(hosts[0].id, "3");
        assert_eq!(hosts[1].id, "1");
        assert_eq!(hosts[2].id, "2");
    }

    #[test]
    fn test_mock_storage_history_crud_and_pin() {
        let storage = MockStorage::new_seeded();
        let history = storage.history().list_all().unwrap();
        assert_eq!(history.len(), 10);
        // 置顶项排在最前
        assert!(history[0].is_pinned);
        assert!(history[1].is_pinned);

        // 验证种子中已包含终端屏幕快照
        let snap1 = storage.history().get_snapshot("hist-seed-1").unwrap();
        assert!(snap1.is_some());
        assert!(snap1.unwrap().contains("prod-server-01"));

        // 新建并保存一条新记录
        let mut new_hist = HistoryRecord::new_ssh(
            "test-hist-1".to_string(),
            Some("1".to_string()),
            "custom-ssh".to_string(),
            "1.2.3.4:22".to_string(),
            22,
            "root".to_string(),
            1725020000,
        );
        storage.history().save(&new_hist).unwrap();

        let list_after_save = storage.history().list_all().unwrap();
        assert_eq!(list_after_save.len(), 11);

        // 切换置顶
        let pinned = storage.history().toggle_pin("test-hist-1").unwrap();
        assert!(pinned);

        // 验证置顶后排序
        let list_after_pin = storage.history().list_all().unwrap();
        assert!(list_after_pin[0].is_pinned);

        // 标记关闭
        new_hist.mark_closed(1725020600);
        storage.history().save(&new_hist).unwrap();
        let fetched = storage.history().get_by_id("test-hist-1").unwrap().unwrap();
        assert_eq!(fetched.exit_status, "success");
        assert_eq!(fetched.duration_secs, 600);

        // 删除记录
        let deleted = storage.history().delete("test-hist-1").unwrap();
        assert!(deleted);
        assert_eq!(storage.history().list_all().unwrap().len(), 10);

        // 清空（保留置顶）
        storage.history().clear_all(true).unwrap();
        let remaining = storage.history().list_all().unwrap();
        assert_eq!(remaining.len(), 2); // 仅剩 2 个种子置顶项
        assert!(remaining.iter().all(|r| r.is_pinned));
    }


    #[test]
    fn test_mock_storage_session_snapshot() {
        let storage = MockStorage::new();
        let mut hist = HistoryRecord::new_ssh(
            "hist-snap-1".to_string(),
            None,
            "test-server".to_string(),
            "192.168.1.1:22".to_string(),
            22,
            "root".to_string(),
            1000,
        );

        // 模拟多行终端屏幕输出
        let raw_output = "line 1: welcome\nline 2: login success\nline 3: ls -la\nline 4: output 1\nline 5: exit 0";
        storage.history().save_snapshot("hist-snap-1", raw_output, 3).unwrap(); // 限制最多保留 3 行

        let snapshot = storage.history().get_snapshot("hist-snap-1").unwrap().unwrap();
        let lines: Vec<&str> = snapshot.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line 3: ls -la");
        assert_eq!(lines[1], "line 4: output 1");
        assert_eq!(lines[2], "line 5: exit 0");

        hist.record_snapshot(3);
        storage.history().save(&hist).unwrap();

        let fetched = storage.history().get_by_id("hist-snap-1").unwrap().unwrap();
        assert!(fetched.has_snapshot);
        assert_eq!(fetched.snapshot_lines, 3);

        // 删除历史会话，快照应自动关联清除
        storage.history().delete("hist-snap-1").unwrap();
        let snap_after_del = storage.history().get_snapshot("hist-snap-1").unwrap();
        assert!(snap_after_del.is_none());
    }

    #[test]
    fn test_mock_storage_local_shell_history() {
        let storage = MockStorage::new();
        let local_hist = HistoryRecord::new_local(
            "hist-local-1".to_string(),
            Some("local-powershell".to_string()),
            "PowerShell 7".to_string(),
            "PowerShell 7 (pwsh)".to_string(),
            1725020000,
        );
        storage.history().save(&local_hist).unwrap();

        let list = storage.history().list_all().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].session_type, "local");
        assert_eq!(list[0].host_id.as_deref(), Some("local-powershell"));
        assert_eq!(list[0].address, "Local (PowerShell 7)");
    }

    #[test]
    fn test_mock_storage_credential_crud() {
        let storage = MockStorage::new_seeded();
        let list = storage.credentials().list_all().unwrap();
        assert_eq!(list.len(), 6);

        // 新增凭据
        let new_cred = CredentialRecord::new_key(
            "cred-test-key",
            "测试 Ed25519 凭据",
            "Ed25519",
            "test_private_key",
            None,
            Some("ssh-ed25519 AAAAC... test@test".to_string()),
            Some("SHA256:11223344".to_string()),
            "单元测试专供",
        );
        storage.credentials().save(&new_cred).unwrap();
        assert_eq!(storage.credentials().list_all().unwrap().len(), 7);

        // 查询单条
        let fetched = storage.credentials().get_by_id("cred-test-key").unwrap().unwrap();
        assert_eq!(fetched.name, "测试 Ed25519 凭据");
        assert_eq!(fetched.cred_type, CredentialType::Key);
        assert_eq!(fetched.fingerprint.as_deref(), Some("SHA256:11223344"));

        // 更新单条
        let mut updated = fetched;
        updated.name = "已重命名测试凭据".to_string();
        storage.credentials().save(&updated).unwrap();
        assert_eq!(storage.credentials().get_by_id("cred-test-key").unwrap().unwrap().name, "已重命名测试凭据");

        // 删除单条
        let deleted = storage.credentials().delete("cred-test-key").unwrap();
        assert!(deleted);
        assert_eq!(storage.credentials().list_all().unwrap().len(), 6);

        // 验证凭据与主机相互引用与查询
        // 1. 根据凭据查关联主机
        let prod_hosts = storage.hosts().list_by_credential("cred-prod-ed25519").unwrap();
        assert_eq!(prod_hosts.len(), 3);
        assert_eq!(prod_hosts[0].id, "1");

        // 2. 根据凭据查关联主机 ID
        let bound_ids = storage.credentials().get_bound_hosts("cred-prod-ed25519").unwrap();
        assert_eq!(bound_ids.len(), 3);
        assert!(bound_ids.contains(&"1".to_string()));
        assert!(bound_ids.contains(&"2".to_string()));
        assert!(bound_ids.contains(&"host-k8s-w1".to_string()));

        // 3. 动态引用计数验证
        let cred_prod = storage.credentials().get_by_id("cred-prod-ed25519").unwrap().unwrap();
        assert_eq!(cred_prod.bound_host_count, 3);

        // 4. 按类型分类查询
        let agent_creds = storage.credentials().list_by_type(CredentialType::Agent).unwrap();
        assert_eq!(agent_creds.len(), 3);

        // 5. 模糊搜索
        let search_results = storage.credentials().search("1Password").unwrap();
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].id, "cred-1pwd-agent");
    }

    #[test]
    fn test_mock_storage_snippets_and_groups_crud() {
        let storage = MockStorage::new_seeded();

        // 1. 验证预设种子
        let groups = storage.snippets().list_groups().unwrap();
        let snippets = storage.snippets().list_all().unwrap();
        assert_eq!(groups.len(), 5);
        assert_eq!(snippets.len(), 10);

        // 2. 按分组过滤
        let docker_snippets = storage.snippets().list_by_group(Some("sgrp-docker")).unwrap();
        assert_eq!(docker_snippets.len(), 3);

        // 3. 搜索
        let search_res = storage.snippets().search("restart").unwrap();
        assert_eq!(search_res.len(), 1);
        assert_eq!(search_res[0].id, "snip-k8s-restart");

        // 4. 星标切换
        let fav_before = storage.snippets().get_by_id("snip-docker-prune").unwrap().unwrap().is_favorite;
        assert!(!fav_before);
        let fav_after = storage.snippets().toggle_favorite("snip-docker-prune").unwrap();
        assert!(fav_after);
        assert!(storage.snippets().get_by_id("snip-docker-prune").unwrap().unwrap().is_favorite);

        // 5. 新建与删除片段
        let new_snip = SnippetRecord::new("snip-test", "测试脚本", "echo 'hello'", "bash");
        storage.snippets().save(&new_snip).unwrap();
        assert_eq!(storage.snippets().list_all().unwrap().len(), 11);
        storage.snippets().delete("snip-test").unwrap();
        assert_eq!(storage.snippets().list_all().unwrap().len(), 10);

        // 6. 新建与移动分组
        let new_grp = SnippetGroupRecord::child("sgrp-test-child", "子分组", "sgrp-docker", 1);
        storage.snippets().save_group(&new_grp).unwrap();
        assert_eq!(storage.snippets().list_groups().unwrap().len(), 6);
        storage.snippets().delete_group("sgrp-test-child").unwrap();
        assert_eq!(storage.snippets().list_groups().unwrap().len(), 5);
    }

    #[test]
    fn test_mock_storage_tunnels_crud_and_status() {
        let storage = MockStorage::new_seeded();

        // 1. 种子加载校验
        let tunnels = storage.tunnels().list_all().unwrap();
        assert_eq!(tunnels.len(), 6);

        // 2. 按类型过滤
        let locals = storage.tunnels().list_by_type(TunnelType::Local).unwrap();
        assert_eq!(locals.len(), 2);
        let remotes = storage.tunnels().list_by_type(TunnelType::Remote).unwrap();
        assert_eq!(remotes.len(), 1);
        let proxies = storage.tunnels().list_by_type(TunnelType::ProxyServer).unwrap();
        assert_eq!(proxies.len(), 1);

        // 3. 搜索过滤
        let search_mysql = storage.tunnels().search("mysql").unwrap();
        assert_eq!(search_mysql.len(), 1);
        assert_eq!(search_mysql[0].id, "tun-mysql-prod");

        // 4. 启停状态切换
        let is_running = storage.tunnels().get_by_id("tun-webhook-dev").unwrap().unwrap().is_running;
        assert!(!is_running);
        storage.tunnels().set_running("tun-webhook-dev", true).unwrap();
        let is_running_after = storage.tunnels().get_by_id("tun-webhook-dev").unwrap().unwrap().is_running;
        assert!(is_running_after);

        // 5. 流量更新
        storage.tunnels().update_metrics("tun-webhook-dev", 2, 1024, 2048).unwrap();
        let updated = storage.tunnels().get_by_id("tun-webhook-dev").unwrap().unwrap();
        assert_eq!(updated.active_connections, 2);
        assert!(updated.total_bytes_in >= 1024);

        // 6. 新建与删除
        let mut new_tun = updated.clone();
        new_tun.id = "tun-temp-test".to_string();
        new_tun.name = "临时测试隧道".to_string();
        storage.tunnels().save(&new_tun).unwrap();
        assert_eq!(storage.tunnels().list_all().unwrap().len(), 7);

        let deleted = storage.tunnels().delete("tun-temp-test").unwrap();
        assert!(deleted);
        assert_eq!(storage.tunnels().list_all().unwrap().len(), 6);
    }

    #[test]
    fn test_mock_storage_config_crud_and_update() {
        let storage = MockStorage::new_seeded();

        // 1. 读取初始默认配置
        let cfg = storage.config().get().unwrap();
        assert_eq!(cfg.language, "zh-CN");
        assert_eq!(cfg.theme_id, "builtin.ui.darcula");
        assert_eq!(cfg.font_size, 13.0);
        assert!(!cfg.flag_desktop_notifications);

        // 2. 局部修改 update
        let updated = storage.config().update(Box::new(|c| {
            c.font_size = 15.0;
            c.theme_id = "builtin.ui.one-dark".to_string();
            c.flag_desktop_notifications = true;
        })).unwrap();
        assert_eq!(updated.font_size, 15.0);
        assert_eq!(updated.theme_id, "builtin.ui.one-dark");
        assert!(updated.flag_desktop_notifications);

        // 3. 读取验证
        let fresh = storage.config().get().unwrap();
        assert_eq!(fresh.font_size, 15.0);
        assert_eq!(fresh.theme_id, "builtin.ui.one-dark");

        // 4. 重置回默认值
        let reset = storage.config().reset_to_default().unwrap();
        assert_eq!(reset.font_size, 13.0);
        assert_eq!(reset.theme_id, "builtin.ui.darcula");
        assert!(!reset.flag_desktop_notifications);
    }
}



