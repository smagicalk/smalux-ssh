//! 代码片段与多层层级分组领域模型。
//!
//! 支持代码片段的层级文件夹分类、多语言标记、动态模板占位符提取与参数化渲染。

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 动态模板参数变量定义 (从 `{{key}}` 或 `{{key:default}}` 中提取)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetVariable {
    /// 占位符变量唯一键名 (如 "port", "container_name")
    pub key: String,
    /// 显示标签文案
    pub label: String,
    /// 缺省预填默认值 (可选)
    pub default_value: Option<String>,
}

/// 代码片段层级文件夹分组实体
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetGroupRecord {
    /// 分组全局唯一 ID (如 "sgrp-docker", "sgrp-linux")
    pub id: String,
    /// 分组名称 (如 "Docker 容器运维", "K8s 集群管理")
    pub name: String,
    /// 父级分组 ID (None 表示顶级分组，支持无限级多层嵌套)
    pub parent_id: Option<String>,
    /// 嵌套深度层级 (0 为根目录)
    pub level: u32,
    /// 当前是否处于展开状态
    pub is_expanded: bool,
    /// 显示排序权重 (数字越小越靠前)
    pub sort_order: i32,
}

impl SnippetGroupRecord {
    /// 构造顶级根目录分组
    pub fn root(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            parent_id: None,
            level: 0,
            is_expanded: true,
            sort_order: 0,
        }
    }

    /// 构造子级嵌套分组
    pub fn child(
        id: impl Into<String>,
        name: impl Into<String>,
        parent_id: impl Into<String>,
        level: u32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            parent_id: Some(parent_id.into()),
            level,
            is_expanded: true,
            sort_order: 0,
        }
    }
}

/// 代码片段资产实体
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetRecord {
    /// 代码片段全局唯一 ID (如 "snip-docker-ps", "snip-nginx-reload")
    pub id: String,
    /// 所属文件夹分组 ID (None 表示存放在根目录下)
    pub parent_group_id: Option<String>,
    /// 代码片段标题名称
    pub title: String,
    /// 代码片段/脚本主体内容 (支持多行)
    pub content: String,
    /// 脚本语言类型 (如 "bash", "sh", "powershell", "python", "sql", "yaml")
    pub language: String,
    /// 标签分类列表 (如 ["docker", "ops", "monitor"])
    pub tags: Vec<String>,
    /// 注入终端后是否自动发送回车立即执行 (true: 立即执行; false: 仅粘贴到光标处)
    pub auto_execute: bool,
    /// 详细说明与使用备注
    pub description: String,
    /// 是否星标置顶
    pub is_favorite: bool,
    /// 显示排序权重
    pub sort_order: i32,
    /// 最后修改时间戳 (ISO 8601 格式)
    pub updated_at: String,
}

