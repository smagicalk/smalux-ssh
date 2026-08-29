//! 场景模拟预设生成器 (Mock Presets Injector)

use crate::models::{DebugHostCard, DebugRawNode};

/// 预设场景类别
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetKind {
    /// 预设 1: 海量企业级生产集群 (35+ 台主机)
    RichClusters,
    /// 预设 2: 极简精简数据 (3 台主机)
    Minimal,
    /// 预设 3: 超长名称极限测试 (50+ 字符)
    LongNames,
    /// 预设 4: 极深层级嵌套 (5 级结构)
    DeepNested,
    /// 预设 5: 全告警与离线故障集
    Faults,
}

impl PresetKind {
    /// 从 ID 字符串解析预设类别
    pub fn from_id(id: &str) -> Self {
        match id {
            "rich_clusters" => Self::RichClusters,
            "minimal" => Self::Minimal,
            "long_names" => Self::LongNames,
            "deep_nested" => Self::DeepNested,
            "faults" => Self::Faults,
            _ => Self::Minimal,
        }
    }
}

/// 根据预设 ID 注入对应的树结构与卡片数据集
pub fn get_preset_by_id(id: &str) -> (Vec<DebugRawNode>, Vec<DebugHostCard>) {
    match PresetKind::from_id(id) {
        PresetKind::RichClusters => get_preset_rich_clusters(),
        PresetKind::Minimal => get_preset_minimal(),
        PresetKind::LongNames => get_preset_long_names(),
        PresetKind::DeepNested => get_preset_deep_nested(),
        PresetKind::Faults => get_preset_faults(),
    }
}

