//! 凭据与凭据分组的核心动作。
//!
//! 这里聚合凭据元数据、分组、导入导出、内容替换与证书生成等逻辑。

#[path = "storage_admin/credential.rs"]
mod credential;
#[path = "storage_admin/credential_certificate_params.rs"]
mod credential_certificate_params;
#[path = "storage_admin/credential_groups.rs"]
mod credential_groups;
#[path = "storage_admin/credential_ids.rs"]
mod credential_ids;
#[path = "storage_admin/credential_material.rs"]
mod credential_material;
#[path = "storage_admin/credential_material_certificate.rs"]
mod credential_material_certificate;
#[path = "storage_admin/credential_material_generate.rs"]
mod credential_material_generate;
#[path = "storage_admin/credential_payload.rs"]
mod credential_payload;
#[path = "storage_admin/credential_refs.rs"]
pub(crate) mod credential_refs;