impl SnippetRecord {
    /// 创建一个新的代码片段
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
        language: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            parent_group_id: None,
            title: title.into(),
            content: content.into(),
            language: language.into(),
            tags: Vec::new(),
            auto_execute: true,
            description: String::new(),
            is_favorite: false,
            sort_order: 0,
            updated_at: "2026-09-01T20:00:00Z".to_string(),
        }
    }

    /// 设置所属文件夹分组
    pub fn with_group(mut self, group_id: impl Into<String>) -> Self {
        self.parent_group_id = Some(group_id.into());
        self
    }

    /// 设置标签列表
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// 设置描述说明
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 设置自动执行模式
    pub fn with_auto_execute(mut self, auto: bool) -> Self {
        self.auto_execute = auto;
        self
    }

    /// 从代码内容中自动提取所有动态参数占位符 `{{key}}` 或 `{{key:default}}`
    pub fn extract_variables(&self) -> Vec<SnippetVariable> {
        let mut vars = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let text = &self.content;

        let mut start_idx = 0;
        while let Some(open_pos) = text[start_idx..].find("{{") {
            let actual_open = start_idx + open_pos + 2;
            if let Some(close_pos) = text[actual_open..].find("}}") {
                let actual_close = actual_open + close_pos;
                let raw_token = text[actual_open..actual_close].trim();
                if !raw_token.is_empty() {
                    let (key, default_val) = if let Some((k, d)) = raw_token.split_once(':') {
                        (k.trim().to_string(), Some(d.trim().to_string()))
                    } else if let Some((k, d)) = raw_token.split_once('=') {
                        (k.trim().to_string(), Some(d.trim().to_string()))
                    } else {
                        (raw_token.to_string(), None)
                    };

                    if !key.is_empty() && !seen.contains(&key) {
                        seen.insert(key.clone());
                        vars.push(SnippetVariable {
                            label: key.clone(),
                            key,
                            default_value: default_val,
                        });
                    }
                }
                start_idx = actual_close + 2;
            } else {
                break;
            }
        }
        vars
    }

    /// 根据用户提供的参数映射表渲染生成最终的可执行命令字符串
    pub fn render_content(&self, params: &HashMap<String, String>) -> String {
        let mut rendered = self.content.clone();
        for var in self.extract_variables() {
            let user_val = params.get(&var.key)
                .cloned()
                .or_else(|| var.default_value.clone())
                .unwrap_or_default();

            // 替换无默认值形式 {{key}}
            let pattern1 = format!("{{{{{}}}}}", var.key);
            rendered = rendered.replace(&pattern1, &user_val);

            // 替换带默认值形式 {{key:...}}
            if let Some(ref def) = var.default_value {
                let pattern2 = format!("{{{{{}:{}}}}}", var.key, def);
                rendered = rendered.replace(&pattern2, &user_val);
                let pattern3 = format!("{{{{{}: {}}}}}", var.key, def);
                rendered = rendered.replace(&pattern3, &user_val);
                let pattern4 = format!("{{{{{}: {}}}}}", var.key, def);
                rendered = rendered.replace(&pattern4, &user_val);
                let pattern5 = format!("{{{{{}: {} }}}}", var.key, def);
                rendered = rendered.replace(&pattern5, &user_val);
                let pattern_eq = format!("{{{{{}= {}}}}}", var.key, def);
                rendered = rendered.replace(&pattern_eq, &user_val);
                let pattern_eq2 = format!("{{{{{}: {} }}}}", var.key, def);
                rendered = rendered.replace(&pattern_eq2, &user_val);
            }
        }
        rendered
    }
}

/// 动态参数记忆与智能预填模型。
///
/// 记录用户在执行含占位符命令片段时所填写的参数历史值。
/// 支持特定片段作用域（Snippet-Scoped）与全局同名变量兜底作用域（Global-Scoped），实现智能跨会话复用。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SnippetParamMemory {
    /// 片段专属参数映射: snippet_id -> (param_key -> value)
    pub scoped_params: HashMap<String, HashMap<String, String>>,
    /// 全局参数记忆映射 (用于跨片段同名变量智能推导，如 "port", "container", "user"): param_key -> value
    pub global_params: HashMap<String, String>,
}

impl SnippetParamMemory {
    /// 创建一个新的动态参数记忆实例
    pub fn new() -> Self {
        Self::default()
    }

    /// 记忆单条参数
    pub fn remember(&mut self, snippet_id: &str, key: &str, value: &str) {
        let val_trimmed = value.trim();
        if val_trimmed.is_empty() {
            return;
        }
        self.scoped_params
            .entry(snippet_id.to_string())
            .or_default()
            .insert(key.to_string(), val_trimmed.to_string());
        self.global_params.insert(key.to_string(), val_trimmed.to_string());
    }

    /// 批量记忆参数映射表
    pub fn remember_all(&mut self, snippet_id: &str, params: &HashMap<String, String>) {
        for (k, v) in params {
            self.remember(snippet_id, k, v);
        }
    }

    /// 智能解析预填参数值
    ///
    /// 优先级规则：
    /// 1. 当前片段上次填写的专属历史值 (Snippet-Scoped)
    /// 2. 全局最近针对该变量名使用过的历史值 (Global-Scoped)
    /// 3. 命令模板中定义的默认值 `{{key:default}}`
    /// 4. 空字符串兜底
    pub fn resolve_param(&self, snippet_id: &str, key: &str, default_value: Option<&str>) -> String {
        if let Some(val) = self.scoped_params.get(snippet_id).and_then(|m| m.get(key)) {
            if !val.trim().is_empty() {
                return val.clone();
            }
        }
        if let Some(val) = self.global_params.get(key) {
            if !val.trim().is_empty() {
                return val.clone();
            }
        }
        default_value.unwrap_or_default().to_string()
    }