/// 预设 1: 海量企业级生产集群 (35+ 台主机)
pub fn get_preset_rich_clusters() -> (Vec<DebugRawNode>, Vec<DebugHostCard>) {
    let mut tree = Vec::new();
    let mut cards = Vec::new();

    let clusters = vec![
        ("grp-prod", "生产核心集群 (Production)", vec![
            ("p-1", "prod-web-01", "10.0.1.10", 80, "online", 18),
            ("p-2", "prod-web-02", "10.0.1.11", 80, "online", 19),
            ("p-3", "prod-api-01", "10.0.1.20", 443, "online", 22),
            ("p-4", "prod-api-02", "10.0.1.21", 443, "online", 24),
            ("p-5", "prod-auth-service", "10.0.1.30", 8080, "online", 21),
        ]),
        ("grp-k8s", "Kubernetes 容器编排集群", vec![
            ("k-1", "k8s-control-plane-01", "10.10.0.1", 6443, "online", 35),
            ("k-2", "k8s-control-plane-02", "10.10.0.2", 6443, "online", 36),
            ("k-3", "k8s-worker-node-01", "10.10.1.1", 22, "online", 28),
            ("k-4", "k8s-worker-node-02", "10.10.1.2", 22, "online", 27),
            ("k-5", "k8s-worker-node-03", "10.10.1.3", 22, "online", 29),
            ("k-6", "k8s-ingress-controller", "10.10.2.1", 443, "online", 30),
        ]),
        ("grp-db", "核心持久化数据库集群", vec![
            ("db-1", "pg-master-primary", "10.20.1.100", 5432, "online", 12),
            ("db-2", "pg-standby-replica", "10.20.1.101", 5432, "online", 14),
            ("db-3", "redis-cluster-shard-01", "10.20.2.10", 6379, "online", 8),
            ("db-4", "redis-cluster-shard-02", "10.20.2.11", 6379, "online", 9),
            ("db-5", "mongo-shard-primary", "10.20.3.50", 27017, "online", 15),
        ]),
        ("grp-bigdata", "大数据离线与实时计算", vec![
            ("bd-1", "hadoop-namenode", "10.30.1.1", 9000, "online", 42),
            ("bd-2", "hadoop-datanode-01", "10.30.1.11", 50010, "online", 45),
            ("bd-3", "spark-master", "10.30.2.1", 7077, "online", 38),
            ("bd-4", "flink-jobmanager", "10.30.3.1", 8081, "online", 33),
            ("bd-5", "kafka-broker-01", "10.30.4.1", 9092, "online", 20),
            ("bd-6", "clickhouse-olap-01", "10.30.5.1", 8123, "online", 25),
        ]),
        ("grp-ai", "AI 大模型算力与推理中心", vec![
            ("ai-1", "nvidia-h100-train-01", "10.40.1.10", 22, "online", 15),
            ("ai-2", "nvidia-h100-train-02", "10.40.1.11", 22, "online", 16),
            ("ai-3", "vllm-inference-service", "10.40.2.1", 8000, "online", 14),
            ("ai-4", "triton-model-server", "10.40.2.2", 8001, "online", 18),
        ]),
        ("grp-sec", "边缘网关与安全监控", vec![
            ("sec-1", "cloudflare-edge-proxy", "172.16.1.1", 443, "online", 48),
            ("sec-2", "waf-security-firewall", "172.16.1.10", 22, "online", 31),
            ("sec-3", "bastion-jump-server", "172.16.2.1", 2222, "online", 26),
            ("sec-4", "vault-kms-cluster", "172.16.3.1", 8200, "online", 22),
        ]),
        ("grp-dr", "多活容灾与冷备中心", vec![
            ("dr-1", "dr-backup-primary", "192.168.200.1", 22, "online", 55),
            ("dr-2", "dr-storage-archive", "192.168.200.2", 22, "offline", 0),
            ("dr-3", "dr-snapshot-worker", "192.168.200.3", 22, "warning", 85),
        ]),
    ];

    for (gid, gname, hosts) in clusters {
        tree.push(DebugRawNode {
            id: gid.into(),
            name: gname.into(),
            is_group: true,
            parent_id: "".into(),
            level: 0,
            address: "".into(),
            port: 0,
            status: "online".into(),
            ping_ms: 0,
            item_count: hosts.len() as i32,
        });

        for (hid, hname, haddr, hport, hstatus, hping) in hosts {
            tree.push(DebugRawNode {
                id: hid.into(),
                name: hname.into(),
                is_group: false,
                parent_id: gid.into(),
                level: 1,
                address: haddr.into(),
                port: hport,
                status: hstatus.into(),
                ping_ms: hping,
                item_count: 0,
            });

            cards.push(DebugHostCard {
                id: hid.into(),
                name: hname.into(),
                address: haddr.into(),
                port: hport,
                group: gname.into(),
                status: hstatus.into(),
                ping_ms: hping,
            });
        }
    }

    (tree, cards)
}

/// 预设 2: 极简精简数据 (3 台主机)
pub fn get_preset_minimal() -> (Vec<DebugRawNode>, Vec<DebugHostCard>) {
    let tree = vec![
        DebugRawNode { id: "grp-prod".into(), name: "生产集群 (Prod)".into(), is_group: true, parent_id: "".into(), level: 0, address: "".into(), port: 0, status: "online".into(), ping_ms: 0, item_count: 2 },
        DebugRawNode { id: "1".into(), name: "prod-server-01".into(), is_group: false, parent_id: "grp-prod".into(), level: 1, address: "192.168.1.100".into(), port: 22, status: "online".into(), ping_ms: 21, item_count: 0 },
        DebugRawNode { id: "2".into(), name: "web-server-02".into(), is_group: false, parent_id: "grp-prod".into(), level: 1, address: "192.168.1.101".into(), port: 22, status: "online".into(), ping_ms: 25, item_count: 0 },
        DebugRawNode { id: "3".into(), name: "backup-node".into(), is_group: false, parent_id: "".into(), level: 0, address: "192.168.1.200".into(), port: 22, status: "offline".into(), ping_ms: 0, item_count: 0 },
    ];
    let cards = vec![
        DebugHostCard { id: "1".into(), name: "prod-server-01".into(), address: "192.168.1.100".into(), port: 22, group: "生产集群".into(), status: "online".into(), ping_ms: 21 },
        DebugHostCard { id: "2".into(), name: "web-server-02".into(), address: "192.168.1.101".into(), port: 22, group: "生产集群".into(), status: "online".into(), ping_ms: 25 },
        DebugHostCard { id: "3".into(), name: "backup-node".into(), address: "192.168.1.200".into(), port: 22, group: "未分组".into(), status: "offline".into(), ping_ms: 0 },
    ];
    (tree, cards)
}

