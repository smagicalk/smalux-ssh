//! WebDAV 容灾备份驱动 (WebDAV Protocol Driver)
//!
//! 兼容坚果云、群晖 NAS、Nextcloud、Owncloud 及 Apache/Nginx WebDAV 模块。

use async_trait::async_trait;
use reqwest::{Client, Method, StatusCode};

use super::{BackupDriver, RemoteSnapshotInfo};

/// WebDAV 存储协议驱动
pub struct WebdavBackupDriver {
    client: Client,
    endpoint: String,
    user: String,
    pass: String,
}

impl WebdavBackupDriver {
    /// 构造 WebDAV 驱动实例
    pub fn new(endpoint: &str, user: &str, pass: &str) -> Self {
        let ep = endpoint.trim().trim_end_matches('/').to_string();
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            client,
            endpoint: ep,
            user: user.trim().to_string(),
            pass: pass.trim().to_string(),
        }
    }

    fn url_for(&self, path: &str) -> String {
        let p = path.trim_start_matches('/');
        if p.is_empty() {
            self.endpoint.clone()
        } else {
            format!("{}/{}", self.endpoint, p)
        }
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if !self.user.is_empty() || !self.pass.is_empty() {
            req.basic_auth(&self.user, Some(&self.pass))
        } else {
            req
        }
    }
}

#[async_trait]
impl BackupDriver for WebdavBackupDriver {
    async fn test_connection(&self) -> Result<(), String> {
        let url = self.url_for("");
        let req = self.client.request(Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Depth", "0");
        let req = self.apply_auth(req);

        let resp = req.send().await.map_err(|e| format!("无法连接至 WebDAV 服务器: {}", e))?;
        let status = resp.status();

        if status.is_success() || status == StatusCode::MULTI_STATUS {
            Ok(())
        } else if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            Err(format!("WebDAV 鉴权失败 (HTTP {})，请检查用户名与应用密码", status))
        } else {
            // 某些 WebDAV 服务器不支持对根路径 PROPFIND，尝试 HEAD
            let head_req = self.apply_auth(self.client.head(&url));
            if let Ok(head_resp) = head_req.send().await {
                if head_resp.status().is_success() {
                    return Ok(());
                }
            }
            Err(format!("WebDAV 响应异常: HTTP {}", status))
        }
    }

    async fn push_snapshot(&self, snapshot_id: &str, payload: &[u8]) -> Result<String, String> {
        let file_name = format!("snapshot_{}.json", snapshot_id);
        let url = self.url_for(&file_name);

        let req = self.client.put(&url)
            .header("Content-Type", "application/json")
            .body(payload.to_vec());
        let req = self.apply_auth(req);

        let resp = req.send().await.map_err(|e| format!("推送 WebDAV 快照失败: {}", e))?;
        let status = resp.status();

        if status.is_success() || status == StatusCode::CREATED || status == StatusCode::NO_CONTENT {
            Ok(file_name)
        } else {
            Err(format!("WebDAV 上传失败: HTTP {}", status))
        }
    }

    async fn list_snapshots(&self) -> Result<Vec<RemoteSnapshotInfo>, String> {
        let url = self.url_for("");
        let req = self.client.request(Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Depth", "1");
        let req = self.apply_auth(req);

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => return Err(format!("PROPFIND 请求失败: {}", e)),
        };

        if !resp.status().is_success() && resp.status() != StatusCode::MULTI_STATUS {
            return Err(format!("枚举快照失败: HTTP {}", resp.status()));
        }

        let body = resp.text().await.map_err(|e| e.to_string())?;

        // 简易轻量提取 href 与文件名 (避免庞大 XML 依赖)
        let mut list = Vec::new();
        for chunk in body.split("<d:response>").chain(body.split("<D:response>")) {
            if let Some(href_start) = chunk.find("<d:href>").or_else(|| chunk.find("<D:href>")) {
                let rest = &chunk[href_start..];
                if let Some(end) = rest.find("</d:href>").or_else(|| rest.find("</D:href>")) {
                    let full_tag = &rest[..end];
                    let href = full_tag.split('>').last().unwrap_or_default().trim();
                    let file_name = href.split('/').last().unwrap_or_default();
                    if file_name.starts_with("snapshot_") && file_name.ends_with(".json") {
                        // 提取长度与时间 (若有)
                        let size_bytes = chunk
                            .find("getcontentlength>")
                            .and_then(|idx| {
                                let sub = &chunk[idx..];
                                sub.split('<').next()?.split('>').last()?.parse::<u64>().ok()
                            })
                            .unwrap_or(0);

                        let epoch_secs = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        list.push(RemoteSnapshotInfo {
                            remote_id: file_name.to_string(),
                            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                            epoch_secs,
                            size_bytes,
                            hash: "sha256:remote".to_string(),
                        });
                    }
                }
            }
        }

        Ok(list)
    }

    async fn pull_snapshot(&self, remote_id: &str) -> Result<Vec<u8>, String> {
        let url = self.url_for(remote_id);
        let req = self.apply_auth(self.client.get(&url));

        let resp = req.send().await.map_err(|e| format!("拉取 WebDAV 快照失败: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("下载快照失败: HTTP {}", resp.status()));
        }

        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        Ok(bytes.to_vec())
    }

    async fn delete_snapshot(&self, remote_id: &str) -> Result<(), String> {
        let url = self.url_for(remote_id);
        let req = self.apply_auth(self.client.delete(&url));

        let resp = req.send().await.map_err(|e| format!("删除 WebDAV 快照失败: {}", e))?;
        if resp.status().is_success() || resp.status() == StatusCode::NO_CONTENT || resp.status() == StatusCode::NOT_FOUND {
            Ok(())
        } else {
            Err(format!("删除失败: HTTP {}", resp.status()))
        }
    }
}