    /// 针对指定代码片段的所有占位符，解析生成完整的参数填充字典
    pub fn resolve_all_params(&self, snippet: &SnippetRecord) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for var in snippet.extract_variables() {
            let val = self.resolve_param(&snippet.id, &var.key, var.default_value.as_deref());
            map.insert(var.key, val);
        }
        map
    }

    /// 清空指定片段的记忆
    pub fn clear_snippet(&mut self, snippet_id: &str) {
        self.scoped_params.remove(snippet_id);
    }

    /// 从 JSON 字符串反序列化
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// 单个代码片段的执行频次与调用度量
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnippetUsageRecord {
    /// 累计执行次数
    pub execution_count: u32,
    /// 最近一次执行的 Unix 时间戳 (秒)
    pub last_executed_at: u64,
    /// 最近一次执行的终端会话 ID
    pub last_session_id: Option<String>,
}

impl Default for SnippetUsageRecord {
    fn default() -> Self {
        Self {
            execution_count: 0,
            last_executed_at: 0,
            last_session_id: None,
        }
    }
}

/// 代码片段智能推荐与评分项
#[derive(Debug, Clone, PartialEq)]
pub struct SnippetScoredItem {
    /// 代码片段全局唯一标识符
    pub snippet_id: String,
    /// 综合智能推荐得分
    pub score: f64,
    /// 累计执行次数
    pub execution_count: u32,
    /// 最后执行时间戳 (秒)
    pub last_executed_at: u64,
    /// 是否为星标收藏项
    pub is_favorite: bool,
}

/// 代码片段使用度量追踪与智能推荐引擎
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SnippetUsageTracker {
    /// 片段 ID 到使用统计的映射
    pub usage: HashMap<String, SnippetUsageRecord>,
}

impl SnippetUsageTracker {
    /// 创建一个新的代码片段使用度量追踪器
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次代码片段注入执行
    pub fn record_execution(&mut self, snippet_id: &str, session_id: Option<&str>, now_timestamp: u64) {
        let entry = self.usage.entry(snippet_id.to_string()).or_default();
        entry.execution_count = entry.execution_count.saturating_add(1);
        entry.last_executed_at = now_timestamp;
        entry.last_session_id = session_id.map(|s| s.to_string());
    }

    /// 获取特定片段的执行次数
    pub fn get_count(&self, snippet_id: &str) -> u32 {
        self.usage.get(snippet_id).map(|u| u.execution_count).unwrap_or(0)
    }

    /// 获取特定片段的最后执行时间戳
    pub fn get_last_executed(&self, snippet_id: &str) -> u64 {
        self.usage.get(snippet_id).map(|u| u.last_executed_at).unwrap_or(0)
    }

    /// 计算代码片段的智能推荐权重得分
    ///
    /// 评分标准：
    /// - 星标收藏基础分: +10,000 分 (确保置顶收藏始终保持最高优先级)
    /// - 执行频次加权: count * 50 分 (高频常用命令随使用次数攀升)
    /// - 平滑时间衰减加分: 24 小时内使用最高 +500 分，按小时平滑衰减
    pub fn calculate_score(&self, snippet_id: &str, is_favorite: bool, now_timestamp: u64) -> f64 {
        let mut score = 0.0;
        if is_favorite {
            score += 10_000.0;
        }
        if let Some(record) = self.usage.get(snippet_id) {
            score += (record.execution_count as f64) * 50.0;
            if record.last_executed_at > 0 && now_timestamp >= record.last_executed_at {
                let diff_secs = now_timestamp - record.last_executed_at;
                let diff_hours = diff_secs as f64 / 3600.0;
                score += 500.0 / (1.0 + diff_hours * 0.1);
            }
        }
        score
    }

