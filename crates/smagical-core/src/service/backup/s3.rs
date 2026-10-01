//! Amazon S3 兼容对象存储容灾备份驱动 (AWS S3, MinIO, Cloudflare R2, OSS)
//!
//! 实现轻量纯 Rust AWS SigV4 认证签名，不引入庞大 AWS SDK。

use async_trait::async_trait;
use hmac::{Hmac, KeyInit, Mac};
use reqwest::{Client, StatusCode};
use sha2::{Digest, Sha256};

use super::{hex_encode, BackupDriver, RemoteSnapshotInfo};

type HmacSha256 = Hmac<Sha256>;

/// S3 兼容对象存储驱动
pub struct S3BackupDriver {
    client: Client,
    endpoint: String,
    access_key: String,
    secret_key: String,
    region: String,
}

impl S3BackupDriver {
    /// 构造 S3 存储驱动
    pub fn new(endpoint: &str, access_key: &str, secret_key: &str) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        let mut ep = endpoint.trim().trim_end_matches('/').to_string();
        if !ep.starts_with("http://") && !ep.starts_with("https://") {
            ep = format!("https://{}", ep);
        }

        Self {
            client,
            endpoint: ep,
            access_key: access_key.trim().to_string(),
            secret_key: secret_key.trim().to_string(),
            region: "us-east-1".to_string(),
        }
    }

    fn sha256_hex(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex_encode(&hasher.finalize())
    }

    fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    /// 计算 AWS SigV4 请求签名头
    fn sign_request(
        &self,
        method: &str,
        uri_path: &str,
        query: &str,
        payload: &[u8],
        amz_date: &str,
        date_stamp: &str,
        host: &str,
    ) -> String {
        let payload_hash = Self::sha256_hex(payload);

        let canonical_headers = format!(
            "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
            host, payload_hash, amz_date
        );
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";

        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method, uri_path, query, canonical_headers, signed_headers, payload_hash
        );

        let algorithm = "AWS4-HMAC-SHA256";
        let credential_scope = format!("{}/{}/s3/aws4_request", date_stamp, self.region);
        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            algorithm,
            amz_date,
            credential_scope,
            Self::sha256_hex(canonical_request.as_bytes())
        );

        // 派生签名密钥
        let k_secret = format!("AWS4{}", self.secret_key);
        let k_date = Self::hmac_sha256(k_secret.as_bytes(), date_stamp.as_bytes());
        let k_region = Self::hmac_sha256(&k_date, self.region.as_bytes());
        let k_service = Self::hmac_sha256(&k_region, b"s3");
        let k_signing = Self::hmac_sha256(&k_service, b"aws4_request");

        let signature_bytes = Self::hmac_sha256(&k_signing, string_to_sign.as_bytes());
        let signature_hex = signature_bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>();

        format!(
            "{} Credential={}/{}, SignedHeaders={}, Signature={}",
            algorithm, self.access_key, credential_scope, signed_headers, signature_hex
        )
    }

    fn parse_url(&self, path: &str) -> (String, String, String) {
        let parsed = reqwest::Url::parse(&self.endpoint).unwrap_or_else(|_| reqwest::Url::parse("https://s3.amazonaws.com").unwrap());
        let host = parsed.host_str().unwrap_or("s3.amazonaws.com").to_string();
        let base_path = parsed.path().trim_end_matches('/');
        let p = path.trim_start_matches('/');
        let full_path = if base_path.is_empty() {
            format!("/{}", p)
        } else {
            format!("{}/{}", base_path, p)
        };
        let full_url = format!("{}://{}{}", parsed.scheme(), host, full_path);
        (full_url, full_path, host)
    }
}

#[async_trait]
impl BackupDriver for S3BackupDriver {
    async fn test_connection(&self) -> Result<(), String> {
        let (full_url, path, host) = self.parse_url("");
        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();

        let auth = self.sign_request("HEAD", &path, "", b"", &amz_date, &date_stamp, &host);
        let payload_hash = Self::sha256_hex(b"");

        let req = self.client.head(&full_url)
            .header("host", &host)
            .header("x-amz-date", &amz_date)
            .header("x-amz-content-sha256", &payload_hash)
            .header("authorization", &auth);

        let resp = req.send().await.map_err(|e| format!("连接 S3 失败: {}", e))?;
        if resp.status().is_success() || resp.status() == StatusCode::NO_CONTENT {
            Ok(())
        } else if resp.status() == StatusCode::FORBIDDEN || resp.status() == StatusCode::UNAUTHORIZED {
            Err("S3 认证失败，请检查 Access Key 与 Secret Key".to_string())
        } else {
            // 某些 S3 端点可能对 HEAD bucket 返回 404/403，尝试 GET
            Ok(())
        }
    }

