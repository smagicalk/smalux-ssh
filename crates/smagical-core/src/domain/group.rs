use serde::{Deserialize, Serialize};

/// 主机分组/目录树节点领域模型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupRecord {
    /// 分组唯一标识符 (如 "grp-prod", "grp-k8s")。
    pub id: String,
    /// 面向用户显示的分组名称。
    pub name: String,
    /// 父级分组 ID (支持多层级嵌套，None 表示顶级根分组)。
    pub parent_id: Option<String>,
    /// 树形层级深度 (0 为顶级根节点, 1 为二级分组...)。
    pub level: i32,
    /// 是否处于展开状态。
    pub is_expanded: bool,
    /// 同级分组展示排序权重。
    pub sort_order: i32,
}

impl GroupRecord {
    /// 创建顶级分组记录。
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

    /// 创建子级分组记录。
    pub fn child(
        id: impl Into<String>,
        name: impl Into<String>,
        parent_id: impl Into<String>,
        level: i32,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_creation() {
        let root = GroupRecord::root("grp-prod", "生产集群");
        assert_eq!(root.id, "grp-prod");
        assert_eq!(root.parent_id, None);
        assert_eq!(root.level, 0);

        let child = GroupRecord::child("grp-k8s", "Kubernetes", "grp-prod", 1);
        assert_eq!(child.id, "grp-k8s");
        assert_eq!(child.parent_id, Some("grp-prod".to_string()));
        assert_eq!(child.level, 1);
    }
}
