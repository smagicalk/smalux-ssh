use std::time::{SystemTime, UNIX_EPOCH};

use russh::keys::ssh_key::certificate;

pub(super) fn parse_certificate_type(cert_type: &str) -> Result<certificate::CertType, String> {
    match cert_type.trim() {
        "" | "User" => Ok(certificate::CertType::User),
        "Host" => Ok(certificate::CertType::Host),
        _ => Err("证书类型无效".to_owned()),
    }
}

pub(super) fn parse_certificate_principals(principals: &str) -> Result<Vec<String>, String> {
    let mut parsed = Vec::new();
    for principal in principals
        .split(|character: char| character == ',' || character == ';' || character.is_whitespace())
        .map(str::trim)
        .filter(|principal| !principal.is_empty())
    {
        if !parsed.iter().any(|existing| existing == principal) {
            parsed.push(principal.to_owned());
        }
    }

    if parsed.is_empty() {
        Err("Principal 不能为空".to_owned())
    } else {
        Ok(parsed)
    }
}

pub(super) fn parse_certificate_valid_days(valid_days: &str) -> Result<u64, String> {
    let valid_days = valid_days.trim();
    let days = if valid_days.is_empty() {
        365
    } else {
        valid_days
            .parse::<u64>()
            .map_err(|_| "有效天数必须是正整数".to_owned())?
    };

    if days == 0 || days > 36_500 {
        Err("有效天数必须在 1 到 36500 之间".to_owned())
    } else {
        Ok(days)
    }
}

pub(super) fn parse_certificate_serial(serial: &str) -> Result<u64, String> {
    let serial = serial.trim();
    if serial.is_empty() {
        return current_unix_seconds();
    }

    serial
        .parse::<u64>()
        .map_err(|_| "序列号必须是非负整数".to_owned())
}

pub(super) fn current_unix_seconds() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("读取系统时间失败：{error}"))
}
