//! GitHub Gist 容灾备份驱动 (GitHub Gist Driver)
//!
//! 利用 GitHub Gist 的原生 Git 版本控制能力，自动留存每次快照的 Git Commit 历史版本。

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde_json::Value;

use super::{BackupDriver, RemoteSnapshotInfo};

/// GitHub Gist 存储驱动
pub struct GistBackupDriver {
    client: Client,
    gist_id: String,
    token: String,
}

impl GistBackupDriver {
    /// 构造 Gist 驱动实例
    pub fn new(gist_id: &str, token: &str) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            client,
            gist_id: gist_id.trim().to_string(),
            token: token.trim().to_string(),
        }
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header("User-Agent", "smalux-ssh")
            .header("Accept", "application/vnd.github+json")
            .bearer_auth(&self.token)
    }
}

#[async_trait]
impl BackupDriver for GistBackupDriver {
    async fn test_connection(&self) -> Result<(), String> {
        if self.token.is_empty() {
            return Err("GitHub Personal Access Token (PAT) 不能为空".to_string());
        }

        let url = if !self.gist_id.is_empty() {
            format!("https://api.github.com/gists/{}", self.gist_id)
        } else {
            "https://api.github.com/user".to_string()
        };

        let req = self.apply_auth(self.client.get(&url));
        let resp = req.send().await.map_err(|e| format!("连接 GitHub API 失败: {}", e))?;
        let status = resp.status();

        if status.is_success() {
            Ok(())
        } else if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            Err(format!("GitHub Token 无效或无 Gist 权限 (HTTP {})", status))
        } else if status == StatusCode::NOT_FOUND {
            Err(format!("目标 Gist ID「{}」不存在", self.gist_id))
        } else {
            Err(format!("GitHub API 响应异常: HTTP {}", status))
        }
    }

    async fn push_snapshot(&self, _snapshot_id: &str, payload: &[u8]) -> Result<String, String> {
        let content_str = String::from_utf8(payload.to_vec())
            .map_err(|e| format!("快照非 UTF-8 文本: {}", e))?;

        if self.gist_id.is_empty() {
            // 新建私有 Gist
            let body = serde_json::json!({
                "description": "Smalux SSH 自动容灾备份快照",
                "public": false,
                "files": {
                    "smalux_backup.json": {
                        "content": content_str
                    }
                }
            });

            let req = self.apply_auth(self.client.post("https://api.github.com/gists")).json(&body);
            let resp = req.send().await.map_err(|e| format!("创建 Gist 失败: {}", e))?;
            if !resp.status().is_success() {
                return Err(format!("创建 Gist 失败: HTTP {}", resp.status()));
            }

            let val: Value = resp.json().await.map_err(|e| e.to_string())?;
            let new_id = val.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            Ok(new_id.to_string())
        } else {
            // 更新现有 Gist
            let url = format!("https://api.github.com/gists/{}", self.gist_id);
            let body = serde_json::json!({
                "description": format!("Smalux SSH 自动容灾备份快照 ({})", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")),
                "files": {
                    "smalux_backup.json": {
                        "content": content_str
                    }
                }
            });

            let req = self.apply_auth(self.client.patch(&url)).json(&body);
            let resp = req.send().await.map_err(|e| format!("更新 Gist 失败: {}", e))?;
            if !resp.status().is_success() {
                return Err(format!("更新 Gist 失败: HTTP {}", resp.status()));
            }

            // 获取最新 commit sha
            let val: Value = resp.json().await.map_err(|e| e.to_string())?;
            let commit_sha = val.get("history")
                .and_then(|h| h.as_array())
                .and_then(|arr| arr.first())
                .and_then(|item| item.get("version"))
                .and_then(|v| v.as_str())
                .unwrap_or(&self.gist_id);

            Ok(commit_sha.to_string())
        }
    }

    async fn list_snapshots(&self) -> Result<Vec<RemoteSnapshotInfo>, String> {
        if self.gist_id.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("https://api.github.com/gists/{}", self.gist_id);
        let req = self.apply_auth(self.client.get(&url));
        let resp = req.send().await.map_err(|e| format!("获取 Gist 历史快照失败: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("获取 Gist 失败: HTTP {}", resp.status()));
        }

        let val: Value = resp.json().await.map_err(|e| e.to_string())?;
        let mut list = Vec::new();

        if let Some(history) = val.get("history").and_then(|h| h.as_array()) {
            for item in history {
                let version = item.get("version").and_then(|v| v.as_str()).unwrap_or_default();
                let committed_at = item.get("committed_at").and_then(|v| v.as_str()).unwrap_or_default();

                let epoch_secs = chrono::DateTime::parse_from_rfc3339(committed_at)
                    .map(|dt| dt.timestamp() as u64)
                    .unwrap_or_default();

                list.push(RemoteSnapshotInfo {
                    remote_id: version.to_string(),
                    timestamp: committed_at.replace('T', " ").replace('Z', ""),
                    epoch_secs,
                    size_bytes: 0,
                    hash: format!("sha256:gist:{}", &version[..version.len().min(8)]),
                });
            }
        }

        Ok(list)
    }

    async fn pull_snapshot(&self, remote_id: &str) -> Result<Vec<u8>, String> {
        if self.gist_id.is_empty() {
            return Err("未指定 Gist ID".to_string());
        }

        // 若 remote_id 类似 commit sha，则请求对应 revision
        let url = if remote_id.len() >= 20 {
            format!("https://api.github.com/gists/{}/{}", self.gist_id, remote_id)
        } else {
            format!("https://api.github.com/gists/{}", self.gist_id)
        };

        let req = self.apply_auth(self.client.get(&url));
        let resp = req.send().await.map_err(|e| format!("获取 Gist 版本数据失败: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("下载 Gist 快照失败: HTTP {}", resp.status()));
        }

        let val: Value = resp.json().await.map_err(|e| e.to_string())?;
        let content = val.get("files")
            .and_then(|f| f.get("smalux_backup.json"))
            .and_then(|f| f.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| "Gist 中未找到 smalux_backup.json 文件".to_string())?;

        Ok(content.as_bytes().to_vec())
    }

    async fn delete_snapshot(&self, _remote_id: &str) -> Result<(), String> {
        // Gist commit 历史受 Git 保护无法单独删除 commit，无操作返回成功
        Ok(())
    }
}