    /// 对代码片段实体列表进行智能评分并返回排序列表 (按 score 降序排列)
    pub fn rank_snippets(&self, snippets: &[SnippetRecord], now_timestamp: u64) -> Vec<SnippetScoredItem> {
        let mut scored: Vec<SnippetScoredItem> = snippets.iter().map(|s| {
            let score = self.calculate_score(&s.id, s.is_favorite, now_timestamp);
            let count = self.get_count(&s.id);
            let last_ts = self.get_last_executed(&s.id);
            SnippetScoredItem {
                snippet_id: s.id.clone(),
                score,
                execution_count: count,
                last_executed_at: last_ts,
                is_favorite: s.is_favorite,
            }
        }).collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// 从 JSON 字符串反序列化
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snippet_variable_extraction_and_rendering() {
        let snippet = SnippetRecord::new(
            "snip-docker-log",
            "Docker 日志查看",
            "docker logs -f --tail={{lines:100}} {{container_name}}",
            "bash",
        );

        let vars = snippet.extract_variables();
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[0].key, "lines");
        assert_eq!(vars[0].default_value, Some("100".to_string()));
        assert_eq!(vars[1].key, "container_name");
        assert_eq!(vars[1].default_value, None);

        let mut params = HashMap::new();
        params.insert("container_name".to_string(), "nginx-proxy".to_string());
        // lines 未提供，自动使用 default 100
        let rendered = snippet.render_content(&params);
        assert_eq!(rendered, "docker logs -f --tail=100 nginx-proxy");

        // lines 提供自定义值 50
        params.insert("lines".to_string(), "50".to_string());
        let rendered2 = snippet.render_content(&params);
        assert_eq!(rendered2, "docker logs -f --tail=50 nginx-proxy");
    }

    #[test]
    fn test_snippet_groups_hierarchy() {
        let root = SnippetGroupRecord::root("sgrp-ops", "常用运维");
        let child = SnippetGroupRecord::child("sgrp-ops-docker", "Docker", "sgrp-ops", 1);

        assert_eq!(root.level, 0);
        assert!(root.parent_id.is_none());
        assert_eq!(child.level, 1);
        assert_eq!(child.parent_id.as_deref(), Some("sgrp-ops"));
    }

    #[test]
    fn test_snippet_param_memory_scoping_and_resolution() {
        let mut memory = SnippetParamMemory::new();

        // 1. 无任何记忆时，使用模板 default
        let resolved = memory.resolve_param("snip-1", "port", Some("8080"));
        assert_eq!(resolved, "8080");

        // 2. 记忆 snip-1 的 port 为 9090
        memory.remember("snip-1", "port", "9090");
        assert_eq!(memory.resolve_param("snip-1", "port", Some("8080")), "9090");

        // 3. 另一片段 snip-2 解析 port，未专属设置过时命中全局兜底 (9090)
        assert_eq!(memory.resolve_param("snip-2", "port", Some("3000")), "9090");

        // 4. 为 snip-2 设置专属 port 为 4000
        memory.remember("snip-2", "port", "4000");
        assert_eq!(memory.resolve_param("snip-2", "port", Some("3000")), "4000");
        // snip-1 依然保持 9090
        assert_eq!(memory.resolve_param("snip-1", "port", Some("8080")), "9090");

        // 5. JSON 序列化与反序列化持久性测试
        let json = memory.to_json().expect("序列化成功");
        let restored = SnippetParamMemory::from_json(&json).expect("反序列化成功");
        assert_eq!(restored, memory);
    }

    #[test]
    fn test_snippet_usage_tracker_ranking() {
        let mut tracker = SnippetUsageTracker::new();
        let now = 1700000000u64;

        let s1 = SnippetRecord::new("snip-top", "查看 TOP", "top", "bash");
        let mut s2 = SnippetRecord::new("snip-restart", "重启服务", "systemctl restart nginx", "bash");
        s2.is_favorite = true; // 星标置顶
        let s3 = SnippetRecord::new("snip-df", "查看磁盘", "df -h", "bash");

        // s1 执行 5 次
        for _ in 0..5 {
            tracker.record_execution("snip-top", Some("sess-1"), now);
        }
        // s3 执行 1 次
        tracker.record_execution("snip-df", Some("sess-1"), now);

        assert_eq!(tracker.get_count("snip-top"), 5);
        assert_eq!(tracker.get_count("snip-df"), 1);
        assert_eq!(tracker.get_count("snip-restart"), 0);

        let ranked = tracker.rank_snippets(&[s1, s2, s3], now);
        // 星标 s2 得分最高 (>= 10,000)
        assert_eq!(ranked[0].snippet_id, "snip-restart");
        // 高频 s1 得分次之 (5 次 * 50 + 500)
        assert_eq!(ranked[1].snippet_id, "snip-top");
        // 低频 s3 随后
        assert_eq!(ranked[2].snippet_id, "snip-df");
    }
}

