use crate::domain::host::HostRecord;

/// 无界面依赖的核心运行态。
#[derive(Debug, Default)]
pub struct CoreState {
    /// 当前加载到内存中的主机记录。
    pub hosts: Vec<HostRecord>,
}

impl CoreState {
    /// 创建空的核心运行态。
    pub fn new() -> Self {
        tracing::debug!(target: "smagical_core", "初始化 CoreState 核心状态引擎");
        Self::default()
    }

    /// 插入一条用于界面骨架演示的本地主机记录。
    pub fn seed_example_host(&mut self) {
        tracing::info!(target: "smagical_core", "向 CoreState 注入初始演示主机资产: localhost (127.0.0.1:22)");
        self.hosts
            .push(HostRecord::new("localhost", "127.0.0.1", 22));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_empty_and_example_host_can_be_seeded() {
        let mut state = CoreState::new();
        assert!(state.hosts.is_empty());

        state.seed_example_host();
        assert_eq!(state.hosts.len(), 1);
        assert_eq!(state.hosts[0].address, "127.0.0.1");
        assert_eq!(state.hosts[0].port, 22);
    }
}
