//! `smagical-debug`
//!
//! smalux-ssh 专用的开发者调试控制面板、场景模拟预设生成器、批量数据生成/修改器、tracing 全局日志系统（控制台+文件滚动+UI捕获）与运行时探针工具库。

#![deny(missing_docs)]

pub mod batch;
pub mod inspector;
pub mod logger;
pub mod models;
pub mod presets;
pub mod tracing_layer;

pub use batch::{
    batch_update_port, batch_update_status, ensure_group_hierarchy, generate_batch_hosts,
    BatchGenerateConfig,
};
pub use inspector::{calculate_node_width, calculate_text_width, DebugRuntimeMetrics};
pub use logger::{get_current_timestamp, DebugLogBuffer};
pub use models::{DebugHostCard, DebugLogEntry, DebugLogLevel, DebugRawNode};
pub use presets::{get_preset_by_id, PresetKind};
pub use tracing_layer::{
    clean_expired_logs, get_default_log_dir, get_global_log_buffer, init_tracing,
    is_debug_enabled, set_debug_enabled, TracingGuard, UiLogLayer,
};


/// 开发者调试核心服务门面 (Debug Service Facade)
#[derive(Clone, Debug, Default)]
pub struct DebugService {
    /// 日志缓冲区
    pub logger: DebugLogBuffer,
}

impl DebugService {
    /// 创建新的调试服务
    pub fn new() -> Self {
        Self {
            logger: DebugLogBuffer::default(),
        }
    }

    /// 记录一条调试事件日志
    pub fn log(&mut self, level: &str, module: &str, message: &str) -> DebugLogEntry {
        self.logger.push(level, module, message)
    }

    /// 获取所有日志
    pub fn get_logs(&self) -> Vec<DebugLogEntry> {
        self.logger.get_all()
    }

    /// 清空所有日志
    pub fn clear_logs(&mut self) {
        self.logger.clear();
    }

    /// 注入指定预设场景
    pub fn inject_preset(&self, preset_id: &str) -> (Vec<DebugRawNode>, Vec<DebugHostCard>) {
        get_preset_by_id(preset_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_service_logging() {
        let mut service = DebugService::new();
        assert_eq!(service.logger.len(), 0);

        service.log("INFO", "SYS", "系统初始化测试");
        service.log("WARN", "NET", "网络延迟告警");
        service.log("ERROR", "SSH", "连接目标失败");

        assert_eq!(service.logger.len(), 3);
        let logs = service.get_logs();
        assert_eq!(logs[0].message, "连接目标失败");
        assert_eq!(logs[0].level, "ERROR");
        assert_eq!(logs[2].message, "系统初始化测试");

        service.clear_logs();
        assert_eq!(service.logger.len(), 0);
    }

    #[test]
    fn test_presets_generation() {
        let (tree_rich, cards_rich) = get_preset_by_id("rich_clusters");
        assert!(cards_rich.len() >= 30);
        assert!(tree_rich.iter().any(|n| n.is_group));

        let (tree_min, cards_min) = get_preset_by_id("minimal");
        assert_eq!(cards_min.len(), 3);
        assert_eq!(tree_min.len(), 4);

        let (tree_long, cards_long) = get_preset_by_id("long_names");
        assert!(!tree_long.is_empty());
        assert!(cards_long[0].name.len() > 30);

        let (tree_deep, _) = get_preset_by_id("deep_nested");
        assert!(tree_deep.iter().any(|n| n.level >= 4));

        let (_, cards_fault) = get_preset_by_id("faults");
        assert!(cards_fault.iter().any(|c| c.status == "warning" || c.status == "offline"));
    }

    #[test]
    fn test_batch_generation_and_modification() {
        let cfg = BatchGenerateConfig {
            name_prefix: "test-node-".to_string(),
            count: 20,
            start_index: 1,
            ip_prefix: "10.0.1.".to_string(),
            start_ip: 10,
            port: 22,
            group_name: "测试批量组".to_string(),
            status_mode: "online".to_string(),
        };
        let (mut tree, mut cards) = generate_batch_hosts(&cfg);
        assert_eq!(cards.len(), 20);
        assert_eq!(tree.len(), 21); // 1 group + 20 hosts

        // 批量修改状态为 warning
        batch_update_status(&mut tree, &mut cards, "warning");
        assert!(cards.iter().all(|c| c.status == "warning"));
        assert!(tree.iter().filter(|n| !n.is_group).all(|n| n.status == "warning"));

        // 批量修改端口为 2222
        batch_update_port(&mut tree, &mut cards, 2222);
        assert!(cards.iter().all(|c| c.port == 2222));
    }

    #[test]
    fn test_nested_group_hierarchy() {
        let cfg = BatchGenerateConfig {
            name_prefix: "k8s-pod-".to_string(),
            count: 5,
            start_index: 1,
            ip_prefix: "10.244.0.".to_string(),
            start_ip: 10,
            port: 22,
            group_name: "基础设施/亚太集群/杭州机房/POD-01".to_string(),
            status_mode: "online".to_string(),
        };
        let (tree, cards) = generate_batch_hosts(&cfg);
        assert_eq!(cards.len(), 5);
        // 4 级目录结构: 基础设施(0) -> 亚太集群(1) -> 杭州机房(2) -> POD-01(3) + 5 个主机(4)
        assert_eq!(tree.len(), 9);

        let root_grp = tree.iter().find(|n| n.name == "基础设施").unwrap();
        assert_eq!(root_grp.level, 0);
        assert_eq!(root_grp.parent_id, "");

        let sub_grp = tree.iter().find(|n| n.name == "亚太集群").unwrap();
        assert_eq!(sub_grp.level, 1);
        assert_eq!(sub_grp.parent_id, root_grp.id);

        let pod_grp = tree.iter().find(|n| n.name == "POD-01").unwrap();
        assert_eq!(pod_grp.level, 3);

        // 所有主机 level 为 4，归属于 pod_grp
        for host in tree.iter().filter(|n| !n.is_group) {
            assert_eq!(host.level, 4);
            assert_eq!(host.parent_id, pod_grp.id);
        }
    }

    #[test]
    fn test_inspector_width_calculation() {
        let w_short = calculate_node_width("prod-01", 0);
        let w_long = calculate_node_width("kubernetes-worker-node-apac-east-001-with-nvidia-h100", 3);
        assert!(w_long > w_short);
        assert!(w_short >= 100.0);
    }

    #[test]
    fn test_clean_expired_logs() {
        let temp_dir = std::env::temp_dir().join(format!("smalux_test_logs_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        // 创建 5 个模拟日志文件
        for i in 0..5 {
            let fpath = temp_dir.join(format!("smalux.log.2026-08-2{}", i));
            let _ = std::fs::write(&fpath, format!("log content {}", i));
        }

        // 保留最多 2 个文件
        clean_expired_logs(&temp_dir, 30, 2);

        let remaining = std::fs::read_dir(&temp_dir).unwrap().count();
        assert_eq!(remaining, 2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
