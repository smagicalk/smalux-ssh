use crate::model::{CredentialGroup, CredentialGroupId, CredentialKind};
use uuid::Uuid;

use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 创建根级密钥分组。
    pub(in crate::model::app_state) fn create_credential_group(
        &mut self,
        name: String,
        kind: CredentialKind,
        parent_id: Option<CredentialGroupId>,
    ) -> AppUpdateOutcome {
        let name = name.trim();

        if name.is_empty() {
            return AppUpdateOutcome {
                error: Some("密钥分组名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if let Some(parent_id) = parent_id {
            let parent = self
                .storage
                .credential_groups
                .iter()
                .find(|group| group.id == parent_id);
            if parent.is_none() {
                return AppUpdateOutcome {
                    error: Some("父级密钥分组不存在".to_owned()),
                    ..AppUpdateOutcome::default()
                };
            }
            if parent.is_some_and(|group| group.kind != kind) {
                return AppUpdateOutcome {
                    error: Some("父级密钥分组类型不一致".to_owned()),
                    ..AppUpdateOutcome::default()
                };
            }
        }

        let sort_order = self
            .storage
            .credential_groups
            .iter()
            .filter(|group| group.kind == kind && group.parent_id == parent_id)
            .count() as i32;
        self.storage.upsert_credential_group(CredentialGroup {
            id: CredentialGroupId(Uuid::new_v4()),
            name: name.to_owned(),
            kind,
            parent_id,
            sort_order,
        });

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 重命名一个自建密钥分组。
    pub(in crate::model::app_state) fn rename_credential_group(
        &mut self,
        group_id: CredentialGroupId,
        name: String,
    ) -> AppUpdateOutcome {
        let name = name.trim();

        if name.is_empty() {
            return AppUpdateOutcome {
                error: Some("密钥分组名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if self
            .storage
            .rename_credential_group(group_id, name.to_owned())
        {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some("找不到密钥分组".to_owned()),
                ..AppUpdateOutcome::default()
            }
        }
    }

    /// 删除一个自建密钥分组。
    pub(in crate::model::app_state) fn remove_credential_group(
        &mut self,
        group_id: CredentialGroupId,
    ) -> AppUpdateOutcome {
        if self.storage.credential_group_has_children(group_id) {
            return AppUpdateOutcome {
                error: Some("密钥分组非空，无法删除".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if self.storage.remove_credential_group(group_id) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some("找不到密钥分组".to_owned()),
                ..AppUpdateOutcome::default()
            }
        }
    }

    /// 移动凭据分组到同类型父分组或该类型根节点。
    pub(in crate::model::app_state) fn move_credential_group(
        &mut self,
        group_id: CredentialGroupId,
        parent_id: Option<CredentialGroupId>,
    ) -> AppUpdateOutcome {
        let Some(index) = self
            .storage
            .credential_groups
            .iter()
            .position(|group| group.id == group_id)
        else {
            return AppUpdateOutcome {
                error: Some("找不到密钥分组".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        if parent_id == Some(group_id) {
            return AppUpdateOutcome {
                error: Some("不能移动到自身分组下".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let kind = self.storage.credential_groups[index].kind.clone();
        if let Some(error) =
            validate_credential_group(&self.storage.credential_groups, parent_id, kind.clone())
        {
            return AppUpdateOutcome {
                error: Some(error),
                ..AppUpdateOutcome::default()
            };
        }

        if credential_group_is_descendant(&self.storage.credential_groups, parent_id, group_id) {
            return AppUpdateOutcome {
                error: Some("不能移动到子分组下".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if self.storage.credential_groups[index].parent_id == parent_id {
            return AppUpdateOutcome::default();
        }

        let sort_order = self
            .storage
            .credential_groups
            .iter()
            .filter(|group| group.kind == kind && group.parent_id == parent_id)
            .count() as i32;
        self.storage.credential_groups[index].parent_id = parent_id;
        self.storage.credential_groups[index].sort_order = sort_order;

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }
}

pub(super) fn validate_credential_group(
    groups: &[CredentialGroup],
    group_id: Option<CredentialGroupId>,
    kind: CredentialKind,
) -> Option<String> {
    let group_id = group_id?;
    let group = groups.iter().find(|group| group.id == group_id);
    if group.is_none() {
        return Some("密钥分组不存在".to_owned());
    }
    if group.is_some_and(|group| group.kind != kind) {
        return Some("密钥分组类型不匹配".to_owned());
    }
    None
}

pub(super) fn credential_group_is_descendant(
    groups: &[CredentialGroup],
    parent_id: Option<CredentialGroupId>,
    ancestor_id: CredentialGroupId,
) -> bool {
    let mut current = parent_id;
    while let Some(group_id) = current {
        if group_id == ancestor_id {
            return true;
        }
        current = groups
            .iter()
            .find(|group| group.id == group_id)
            .and_then(|group| group.parent_id);
    }
    false
}
