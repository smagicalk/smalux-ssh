//! 预设集群与丰富演示资产种子数据

use std::collections::HashMap;
use crate::domain::{
    credential::{CredentialRecord, CredentialType},
    group::GroupRecord,
    history::HistoryRecord,
    host::{HostRecord, HostStatus},
    snippet::{SnippetGroupRecord, SnippetRecord},
    tunnel::{TunnelRecord, TunnelRunMode, TunnelType},
};

/// 预设模拟测试与演示种子数据集合
pub struct SeedData {
    /// 预设主机分组列表
    pub groups: Vec<GroupRecord>,
    /// 预设主机记录列表
    pub hosts: Vec<HostRecord>,
    /// 预设安全凭据记录列表
    pub credentials: Vec<CredentialRecord>,
    /// 预设代码片段分组列表
    pub snippet_groups: Vec<SnippetGroupRecord>,
    /// 预设代码片段列表
    pub snippets: Vec<SnippetRecord>,
    /// 预设网络隧道与端口转发列表
    pub tunnels: Vec<TunnelRecord>,
    /// 预设会话历史记录列表
    pub history: Vec<HistoryRecord>,
    /// 预设终端屏幕快照字典
    pub snapshots: HashMap<String, String>,
}

/// 生成完整的演示与测试种子数据集
pub fn generate_seed_data() -> SeedData {
        let groups = vec![
            GroupRecord {
                id: "grp-prod".to_string(),
                name: "生产集群 (Production)".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: true,
                sort_order: 0,
            },
            GroupRecord {
                id: "grp-k8s".to_string(),
                name: "Kubernetes 集群".to_string(),
                parent_id: Some("grp-prod".to_string()),
                level: 1,
                is_expanded: true,
                sort_order: 1,
            },
            GroupRecord {
                id: "grp-db".to_string(),
                name: "核心数据库集群".to_string(),
                parent_id: Some("grp-prod".to_string()),
                level: 1,
                is_expanded: true,
                sort_order: 2,
            },
            GroupRecord {
                id: "grp-edge".to_string(),
                name: "边缘网关与缓存".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: true,
                sort_order: 3,
            },
            GroupRecord {
                id: "grp-ai".to_string(),
                name: "AI 算力集群 (GPU)".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: true,
                sort_order: 4,
            },
            GroupRecord {
                id: "grp-dr".to_string(),
                name: "容灾与测试环境".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: false,
                sort_order: 5,
            },
        ];

        let hosts = vec![
            HostRecord {
                id: "1".to_string(),
                name: "prod-server-01".to_string(),
                address: "192.168.1.100".to_string(),
                port: 22,
                parent_group_id: Some("grp-prod".to_string()),
                credential_id: Some("cred-prod-ed25519".to_string()),
                status: HostStatus::Online,
                ping_ms: 21,
                sort_order: 0,
                notes: "主生产业务服务".to_string(),
                ..Default::default()
            },
            HostRecord {
                id: "2".to_string(),
                name: "k8s-control-plane".to_string(),
                address: "10.0.0.1".to_string(),
                port: 6443,
                parent_group_id: Some("grp-k8s".to_string()),
                credential_id: Some("cred-prod-ed25519".to_string()),
                status: HostStatus::Warning,
                ping_ms: 68,
                sort_order: 1,
                notes: "K8s 主控节点".to_string(),
                ..Default::default()
            },
            HostRecord {
                id: "host-k8s-w1".to_string(),
                name: "k8s-worker-node-01".to_string(),
                address: "10.0.0.11".to_string(),
                port: 22,
                parent_group_id: Some("grp-k8s".to_string()),
                credential_id: Some("cred-prod-ed25519".to_string()),
                status: HostStatus::Online,
                ping_ms: 24,
                sort_order: 2,
                notes: "K8s 工作节点 1".to_string(),
                ..Default::default()
            },
            HostRecord {
                id: "3".to_string(),
                name: "db-cluster-primary".to_string(),
                address: "10.0.1.50".to_string(),
                port: 5432,
                parent_group_id: Some("grp-db".to_string()),
                credential_id: Some("cred-bastion-pwd".to_string()),
                status: HostStatus::Online,
                ping_ms: 18,
                sort_order: 3,
                notes: "PostgreSQL 主库".to_string(),
                ..Default::default()
            },
            HostRecord {
                id: "host-db-s1".to_string(),
                name: "db-cluster-standby".to_string(),
                address: "10.0.1.51".to_string(),
                port: 5432,
                parent_group_id: Some("grp-db".to_string()),
                credential_id: Some("cred-bastion-pwd".to_string()),
                status: HostStatus::Online,
                ping_ms: 20,
                sort_order: 4,
                notes: "PostgreSQL 从库".to_string(),
                ..Default::default()
            },
            HostRecord {
                id: "4".to_string(),
                name: "redis-cache-shard-0".to_string(),
                address: "10.0.2.10".to_string(),
                port: 6379,
                parent_group_id: Some("grp-edge".to_string()),
                credential_id: Some("cred-1pwd-agent".to_string()),
                status: HostStatus::Online,
                ping_ms: 12,
                sort_order: 5,
                notes: "Redis 缓存分片".to_string(),
                ..Default::default()
            },
            HostRecord {
                id: "5".to_string(),
                name: "auth-gateway-edge".to_string(),
                address: "47.98.12.33".to_string(),
                port: 443,
                parent_group_id: Some("grp-edge".to_string()),
                credential_id: Some("cred-1pwd-agent".to_string()),
                status: HostStatus::Online,
                ping_ms: 35,
                sort_order: 6,
                notes: "边缘认证网关".to_string(),
                ..Default::default()
            },
            HostRecord {
                id: "6".to_string(),
                name: "ai-inference-gpu".to_string(),
                address: "10.0.8.200".to_string(),
                port: 22,
                parent_group_id: Some("grp-ai".to_string()),
                credential_id: Some("cred-dev-rsa".to_string()),
                status: HostStatus::Online,
                ping_ms: 14,
                sort_order: 7,
                notes: "NVIDIA H100 推理卡".to_string(),
                ..Default::default()
            },
            HostRecord {
                id: "7".to_string(),
                name: "backup-node-dr".to_string(),
                address: "192.168.100.250".to_string(),
                port: 22,
                parent_group_id: Some("grp-dr".to_string()),
                credential_id: Some("cred-openssh-agent".to_string()),
                status: HostStatus::Offline,
                ping_ms: 0,
                sort_order: 8,
                notes: "冷备容灾节点".to_string(),
                ..Default::default()
            },
            HostRecord {
                id: "host-staging".to_string(),
                name: "staging-api-test".to_string(),
                address: "10.0.12.88".to_string(),
                port: 22,
                parent_group_id: Some("grp-dr".to_string()),
                credential_id: Some("cred-bitwarden-agent".to_string()),
                status: HostStatus::Offline,
                ping_ms: 0,
                sort_order: 9,
                notes: "预发接口测试机".to_string(),
                ..Default::default()
            },
        ];

        // 预设开箱即用的真实历史会话种子
        let now_sec = 1725019200u64; // 基准时间戳
        let history = vec![
            HistoryRecord {
                id: "hist-seed-1".to_string(),
                host_id: Some("1".to_string()),
                title: "prod-server-01".to_string(),
                address: "192.168.1.100:22".to_string(),
                port: 22,
                username: "root".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 600, // 10分钟前
                disconnected_at: None,
                duration_secs: 600,
                exit_status: "active".to_string(),
                error_msg: None,
                is_pinned: true,
                connect_count: 18,
                has_snapshot: true,
                snapshot_lines: 16,
            },
            HistoryRecord {
                id: "hist-seed-7".to_string(),
                host_id: Some("7".to_string()),
                title: "ci-runner-master".to_string(),
                address: "10.0.3.10:22".to_string(),
                port: 22,
                username: "gitlab-runner".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 1800, // 30分钟前
                disconnected_at: Some(now_sec - 1350),
                duration_secs: 450,
                exit_status: "success".to_string(),
                error_msg: None,
                is_pinned: true,
                connect_count: 24,
                has_snapshot: true,
                snapshot_lines: 10,
            },
            HistoryRecord {
                id: "hist-seed-2".to_string(),
                host_id: Some("5".to_string()),
                title: "auth-gateway-edge".to_string(),
                address: "47.98.12.33:443".to_string(),
                port: 443,
                username: "admin".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 3600, // 1小时前
                disconnected_at: Some(now_sec - 2700),
                duration_secs: 900,
                exit_status: "success".to_string(),
                error_msg: None,
                is_pinned: false,
                connect_count: 5,
                has_snapshot: true,
                snapshot_lines: 12,
            },
            HistoryRecord {
                id: "hist-seed-3".to_string(),
                host_id: Some("3".to_string()),
                title: "db-cluster-primary".to_string(),
                address: "10.0.1.50:5432".to_string(),
                port: 5432,
                username: "postgres".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 7200, // 2小时前
                disconnected_at: Some(now_sec - 3600),
                duration_secs: 3600,
                exit_status: "success".to_string(),
                error_msg: None,
                is_pinned: false,
                connect_count: 12,
                has_snapshot: true,
                snapshot_lines: 15,
            },
            HistoryRecord {
                id: "hist-seed-4".to_string(),
                host_id: Some("2".to_string()),
                title: "k8s-control-plane".to_string(),
                address: "10.0.0.1:6443".to_string(),
                port: 6443,
                username: "admin".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 86400, // 昨天
                disconnected_at: Some(now_sec - 85200),
                duration_secs: 1200,
                exit_status: "success".to_string(),
                error_msg: None,
                is_pinned: false,
                connect_count: 7,
                has_snapshot: true,
                snapshot_lines: 11,
            },
            HistoryRecord {
                id: "hist-seed-5".to_string(),
                host_id: Some("4".to_string()),
                title: "redis-cache-shard-0".to_string(),
                address: "10.0.2.10:6379".to_string(),
                port: 6379,
                username: "dev".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 90000, // 昨天
                disconnected_at: Some(now_sec - 89998),
                duration_secs: 2,
                exit_status: "timeout".to_string(),
                error_msg: Some("连接超时: 目标主机网络无响应 (ETIMEDOUT)".to_string()),
                is_pinned: false,
                connect_count: 2,
                has_snapshot: false,
                snapshot_lines: 0,
            },
            HistoryRecord {
                id: "hist-seed-8".to_string(),
                host_id: None,
                title: "bastion-jump-server".to_string(),
                address: "114.55.88.99:2222".to_string(),
                port: 2222,
                username: "ops".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 100000, // 昨天
                disconnected_at: Some(now_sec - 94600),
                duration_secs: 5400,
                exit_status: "success".to_string(),
                error_msg: None,
                is_pinned: false,
                connect_count: 3,
                has_snapshot: false,
                snapshot_lines: 0,
            },
            HistoryRecord {
                id: "hist-seed-6".to_string(),
                host_id: Some("6".to_string()),
                title: "ai-inference-gpu".to_string(),
                address: "10.0.8.200:22".to_string(),
                port: 22,
                username: "cuda".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 259200, // 3天前
                disconnected_at: Some(now_sec - 252000),
                duration_secs: 7200,
                exit_status: "success".to_string(),
                error_msg: None,
                is_pinned: false,
                connect_count: 9,
                has_snapshot: true,
                snapshot_lines: 14,
            },
            HistoryRecord {
                id: "hist-seed-9".to_string(),
                host_id: None,
                title: "dev-sandbox-container".to_string(),
                address: "192.168.10.5:22".to_string(),
                port: 22,
                username: "developer".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 345600, // 4天前
                disconnected_at: Some(now_sec - 345300),
                duration_secs: 300,
                exit_status: "auth_failed".to_string(),
                error_msg: Some("SSH 密钥认证被拒绝 (Permission denied - publickey)".to_string()),
                is_pinned: false,
                connect_count: 1,
                has_snapshot: false,
                snapshot_lines: 0,
            },
            HistoryRecord {
                id: "hist-seed-10".to_string(),
                host_id: None,
                title: "backup-nas-storage".to_string(),
                address: "192.168.1.250:22".to_string(),
                port: 22,
                username: "backup".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 432000, // 5天前
                disconnected_at: Some(now_sec - 417600),
                duration_secs: 14400,
                exit_status: "success".to_string(),
                error_msg: None,
                is_pinned: false,
                connect_count: 4,
                has_snapshot: true,
                snapshot_lines: 9,
            },
        ];

        // 真实且具代表性的终端屏幕快照文本
        let mut snapshots = std::collections::HashMap::new();
        snapshots.insert(
            "hist-seed-1".to_string(),
            r#"Linux prod-server-01 5.15.0-1031-aws #35-Ubuntu SMP Fri Jan 24 16:30:11 UTC 2026 x86_64
Welcome to Ubuntu 22.04.4 LTS (GNU/Linux 5.15.0-1031-aws x86_64)

 * Documentation:  https://help.ubuntu.com
 * Management:     https://landscape.canonical.com
 * Support:        https://ubuntu.com/pro

root@prod-server-01:~# uptime
 20:30:15 up 42 days,  3:14,  1 user,  load average: 0.24, 0.18, 0.12
root@prod-server-01:~# systemctl status nginx
● nginx.service - A high performance web server and a reverse proxy server
     Loaded: loaded (/lib/systemd/system/nginx.service; enabled; vendor preset: enabled)
     Active: active (running) since Wed 2026-08-19 10:12:04 CST; 11 days ago
   Main PID: 14829 (nginx)
      Tasks: 9 (limit: 38241)
     Memory: 48.2M
        CPU: 1min 24.120s
root@prod-server-01:~# tail -n 3 /var/log/nginx/access.log
192.168.1.15 - - [30/Aug/2026:20:29:45 +0800] "GET /api/v1/health HTTP/1.1" 200 45 "-" "curl/7.81.0"
192.168.1.18 - - [30/Aug/2026:20:29:48 +0800] "POST /api/v1/metrics HTTP/1.1" 200 128 "-" "Prometheus/2.45.0"
192.168.1.20 - - [30/Aug/2026:20:29:50 +0800] "GET /api/v1/nodes HTTP/1.1" 200 1024 "-" "smalux-agent/0.1.0""#.to_string(),
        );

        snapshots.insert(
            "hist-seed-7".to_string(),
            r#"gitlab-runner@ci-runner-master:~$ docker info | grep "Server Version"
 Server Version: 26.1.3
gitlab-runner@ci-runner-master:~$ gitlab-runner status
Runtime platform                                    arch=amd64 os=linux pid=1892 revision=08101416 version=16.11.0
gitlab-runner: Service is running!
gitlab-runner@ci-runner-master:~$ gitlab-runner run-single --builds-dir /tmp/builds
Checking for jobs... nothing
gitlab-runner@ci-runner-master:~$ exit
logout"#.to_string(),
        );

        snapshots.insert(
            "hist-seed-2".to_string(),
            r#"admin@auth-gateway-edge:~$ sudo iptables -L -n -v --line-numbers
Chain INPUT (policy ACCEPT 1420K packets, 189M bytes)
num   pkts bytes target     prot opt in     out     source               destination         
1     982K  132M ACCEPT     tcp  --  eth0   *       0.0.0.0/0            0.0.0.0/0            tcp dpt:443
2     124K   18M ACCEPT     tcp  --  eth0   *       0.0.0.0/0            0.0.0.0/0            tcp dpt:80
admin@auth-gateway-edge:~$ curl -I http://127.0.0.1:8080/health
HTTP/1.1 200 OK
Date: Sun, 30 Aug 2026 19:40:02 GMT
Content-Type: application/json
Content-Length: 18
Server: smalux-gateway/2.4.0

{"status":"healthy"}
admin@auth-gateway-edge:~$ exit
logout"#.to_string(),
        );

        snapshots.insert(
            "hist-seed-3".to_string(),
            r#"postgres@db-cluster-primary:~$ psql -U postgres -d smalux_db
psql (16.2 (Debian 16.2-1.pgdg120+1))
Type "help" for help.

smalux_db=# SELECT datname, numbackends, xact_commit, xact_rollback FROM pg_stat_database WHERE datname='smalux_db';
  datname  | numbackends | xact_commit | xact_rollback 
-----------+-------------+-------------+---------------
 smalux_db |          16 |     4892182 |           142
(1 row)

smalux_db=# SELECT count(*) FROM hosts_inventory;
 count 
-------
   128
(1 row)

smalux_db=# \q
postgres@db-cluster-primary:~$ exit
logout"#.to_string(),
        );

        snapshots.insert(
            "hist-seed-4".to_string(),
            r#"admin@k8s-control-plane:~$ kubectl get nodes -o wide
NAME           STATUS   ROLES           AGE   VERSION   INTERNAL-IP   OS-IMAGE             KERNEL-VERSION
k8s-master-1   Ready    control-plane   85d   v1.29.2   10.0.0.1      Ubuntu 22.04.3 LTS   5.15.0-94-generic
k8s-worker-1   Ready    <none>          85d   v1.29.2   10.0.0.11     Ubuntu 22.04.3 LTS   5.15.0-94-generic
k8s-worker-2   Ready    <none>          85d   v1.29.2   10.0.0.12     Ubuntu 22.04.3 LTS   5.15.0-94-generic
admin@k8s-control-plane:~$ kubectl get pods -n kube-system
NAME                                       READY   STATUS    RESTARTS   AGE
coredns-76f75df574-8k9pl                   1/1     Running   0          85d
etcd-k8s-master-1                          1/1     Running   0          85d
kube-apiserver-k8s-master-1                1/1     Running   0          85d
kube-controller-manager-k8s-master-1       1/1     Running   0          85d
admin@k8s-control-plane:~$ exit
logout"#.to_string(),
        );

        snapshots.insert(
            "hist-seed-6".to_string(),
            r#"cuda@ai-inference-gpu:~$ nvidia-smi
Sun Aug 30 18:22:10 2026       
+-----------------------------------------------------------------------------------------+
| NVIDIA-SMI 550.54.14              Driver Version: 550.54.14      CUDA Version: 12.4     |
|-----------------------------------------+------------------------+----------------------+
| GPU  Name                 Persistence-M | Bus-Id          Disp.A | Volatile Uncorr. ECC |
| Fan  Temp   Perf          Pwr:Usage/Cap |           Memory-Usage | GPU-Util  Compute M. |
|=========================================+========================+======================|
|   0  NVIDIA H100 80GB HBM3          On  | 00000000:06:00.0   Off |                    0 |
| N/A   42C    P0             112W / 700W |  42100MiB / 81559MiB |    68%      Default |
+-----------------------------------------+------------------------+----------------------+
cuda@ai-inference-gpu:~$ docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
NAMES           STATUS         PORTS
vllm-qwen-72b   Up 2 days      0.0.0.0:8000->8000/tcp
cuda@ai-inference-gpu:~$ exit
logout"#.to_string(),
        );

        snapshots.insert(
            "hist-seed-10".to_string(),
            r#"backup@backup-nas-storage:~$ df -h /mnt/data
Filesystem      Size  Used Avail Use% Mounted on
/dev/md0         18T  9.4T  7.8T  55% /mnt/data
backup@backup-nas-storage:~$ zpool status -x
all pools are healthy
backup@backup-nas-storage:~$ rsync --version | head -n 1
rsync  version 3.2.7  protocol version 31
backup@backup-nas-storage:~$ exit
logout"#.to_string(),
        );

        let credentials = vec![
            CredentialRecord {
                id: "cred-prod-ed25519".to_string(),
                name: "生产集群 Ed25519 密钥".to_string(),
                cred_type: CredentialType::Key,
                algorithm: "Ed25519".to_string(),
                username: Some("root".to_string()),
                secret_data: "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\nQyNTUxOQAAACDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQAAAJCR2Y69kdmO\nvQAAAAtzc2gtZWQyNTUxOQAAACDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6\nSQAAAEA6WjG4m2JpL5kZ8yQ3uP9tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP\n4lXqZbM4xGgA9QJ6tL7r1n6SQAAAA1zbWFsdXgtc3NoLWtleQECAwQ=\n-----END OPENSSH PRIVATE KEY-----".to_string(),
                passphrase: Some("••••••••".to_string()),
                public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMfyDbS9fsr2nUF83bA/iVeplszjEaAD1Anq0vuvWfpJ root@smalux-k8s-prod".to_string()),
                fingerprint: Some("SHA256:k9x8Ym+3pLq1G7vX2nR8uM4aP9tL3wQ2bN6pG8oP4qM".to_string()),
                bound_host_count: 5,
                created_at: "2026-08-15 10:20:00".to_string(),
                updated_at: "2026-09-01 09:15:00".to_string(),
                notes: "Kubernetes 核心控制面与网关认证主密钥".to_string(),
            },
            CredentialRecord {
                id: "cred-bastion-pwd".to_string(),
                name: "堡垒跳板机 Root 管理密码".to_string(),
                cred_type: CredentialType::Password,
                algorithm: "Password".to_string(),
                username: Some("root".to_string()),
                secret_data: "SmaluxSecure#2026!P@ss".to_string(),
                passphrase: None,
                public_key: None,
                fingerprint: None,
                bound_host_count: 2,
                created_at: "2026-08-18 14:30:00".to_string(),
                updated_at: "2026-08-30 18:00:00".to_string(),
                notes: "边缘网关与跳板机应急控制台特权密码".to_string(),
            },
            CredentialRecord {
                id: "cred-1pwd-agent".to_string(),
                name: "1Password SSH Agent".to_string(),
                cred_type: CredentialType::Agent,
                algorithm: "1Password".to_string(),
                username: Some("developer".to_string()),
                secret_data: r"\\.\pipe\1password-ssh-agent".to_string(),
                passphrase: None,
                public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPq8Xm7kL9vN2wA4bF6jZ8qM3uP9tL3wR2bN6pG8oP4q 1password-agent".to_string()),
                fingerprint: Some("SHA256:1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d".to_string()),
                bound_host_count: 3,
                created_at: "2026-08-20 16:45:00".to_string(),
                updated_at: "2026-09-01 11:30:00".to_string(),
                notes: "硬件安全保管箱，受 Windows Hello 生物识别保护".to_string(),
            },
            CredentialRecord {
                id: "cred-dev-rsa".to_string(),
                name: "CI/CD 流水线 RSA 密钥".to_string(),
                cred_type: CredentialType::Key,
                algorithm: "RSA-4096".to_string(),
                username: Some("gitlab-runner".to_string()),
                secret_data: "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0k6K9X7L9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQ2Y69kd\nvQMwAAAAtzc2gtcnNhAAAAAwEAAQAAAgEAv7b4a2p8zXqN3vP9xK2m4rL9nO1pQ8tL\n3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQ\nA6WjG4m2JpL5kZ8yQ3uP9tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4l\nXqZbM4xGgA9QJ6tL7r1n6SQAAAA1zbWFsdXgtc3NoLWtleQECAwQFAgcICQoLDA0O\nDxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0\nBBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3Bx\ncnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6Ch\noqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/wMHCw8TFxsfIycrLzM3Oz9DR\n0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/wID\n-----END RSA PRIVATE KEY-----".to_string(),
                passphrase: None,
                public_key: Some("ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDv7b4a2p8zXqN3vP9xK2m4rL9nO1pQ8tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQA6WjG4m2JpL5kZ8yQ3uP9tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQAAAA1zbWFsdXgtc3NoLWtleQECAwQ gitlab@runner".to_string()),
                fingerprint: Some("SHA256:9c8b7a6f5e4d3c2b1a0f9e8d7c6b5a4f".to_string()),
                bound_host_count: 1,
                created_at: "2026-08-25 09:00:00".to_string(),
                updated_at: "2026-08-25 09:00:00".to_string(),
                notes: "GitLab Runner 持续部署构建机专有免密凭据".to_string(),
            },
            CredentialRecord {
                id: "cred-openssh-agent".to_string(),
                name: "Windows OpenSSH Agent".to_string(),
                cred_type: CredentialType::Agent,
                algorithm: "OpenSSH".to_string(),
                username: Some("ssh-agent".to_string()),
                secret_data: r"\\.\pipe\openssh-ssh-agent".to_string(),
                passphrase: None,
                public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPq8Xm7kL9vN2wA4bF6jZ8qM3uP9tL3wR2bN6pG8oP4q openssh-agent".to_string()),
                fingerprint: Some("SHA256:8f4e2c9a1b3d5e7f0a2c4e6b8d0f1a3c".to_string()),
                bound_host_count: 4,
                created_at: "2026-08-22 11:00:00".to_string(),
                updated_at: "2026-08-22 11:00:00".to_string(),
                notes: "Windows 内置 OpenSSH Authentication Agent 命名管道".to_string(),
            },
            CredentialRecord {
                id: "cred-bitwarden-agent".to_string(),
                name: "Bitwarden SSH Agent".to_string(),
                cred_type: CredentialType::Agent,
                algorithm: "Bitwarden".to_string(),
                username: Some("vault".to_string()),
                secret_data: r"\\.\pipe\bitwarden-ssh-agent".to_string(),
                passphrase: None,
                public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPq8Xm7kL9vN2wA4bF6jZ8qM3uP9tL3wR2bN6pG8oP4q bitwarden-agent".to_string()),
                fingerprint: Some("SHA256:3d5e7f9a1c2b4d6e8f0a1b3c5d7e9f1a".to_string()),
                bound_host_count: 2,
                created_at: "2026-08-28 15:20:00".to_string(),
                updated_at: "2026-08-28 15:20:00".to_string(),
                notes: "Bitwarden / Vaultwarden 桌面端安全托管 SSH Agent".to_string(),
            },
        ];

        let snippet_groups = vec![
            SnippetGroupRecord {
                id: "sgrp-docker".to_string(),
                name: "Docker 容器与微服务".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: true,
                sort_order: 0,
            },
            SnippetGroupRecord {
                id: "sgrp-k8s".to_string(),
                name: "Kubernetes 集群排障".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: true,
                sort_order: 1,
            },
            SnippetGroupRecord {
                id: "sgrp-ops".to_string(),
                name: "Linux 系统基础运维".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: true,
                sort_order: 2,
            },
            SnippetGroupRecord {
                id: "sgrp-ops-net".to_string(),
                name: "网络与端口诊断".to_string(),
                parent_id: Some("sgrp-ops".to_string()),
                level: 1,
                is_expanded: true,
                sort_order: 0,
            },
            SnippetGroupRecord {
                id: "sgrp-db".to_string(),
                name: "数据库维护与慢查询".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: true,
                sort_order: 3,
            },
        ];

        let snippets = vec![
            SnippetRecord {
                id: "snip-docker-ps".to_string(),
                parent_group_id: Some("sgrp-docker".to_string()),
                title: "Docker 容器健康列表".to_string(),
                content: "docker ps --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}\t{{.Image}}'".to_string(),
                language: "bash".to_string(),
                tags: vec!["docker".to_string(), "status".to_string()],
                auto_execute: true,
                description: "格式化列出所有正在运行的 Docker 容器及其映射端口".to_string(),
                is_favorite: true,
                sort_order: 0,
                updated_at: "2026-09-01 10:00:00".to_string(),
            },
            SnippetRecord {
                id: "snip-docker-log".to_string(),
                parent_group_id: Some("sgrp-docker".to_string()),
                title: "Docker 实时日志追踪".to_string(),
                content: "docker logs -f --tail={{lines:100}} {{container_name}}".to_string(),
                language: "bash".to_string(),
                tags: vec!["docker".to_string(), "logs".to_string()],
                auto_execute: true,
                description: "动态参数化追踪指定容器尾部日志流".to_string(),
                is_favorite: true,
                sort_order: 1,
                updated_at: "2026-09-01 10:15:00".to_string(),
            },
            SnippetRecord {
                id: "snip-docker-prune".to_string(),
                parent_group_id: Some("sgrp-docker".to_string()),
                title: "清理悬空镜像与未用卷".to_string(),
                content: "docker system prune -f --volumes".to_string(),
                language: "bash".to_string(),
                tags: vec!["docker".to_string(), "cleanup".to_string()],
                auto_execute: false,
                description: "安全释放 Docker 废弃卷与虚悬镜像存储空间 (仅粘贴需手动确认)".to_string(),
                is_favorite: false,
                sort_order: 2,
                updated_at: "2026-08-30 16:00:00".to_string(),
            },
            SnippetRecord {
                id: "snip-k8s-abnormal".to_string(),
                parent_group_id: Some("sgrp-k8s".to_string()),
                title: "K8s 全命名空间异常 Pod 排查".to_string(),
                content: "kubectl get pods -A --field-selector=status.phase!=Running,status.phase!=Succeeded".to_string(),
                language: "bash".to_string(),
                tags: vec!["k8s".to_string(), "pod".to_string(), "troubleshoot".to_string()],
                auto_execute: true,
                description: "快速筛出集群中 CrashLoopBackOff 或 Pending 状态的故障 Pod".to_string(),
                is_favorite: true,
                sort_order: 0,
                updated_at: "2026-08-28 14:00:00".to_string(),
            },
            SnippetRecord {
                id: "snip-k8s-restart".to_string(),
                parent_group_id: Some("sgrp-k8s".to_string()),
                title: "K8s 滚动重启 Deployment".to_string(),
                content: "kubectl rollout restart deployment/{{deployment_name}} -n {{namespace:default}}".to_string(),
                language: "bash".to_string(),
                tags: vec!["k8s".to_string(), "restart".to_string()],
                auto_execute: true,
                description: "无损滚动重启指定的 Kubernetes 工作负载".to_string(),
                is_favorite: false,
                sort_order: 1,
                updated_at: "2026-08-29 11:30:00".to_string(),
            },
            SnippetRecord {
                id: "snip-sys-load".to_string(),
                parent_group_id: Some("sgrp-ops".to_string()),
                title: "Linux CPU 与内存负载 Top 20".to_string(),
                content: "top -b -n 1 | head -n 20".to_string(),
                language: "bash".to_string(),
                tags: vec!["linux".to_string(), "performance".to_string()],
                auto_execute: true,
                description: "单次截取系统负载、任务队列与前 20 活跃进程".to_string(),
                is_favorite: false,
                sort_order: 0,
                updated_at: "2026-08-25 09:00:00".to_string(),
            },
            SnippetRecord {
                id: "snip-disk-large".to_string(),
                parent_group_id: Some("sgrp-ops".to_string()),
                title: "磁盘大文件扫描 (>500MB)".to_string(),
                content: "find {{scan_path:/var/log}} -type f -size +{{size_mb:500}}M -exec ls -lh {} + 2>/dev/null | awk '{print $9 \": \" $5}'".to_string(),
                language: "bash".to_string(),
                tags: vec!["disk".to_string(), "find".to_string()],
                auto_execute: true,
                description: "排查磁盘爆满根因，快速定位指定目录下超过阈值的大文件".to_string(),
                is_favorite: true,
                sort_order: 1,
                updated_at: "2026-08-26 15:40:00".to_string(),
            },
            SnippetRecord {
                id: "snip-net-port".to_string(),
                parent_group_id: Some("sgrp-ops-net".to_string()),
                title: "查询指定端口占用与监听进程".to_string(),
                content: "lsof -i :{{port:8080}} || netstat -tulnp | grep :{{port:8080}}".to_string(),
                language: "bash".to_string(),
                tags: vec!["network".to_string(), "port".to_string()],
                auto_execute: true,
                description: "检测本地端口监听状态与绑定 PID 进程名".to_string(),
                is_favorite: true,
                sort_order: 0,
                updated_at: "2026-08-27 18:00:00".to_string(),
            },
            SnippetRecord {
                id: "snip-db-mysql-process".to_string(),
                parent_group_id: Some("sgrp-db".to_string()),
                title: "MySQL 活跃长事务与锁排查".to_string(),
                content: "mysql -u {{user:root}} -p{{password}} -h {{host:127.0.0.1}} -e 'SHOW FULL PROCESSLIST;'".to_string(),
                language: "sql".to_string(),
                tags: vec!["mysql".to_string(), "database".to_string()],
                auto_execute: false,
                description: "查看 MySQL 当前执行超过阈值的慢查询与卡顿连接".to_string(),
                is_favorite: false,
                sort_order: 0,
                updated_at: "2026-08-28 20:00:00".to_string(),
            },
            SnippetRecord {
                id: "snip-db-redis-info".to_string(),
                parent_group_id: Some("sgrp-db".to_string()),
                title: "Redis 内存占用分析".to_string(),
                content: "redis-cli -h {{host:127.0.0.1}} -p {{port:6379}} info memory | grep -E 'used_memory_human|used_memory_peak_human|mem_fragmentation_ratio'".to_string(),
                language: "bash".to_string(),
                tags: vec!["redis".to_string(), "memory".to_string()],
                auto_execute: true,
                description: "获取 Redis 当前内存峰值与碎片率指标".to_string(),
                is_favorite: true,
                sort_order: 1,
                updated_at: "2026-08-30 09:20:00".to_string(),
            },
        ];

        let tunnels = vec![
            TunnelRecord {
                id: "tun-mysql-prod".to_string(),
                name: "生产 MySQL 数据库直连".to_string(),
                tunnel_type: TunnelType::Local,
                ssh_host_id: Some("1".to_string()),
                ssh_host_name: "prod-server-01".to_string(),
                local_bind: "127.0.0.1".to_string(),
                local_port: 3306,
                remote_host: "10.0.1.50".to_string(),
                remote_port: 3306,
                jump_chain: Vec::new(),
                enabled: true,
                is_running: false,
                run_mode: TunnelRunMode::FollowTerminal,
                auto_start: false,
                auto_reconnect: true,
                remote_dns: false,
                compression: true,
                active_connections: 3,
                total_bytes_in: 25 * 1024 * 1024 + 400 * 1024,
                total_bytes_out: 4 * 1024 * 1024 + 128 * 1024,
                proxy_proto: String::new(),
                proxy_username: String::new(),
                proxy_password: String::new(),
                notes: "将远程隔离区生产主数据库映射至本地 3306 端口，供本地 Navicat 安全连接".to_string(),
                updated_at: "2026-09-01 10:20:00".to_string(),
            },
            TunnelRecord {
                id: "tun-redis-k8s".to_string(),
                name: "K8s Redis 哨兵集群映射".to_string(),
                tunnel_type: TunnelType::Local,
                ssh_host_id: Some("2".to_string()),
                ssh_host_name: "k8s-control-plane".to_string(),
                local_bind: "127.0.0.1".to_string(),
                local_port: 6379,
                remote_host: "10.0.2.10".to_string(),
                remote_port: 6379,
                jump_chain: Vec::new(),
                enabled: true,
                is_running: false,
                run_mode: TunnelRunMode::FollowTerminal,
                auto_start: false,
                auto_reconnect: true,
                remote_dns: false,
                compression: false,
                active_connections: 1,
                total_bytes_in: 8 * 1024 * 1024,
                total_bytes_out: 1024 * 1024,
                proxy_proto: String::new(),
                proxy_username: String::new(),
                proxy_password: String::new(),
                notes: "用于本地查看与排查集群缓存热点 Key 与集群状态".to_string(),
                updated_at: "2026-08-31 16:45:00".to_string(),
            },
            TunnelRecord {
                id: "tun-webhook-dev".to_string(),
                name: "公网穿透本地 Webhook 调试".to_string(),
                tunnel_type: TunnelType::Remote,
                ssh_host_id: Some("5".to_string()),
                ssh_host_name: "auth-gateway-edge".to_string(),
                local_bind: "127.0.0.1".to_string(),
                local_port: 3000,
                remote_host: "0.0.0.0".to_string(),
                remote_port: 8080,
                jump_chain: Vec::new(),
                enabled: false,
                is_running: false,
                run_mode: TunnelRunMode::FollowTerminal,
                auto_start: false,
                auto_reconnect: true,
                remote_dns: false,
                compression: true,
                active_connections: 0,
                total_bytes_in: 512 * 1024,
                total_bytes_out: 1024 * 1024 * 2,
                proxy_proto: String::new(),
                proxy_username: String::new(),
                proxy_password: String::new(),
                notes: "将公网网关 8080 端口打回本地 Node/Rust 开发服务 3000 端口，接收第三方回调".to_string(),
                updated_at: "2026-08-30 18:00:00".to_string(),
            },
            TunnelRecord {
                id: "tun-socks5-corp".to_string(),
                name: "生产全网段 SOCKS5 代理".to_string(),
                tunnel_type: TunnelType::Dynamic,
                ssh_host_id: Some("1".to_string()),
                ssh_host_name: "prod-server-01".to_string(),
                local_bind: "127.0.0.1".to_string(),
                local_port: 1080,
                remote_host: "ANY".to_string(),
                remote_port: 0,
                jump_chain: Vec::new(),
                enabled: true,
                is_running: false,
                run_mode: TunnelRunMode::FollowApp,
                auto_start: false,
                auto_reconnect: true,
                remote_dns: true,
                compression: true,
                active_connections: 8,
                total_bytes_in: 1024 * 1024 * 128,
                total_bytes_out: 1024 * 1024 * 18,
                proxy_proto: String::new(),
                proxy_username: String::new(),
                proxy_password: String::new(),
                notes: "本地透明 SOCKS5 代理网关，支持浏览器或终端走内网网段统一访问".to_string(),
                updated_at: "2026-09-02 08:30:00".to_string(),
            },
            TunnelRecord {
                id: "tun-bastion-chain".to_string(),
                name: "内网第二跳核心堡垒机".to_string(),
                tunnel_type: TunnelType::JumpHost,
                ssh_host_id: Some("5".to_string()),
                ssh_host_name: "auth-gateway-edge".to_string(),
                local_bind: "".to_string(),
                local_port: 0,
                remote_host: "192.168.1.100".to_string(),
                remote_port: 22,
                jump_chain: vec![
                    crate::domain::tunnel::JumpHopRecord {
                        host_id: "5".to_string(),
                        host_name: "auth-gateway-edge".to_string(),
                        host_address: "47.98.12.33".to_string(),
                        host_port: 22,
                        enabled: true,
                    },
                    crate::domain::tunnel::JumpHopRecord {
                        host_id: "1".to_string(),
                        host_name: "prod-server-01".to_string(),
                        host_address: "192.168.1.100".to_string(),
                        host_port: 22,
                        enabled: true,
                    },
                ],
                enabled: true,
                is_running: false,
                run_mode: TunnelRunMode::FollowTerminal,
                auto_start: true,
                auto_reconnect: true,
                remote_dns: false,
                compression: false,
                active_connections: 2,
                total_bytes_in: 1024 * 1024 * 5,
                total_bytes_out: 1024 * 1024 * 3,
                proxy_proto: String::new(),
                proxy_username: String::new(),
                proxy_password: String::new(),
                notes: "级联跳板机 (ProxyJump)，作为访问深度内网物理隔离集群的中转桥梁".to_string(),
                updated_at: "2026-08-29 14:00:00".to_string(),
            },
            TunnelRecord {
                id: "tun-proxy-corp".to_string(),
                name: "公司统一出网 HTTP/SOCKS 代理".to_string(),
                tunnel_type: TunnelType::ProxyServer,
                ssh_host_id: None,
                ssh_host_name: "proxy.internal.corp".to_string(),
                local_bind: "127.0.0.1".to_string(),
                local_port: 7890,
                remote_host: "proxy.corp.net".to_string(),
                remote_port: 7890,
                jump_chain: Vec::new(),
                enabled: true,
                is_running: false,
                run_mode: TunnelRunMode::FollowApp,
                auto_start: false,
                auto_reconnect: true,
                remote_dns: true,
                compression: false,
                active_connections: 5,
                total_bytes_in: 1024 * 1024 * 45,
                total_bytes_out: 1024 * 1024 * 12,
                proxy_proto: "SOCKS5".to_string(),
                proxy_username: "corp_user".to_string(),
                proxy_password: "corp_password".to_string(),
                notes: "企业级合规出网代理池，经由专属代理机房安全出海".to_string(),
                updated_at: "2026-08-28 11:15:00".to_string(),
            },
        ];

        tracing::info!(
            target: "smagical_core::storage",
            "初始化 MockStorage 预设种子引擎完成: 已注入 {} 个层级分组, {} 台主机资产, {} 条历史会话记录 (含 {} 份屏幕快照), {} 条凭据记录, {} 个代码片段分组, {} 个代码片段与 {} 条网络隧道/代理",
            groups.len(),
            hosts.len(),
            history.len(),
            snapshots.len(),
            credentials.len(),
            snippet_groups.len(),
            snippets.len(),
            tunnels.len()
        );

    SeedData {
        groups,
        hosts,
        credentials,
        snippet_groups,
        snippets,
        tunnels,
        history,
        snapshots,
    }
}