/// 预设 3: 超长名称极限测试 (50+ 字符)
pub fn get_preset_long_names() -> (Vec<DebugRawNode>, Vec<DebugHostCard>) {
    let tree = vec![
        DebugRawNode { id: "grp-long".into(), name: "这是一个极具挑战性的超长命名分布式微服务生产环境容器集群 (APAC-East-Production-Cluster-001)".into(), is_group: true, parent_id: "".into(), level: 0, address: "".into(), port: 0, status: "online".into(), ping_ms: 0, item_count: 3 },
        DebugRawNode { id: "h-long-1".into(), name: "kubernetes-worker-node-apac-east-001-with-nvidia-h100-gpu-accelerator".into(), is_group: false, parent_id: "grp-long".into(), level: 1, address: "10.244.100.150".into(), port: 22, status: "online".into(), ping_ms: 28, item_count: 0 },
        DebugRawNode { id: "h-long-2".into(), name: "clickhouse-olap-distributed-analytics-engine-primary-shard-01-node".into(), is_group: false, parent_id: "grp-long".into(), level: 1, address: "10.244.100.151".into(), port: 9000, status: "online".into(), ping_ms: 19, item_count: 0 },
        DebugRawNode { id: "h-long-3".into(), name: "enterprise-security-gateway-cloudflare-edge-zero-trust-proxy-service".into(), is_group: false, parent_id: "grp-long".into(), level: 1, address: "10.244.100.152".into(), port: 443, status: "warning".into(), ping_ms: 72, item_count: 0 },
    ];
    let cards = vec![
        DebugHostCard { id: "h-long-1".into(), name: "kubernetes-worker-node-apac-east-001-with-nvidia-h100-gpu-accelerator".into(), address: "10.244.100.150".into(), port: 22, group: "超长集群".into(), status: "online".into(), ping_ms: 28 },
        DebugHostCard { id: "h-long-2".into(), name: "clickhouse-olap-distributed-analytics-engine-primary-shard-01-node".into(), address: "10.244.100.151".into(), port: 9000, group: "超长集群".into(), status: "online".into(), ping_ms: 19 },
        DebugHostCard { id: "h-long-3".into(), name: "enterprise-security-gateway-cloudflare-edge-zero-trust-proxy-service".into(), address: "10.244.100.152".into(), port: 443, group: "超长集群".into(), status: "warning".into(), ping_ms: 72 },
    ];
    (tree, cards)
}

