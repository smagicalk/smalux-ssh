//! Hook 决策与控制流枚举。

/// 故障降级与自动重试策略。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FallbackStrategy {
    /// 自动降级为密码认证重试
    SwitchToPasswordAuth,
    /// 切换为备用跳板机链路
    SwitchToBackupProxy(String),
    /// 自动重试连接 (带退避延迟与最大重试次数)
    AutoReconnect {
        /// 重试延迟毫秒数
        delay_ms: u64,
        /// 最大重试次数
        max_retries: u32,
    },
}

/// Hook 决策控制流枚举。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookDecision<T = ()> {
    /// 正常放行：继续执行主流程
    Continue,

    /// 拦截并终止：中止后续所有操作，并向用户/日志展示原因
    Abort {
        /// 拦截原因
        reason: String,
    },

    /// 篡改/替换：使用修改后的数据继续后续流程 (例如输入宏替换)
    Modify(T),

    /// 异常恢复/重试：指示系统进行故障重试与自动降级
    RetryWithFallback(FallbackStrategy),

    /// 静默吸收：捕获并吞掉该异常 (不弹窗报错，由插件在后台处理)
    Swallow,
}

impl<T> HookDecision<T> {
    /// 判断是否允许继续执行。
    pub fn is_continue(&self) -> bool {
        matches!(self, Self::Continue)
    }

    /// 判断是否被拦截。
    pub fn is_aborted(&self) -> bool {
        matches!(self, Self::Abort { .. })
    }
}