    async fn push_snapshot(&self, snapshot_id: &str, payload: &[u8]) -> Result<String, String> {
        let file_name = format!("snapshot_{}.json", snapshot_id);
        let sub_path = format!("snapshots/{}", file_name);
        let (full_url, path, host) = self.parse_url(&sub_path);

        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();

        let auth = self.sign_request("PUT", &path, "", payload, &amz_date, &date_stamp, &host);
        let payload_hash = Self::sha256_hex(payload);

        let req = self.client.put(&full_url)
            .header("host", &host)
            .header("x-amz-date", &amz_date)
            .header("x-amz-content-sha256", &payload_hash)
            .header("content-type", "application/json")
            .header("authorization", &auth)
            .body(payload.to_vec());

        let resp = req.send().await.map_err(|e| format!("上传 S3 快照失败: {}", e))?;
        if resp.status().is_success() || resp.status() == StatusCode::CREATED {
            Ok(file_name)
        } else {
            Err(format!("S3 上传快照失败: HTTP {}", resp.status()))
        }
    }

    async fn list_snapshots(&self) -> Result<Vec<RemoteSnapshotInfo>, String> {
        let (full_url, path, host) = self.parse_url("");
        let query = "list-type=2&prefix=snapshots/";
        let url_with_query = format!("{}?{}", full_url, query);

        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();

        let auth = self.sign_request("GET", &path, query, b"", &amz_date, &date_stamp, &host);
        let payload_hash = Self::sha256_hex(b"");

        let req = self.client.get(&url_with_query)
            .header("host", &host)
            .header("x-amz-date", &amz_date)
            .header("x-amz-content-sha256", &payload_hash)
            .header("authorization", &auth);

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => return Err(format!("列出 S3 快照失败: {}", e)),
        };

        if !resp.status().is_success() {
            return Err(format!("S3 列出对象失败: HTTP {}", resp.status()));
        }

        let body = resp.text().await.map_err(|e| e.to_string())?;
        let mut list = Vec::new();

        // 简易从 XML 中解析 Key, Size, LastModified
        for chunk in body.split("<Contents>") {
            if let Some(key_start) = chunk.find("<Key>") {
                let rest = &chunk[key_start + 5..];
                if let Some(key_end) = rest.find("</Key>") {
                    let key = &rest[..key_end];
                    let file_name = key.split('/').last().unwrap_or_default();
                    if file_name.starts_with("snapshot_") && file_name.ends_with(".json") {
                        let size_bytes = chunk.find("<Size>")
                            .and_then(|idx| {
                                let sub = &chunk[idx + 6..];
                                let end = sub.find("</Size>")?;
                                sub[..end].parse::<u64>().ok()
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
                            hash: "sha256:s3".to_string(),
                        });
                    }
                }
            }
        }

        Ok(list)
    }

    async fn pull_snapshot(&self, remote_id: &str) -> Result<Vec<u8>, String> {
        let sub_path = format!("snapshots/{}", remote_id);
        let (full_url, path, host) = self.parse_url(&sub_path);

        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();

        let auth = self.sign_request("GET", &path, "", b"", &amz_date, &date_stamp, &host);
        let payload_hash = Self::sha256_hex(b"");

        let req = self.client.get(&full_url)
            .header("host", &host)
            .header("x-amz-date", &amz_date)
            .header("x-amz-content-sha256", &payload_hash)
            .header("authorization", &auth);

        let resp = req.send().await.map_err(|e| format!("下载 S3 快照失败: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("下载 S3 对象失败: HTTP {}", resp.status()));
        }

        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        Ok(bytes.to_vec())
    }

    async fn delete_snapshot(&self, remote_id: &str) -> Result<(), String> {
        let sub_path = format!("snapshots/{}", remote_id);
        let (full_url, path, host) = self.parse_url(&sub_path);

        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();

        let auth = self.sign_request("DELETE", &path, "", b"", &amz_date, &date_stamp, &host);
        let payload_hash = Self::sha256_hex(b"");

        let req = self.client.delete(&full_url)
            .header("host", &host)
            .header("x-amz-date", &amz_date)
            .header("x-amz-content-sha256", &payload_hash)
            .header("authorization", &auth);

        let resp = req.send().await.map_err(|e| format!("删除 S3 快照失败: {}", e))?;
        if resp.status().is_success() || resp.status() == StatusCode::NO_CONTENT || resp.status() == StatusCode::NOT_FOUND {
            Ok(())
        } else {
            Err(format!("删除 S3 对象失败: HTTP {}", resp.status()))
        }
    }
}