/// 预设 4: 极深层级嵌套 (5 级目录树)
pub fn get_preset_deep_nested() -> (Vec<DebugRawNode>, Vec<DebugHostCard>) {
    let tree = vec![
        DebugRawNode { id: "lvl-0".into(), name: "全球基础设施 (Global Infrastructure)".into(), is_group: true, parent_id: "".into(), level: 0, address: "".into(), port: 0, status: "online".into(), ping_ms: 0, item_count: 1 },
        DebugRawNode { id: "lvl-1".into(), name: "亚太大区 (APAC Region)".into(), is_group: true, parent_id: "lvl-0".into(), level: 1, address: "".into(), port: 0, status: "online".into(), ping_ms: 0, item_count: 1 },
        DebugRawNode { id: "lvl-2".into(), name: "杭州核心可用区 (Hangzhou AZ-A)".into(), is_group: true, parent_id: "lvl-1".into(), level: 2, address: "".into(), port: 0, status: "online".into(), ping_ms: 0, item_count: 1 },
        DebugRawNode { id: "lvl-3".into(), name: "机架机柜 POD-01 (Rack Unit)".into(), is_group: true, parent_id: "lvl-2".into(), level: 3, address: "".into(), port: 0, status: "online".into(), ping_ms: 0, item_count: 1 },
        DebugRawNode { id: "lvl-4".into(), name: "刀片计算阵列 (Blade Server 01)".into(), is_group: true, parent_id: "lvl-3".into(), level: 4, address: "".into(), port: 0, status: "online".into(), ping_ms: 0, item_count: 2 },
        DebugRawNode { id: "deep-h1".into(), name: "blade-host-slot-1".into(), is_group: false, parent_id: "lvl-4".into(), level: 5, address: "10.88.1.1".into(), port: 22, status: "online".into(), ping_ms: 12, item_count: 0 },
        DebugRawNode { id: "deep-h2".into(), name: "blade-host-slot-2".into(), is_group: false, parent_id: "lvl-4".into(), level: 5, address: "10.88.1.2".into(), port: 22, status: "online".into(), ping_ms: 15, item_count: 0 },
    ];
    let cards = vec![
        DebugHostCard { id: "deep-h1".into(), name: "blade-host-slot-1".into(), address: "10.88.1.1".into(), port: 22, group: "刀片计算阵列".into(), status: "online".into(), ping_ms: 12 },
        DebugHostCard { id: "deep-h2".into(), name: "blade-host-slot-2".into(), address: "10.88.1.2".into(), port: 22, group: "刀片计算阵列".into(), status: "online".into(), ping_ms: 15 },
    ];
    (tree, cards)
}

/// 预设 5: 全告警与离线故障集
pub fn get_preset_faults() -> (Vec<DebugRawNode>, Vec<DebugHostCard>) {
    let tree = vec![
        DebugRawNode { id: "grp-fault".into(), name: "异常故障与告警集群 (Troubleshooting)".into(), is_group: true, parent_id: "".into(), level: 0, address: "".into(), port: 0, status: "warning".into(), ping_ms: 0, item_count: 4 },
        DebugRawNode { id: "f-1".into(), name: "network-timeout-node".into(), is_group: false, parent_id: "grp-fault".into(), level: 1, address: "192.168.99.1".into(), port: 22, status: "warning".into(), ping_ms: 148, item_count: 0 },
        DebugRawNode { id: "f-2".into(), name: "packet-loss-router".into(), is_group: false, parent_id: "grp-fault".into(), level: 1, address: "192.168.99.2".into(), port: 22, status: "warning".into(), ping_ms: 280, item_count: 0 },
        DebugRawNode { id: "f-3".into(), name: "power-loss-server".into(), is_group: false, parent_id: "grp-fault".into(), level: 1, address: "192.168.99.3".into(), port: 22, status: "offline".into(), ping_ms: 0, item_count: 0 },
        DebugRawNode { id: "f-4".into(), name: "disk-failure-storage".into(), is_group: false, parent_id: "grp-fault".into(), level: 1, address: "192.168.99.4".into(), port: 22, status: "offline".into(), ping_ms: 0, item_count: 0 },
    ];
    let cards = vec![
        DebugHostCard { id: "f-1".into(), name: "network-timeout-node".into(), address: "192.168.99.1".into(), port: 22, group: "异常集群".into(), status: "warning".into(), ping_ms: 148 },
        DebugHostCard { id: "f-2".into(), name: "packet-loss-router".into(), address: "192.168.99.2".into(), port: 22, group: "异常集群".into(), status: "warning".into(), ping_ms: 280 },
        DebugHostCard { id: "f-3".into(), name: "power-loss-server".into(), address: "192.168.99.3".into(), port: 22, group: "异常集群".into(), status: "offline".into(), ping_ms: 0 },
        DebugHostCard { id: "f-4".into(), name: "disk-failure-storage".into(), address: "192.168.99.4".into(), port: 22, group: "异常集群".into(), status: "offline".into(), ping_ms: 0 },
    ];
    (tree, cards)
}
