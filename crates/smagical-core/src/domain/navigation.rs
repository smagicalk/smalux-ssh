//! 统一页面跳转导航与历史返回栈领域模型。

use std::collections::HashMap;

/// 页面跳转意图请求。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NavigationRequest {
    /// 目标页面/抽屉标识 (如 "hosts", "credentials", "history", "snippets", "settings", "debug")
    pub target_tab: String,
    /// 可选的深层子路由/锚点定位 (如 "terminal_font", "appearance", "ssh_keys")
    pub sub_section: Option<String>,
    /// 携带的参数载荷 (如 {"highlight_id": "host-1", "filter": "prod"})
    pub params: HashMap<String, String>,
    /// 是否记录入历史返回栈 (默认 true)
    pub record_history: bool,
}

impl NavigationRequest {
    /// 创建一个指向指定页面的跳转请求。
    pub fn target(tab: impl Into<String>) -> Self {
        Self {
            target_tab: tab.into(),
            sub_section: None,
            params: HashMap::new(),
            record_history: true,
        }
    }

    /// 设置深层子路由/锚点。
    pub fn with_section(mut self, section: impl Into<String>) -> Self {
        self.sub_section = Some(section.into());
        self
    }

    /// 附加参数键值对。
    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// 标记为不记录入返回栈 (如单纯的重定向或自动恢复)。
    pub fn without_history(mut self) -> Self {
        self.record_history = false;
        self
    }
}

/// 页面导航历史记录栈管理器。
#[derive(Debug, Clone)]
pub struct NavigationRouter {
    /// 当前正处于活动状态的导航请求
    current: Option<NavigationRequest>,
    /// 后退历史栈
    back_stack: Vec<NavigationRequest>,
    /// 前进历史栈
    forward_stack: Vec<NavigationRequest>,
    /// 最大历史记录容量 (默认 50)
    max_history: usize,
}

impl Default for NavigationRouter {
    fn default() -> Self {
        Self::new(50)
    }
}

impl NavigationRouter {
    /// 创建一个导航路由器。
    pub fn new(max_history: usize) -> Self {
        Self {
            current: None,
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
            max_history,
        }
    }

    /// 获取当前活动导航请求。
    pub fn current(&self) -> Option<&NavigationRequest> {
        self.current.as_ref()
    }

    /// 执行页面跳转。
    ///
    /// 返回值：`(前一个被停用的请求, 当前新激活的请求)`
    pub fn navigate_to(&mut self, request: NavigationRequest) -> (Option<NavigationRequest>, NavigationRequest) {
        let previous = self.current.take();

        if let Some(prev) = previous.as_ref().filter(|p| p.record_history && p.target_tab != request.target_tab) {
            self.back_stack.push(prev.clone());
            if self.back_stack.len() > self.max_history {
                self.back_stack.remove(0);
            }
            // 一旦发起新跳转，清空 forward_stack
            self.forward_stack.clear();
        }



        self.current = Some(request.clone());
        (previous, request)
    }

    /// 后退导航 (Back)。
    ///
    /// 若有上一个页面，则将当前页面推入 forward_stack，并激活上一个页面。
    pub fn navigate_back(&mut self) -> Option<(NavigationRequest, NavigationRequest)> {
        if let (Some(target), Some(curr)) = (self.back_stack.pop(), self.current.take()) {
            self.forward_stack.push(curr.clone());
            self.current = Some(target.clone());
            return Some((curr, target));
        }
        None
    }

    /// 前进导航 (Forward)。
    pub fn navigate_forward(&mut self) -> Option<(NavigationRequest, NavigationRequest)> {
        if let (Some(target), Some(curr)) = (self.forward_stack.pop(), self.current.take()) {
            self.back_stack.push(curr.clone());
            self.current = Some(target.clone());
            return Some((curr, target));
        }
        None
    }


    /// 是否可以后退。
    pub fn can_go_back(&self) -> bool {
        !self.back_stack.is_empty()
    }

    /// 是否可以前进。
    pub fn can_go_forward(&self) -> bool {
        !self.forward_stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_router_flow_and_history() {
        let mut router = NavigationRouter::new(10);
        assert_eq!(router.current(), None);
        assert!(!router.can_go_back());

        // 1. 跳转到 hosts
        let (prev, curr) = router.navigate_to(NavigationRequest::target("hosts"));
        assert_eq!(prev, None);
        assert_eq!(curr.target_tab, "hosts");

        // 2. 跳转到 credentials (附带参数)
        let (prev, curr) = router.navigate_to(
            NavigationRequest::target("credentials")
                .with_section("ssh_keys")
                .with_param("highlight_id", "key-01"),
        );
        assert_eq!(prev.unwrap().target_tab, "hosts");
        assert_eq!(curr.target_tab, "credentials");
        assert_eq!(curr.sub_section.as_deref(), Some("ssh_keys"));
        assert_eq!(curr.params.get("highlight_id").map(|s| s.as_str()), Some("key-01"));
        assert!(router.can_go_back());

        // 3. 后退 (Back) 到 hosts
        let (to_deactivate, to_activate) = router.navigate_back().unwrap();
        assert_eq!(to_deactivate.target_tab, "credentials");
        assert_eq!(to_activate.target_tab, "hosts");
        assert_eq!(router.current().unwrap().target_tab, "hosts");
        assert!(router.can_go_forward());

        // 4. 前进 (Forward) 到 credentials
        let (to_deactivate, to_activate) = router.navigate_forward().unwrap();
        assert_eq!(to_deactivate.target_tab, "hosts");
        assert_eq!(to_activate.target_tab, "credentials");
        assert_eq!(router.current().unwrap().target_tab, "credentials");
    }
}

