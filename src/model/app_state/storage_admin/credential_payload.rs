use crate::model::{
    CertificateInspection, CredentialId, CredentialInspection, CredentialKind, KeyAlgorithm,
    SecretMaterialKind,
};
use russh::keys::{
    Algorithm, EcdsaCurve, HashAlg, PrivateKey,
    ssh_key::{
        Certificate,
        private::{KeypairData, RsaKeypair},
        rand_core::OsRng,
    },
};

const RSA_PRIVATE_KEY_BITS: usize = 2048;

pub(super) fn inspect_credential_payload(
    credential_id: CredentialId,
    kind: &CredentialKind,
    payload: &[u8],
) -> CredentialInspection {
    let mut inspection = CredentialInspection {
        credential_id,
        kind: kind.clone(),
        payload_hash: payload_hash(payload),
        parser_version: 1,
        parse_error: None,
        algorithm: detect_imported_key_algorithm(kind, payload),
        fingerprint: None,
        public_key: None,
        comment: None,
        encrypted: None,
        password_length: None,
        certificate: None,
    };

    match kind {
        CredentialKind::PrivateKey => inspect_private_key_payload(payload, &mut inspection),
        CredentialKind::Certificate => inspect_certificate_payload(payload, &mut inspection),
        CredentialKind::Password => {
            inspection.password_length = Some(password_payload_length(payload));
        }
        CredentialKind::Agent => {}
    }

    inspection
}

pub(super) fn credential_payload_validation_error(
    kind: &CredentialKind,
    inspection: &CredentialInspection,
) -> Option<String> {
    match kind {
        CredentialKind::PrivateKey | CredentialKind::Certificate => inspection.parse_error.clone(),
        CredentialKind::Password | CredentialKind::Agent => None,
    }
}

pub(super) fn decode_plaintext_private_key(
    payload: &[u8],
    display_name: &str,
) -> Result<PrivateKey, String> {
    let text =
        std::str::from_utf8(payload).map_err(|_| format!("{display_name}不是 UTF-8 文本"))?;

    PrivateKey::from_openssh(payload)
        .or_else(|_| russh::keys::decode_secret_key(text, None))
        .map_err(|error| format!("{display_name}不是可用的未加密 OpenSSH 私钥：{error}"))
}

pub(super) fn local_secret_kind_for_credential(
    kind: &CredentialKind,
) -> Option<SecretMaterialKind> {
    match kind {
        CredentialKind::Password => Some(SecretMaterialKind::Password),
        CredentialKind::PrivateKey => Some(SecretMaterialKind::PrivateKey),
        CredentialKind::Certificate => Some(SecretMaterialKind::Certificate),
        CredentialKind::Agent => None,
    }
}

pub(super) fn replacement_payload(kind: &CredentialKind, secret_text: String) -> Vec<u8> {
    match kind {
        CredentialKind::Password => secret_text.into_bytes(),
        CredentialKind::PrivateKey | CredentialKind::Certificate => {
            let mut payload = secret_text.trim().as_bytes().to_vec();
            if !payload.is_empty() && !payload.ends_with(b"\n") {
                payload.push(b'\n');
            }
            payload
        }
        CredentialKind::Agent => Vec::new(),
    }
}

pub(super) fn generate_private_key(
    name: &str,
    algorithm: &KeyAlgorithm,
) -> Result<PrivateKey, String> {
    let mut rng = OsRng;

    if matches!(algorithm, KeyAlgorithm::Rsa) {
        let keypair = RsaKeypair::random(&mut rng, RSA_PRIVATE_KEY_BITS)
            .map_err(|error| error.to_string())?;
        return PrivateKey::new(KeypairData::from(keypair), name)
            .map_err(|error| error.to_string());
    }

    let ssh_algorithm = key_algorithm_to_ssh_algorithm(algorithm)
        .ok_or_else(|| "unsupported algorithm".to_owned())?;
    let mut private_key =
        PrivateKey::random(&mut rng, ssh_algorithm).map_err(|error| error.to_string())?;
    private_key.set_comment(name);
    Ok(private_key)
}

fn inspect_private_key_payload(payload: &[u8], inspection: &mut CredentialInspection) {
    let text = std::str::from_utf8(payload).unwrap_or_default();
    inspection.encrypted = Some(private_key_text_looks_encrypted(text));

    match PrivateKey::from_openssh(payload).or_else(|_| russh::keys::decode_secret_key(text, None))
    {
        Ok(private_key) => {
            inspection.algorithm = Some(KeyAlgorithm::from_ssh_algorithm(
                private_key.algorithm().as_str(),
            ));
            inspection.fingerprint = Some(
                private_key
                    .public_key()
                    .fingerprint(HashAlg::Sha256)
                    .to_string(),
            );
            inspection.public_key = private_key.public_key().to_openssh().ok();
            let comment = private_key.comment().trim();
            if !comment.is_empty() {
                inspection.comment = Some(comment.to_owned());
            }
        }
        Err(error) => {
            inspection.parse_error = Some(format!("私钥解析失败：{error}"));
        }
    }
}

fn inspect_certificate_payload(payload: &[u8], inspection: &mut CredentialInspection) {
    let Ok(text) = std::str::from_utf8(payload) else {
        inspection.parse_error = Some("证书不是 UTF-8 文本".to_owned());
        return;
    };

    match Certificate::from_openssh(text) {
        Ok(certificate) => {
            inspection.algorithm = Some(KeyAlgorithm::from_ssh_algorithm(
                certificate.algorithm().as_str(),
            ));
            inspection.certificate = Some(CertificateInspection {
                cert_type: Some(format!("{:?}", certificate.cert_type())),
                serial: Some(certificate.serial()),
                key_id: Some(certificate.key_id().to_owned()),
                principals: certificate.valid_principals().to_vec(),
                valid_after_unix_secs: Some(certificate.valid_after()),
                valid_before_unix_secs: Some(certificate.valid_before()),
                ca_fingerprint: None,
                subject_fingerprint: None,
                critical_options_json: None,
                extensions_json: None,
            });
        }
        Err(error) => {
            inspection.parse_error = Some(format!("证书解析失败：{error}"));
        }
    }
}

fn payload_hash(payload: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in payload {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn password_payload_length(payload: &[u8]) -> usize {
    std::str::from_utf8(payload)
        .map(|password| password.chars().count())
        .unwrap_or(payload.len())
}

fn private_key_text_looks_encrypted(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("encrypted") || lower.contains("bcrypt")
}

fn key_algorithm_to_ssh_algorithm(algorithm: &KeyAlgorithm) -> Option<Algorithm> {
    match algorithm {
        KeyAlgorithm::Ed25519 => Some(Algorithm::Ed25519),
        KeyAlgorithm::Rsa => Some(Algorithm::Rsa { hash: None }),
        KeyAlgorithm::Ecdsa => Some(Algorithm::Ecdsa {
            curve: EcdsaCurve::NistP256,
        }),
        KeyAlgorithm::Unknown(_) => None,
    }
}

fn detect_imported_key_algorithm(kind: &CredentialKind, payload: &[u8]) -> Option<KeyAlgorithm> {
    match kind {
        CredentialKind::PrivateKey => detect_private_key_algorithm(payload),
        CredentialKind::Certificate => detect_certificate_algorithm(payload),
        CredentialKind::Password | CredentialKind::Agent => None,
    }
}

fn detect_private_key_algorithm(payload: &[u8]) -> Option<KeyAlgorithm> {
    let text = std::str::from_utf8(payload).ok()?;

    PrivateKey::from_openssh(payload)
        .ok()
        .or_else(|| russh::keys::decode_secret_key(text, None).ok())
        .map(|private_key| KeyAlgorithm::from_ssh_algorithm(private_key.algorithm().as_str()))
        .or_else(|| detect_key_algorithm_from_text(text))
}

fn detect_certificate_algorithm(payload: &[u8]) -> Option<KeyAlgorithm> {
    let text = std::str::from_utf8(payload).ok()?;

    Certificate::from_openssh(text)
        .ok()
        .map(|certificate| KeyAlgorithm::from_ssh_algorithm(certificate.algorithm().as_str()))
        .or_else(|| detect_key_algorithm_from_text(text))
}

fn detect_key_algorithm_from_text(text: &str) -> Option<KeyAlgorithm> {
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some(algorithm) = line
            .strip_prefix("PuTTY-User-Key-File-")
            .and_then(|line| line.split_once(':').map(|(_, algorithm)| algorithm.trim()))
            .and_then(key_algorithm_from_ssh_identifier)
        {
            return Some(algorithm);
        }

        if line == "-----BEGIN RSA PRIVATE KEY-----" {
            return Some(KeyAlgorithm::Rsa);
        }
        if line == "-----BEGIN EC PRIVATE KEY-----" {
            return Some(KeyAlgorithm::Ecdsa);
        }
        if line == "-----BEGIN DSA PRIVATE KEY-----" {
            return Some(KeyAlgorithm::Unknown("ssh-dss".to_owned()));
        }

        if let Some(algorithm) = line
            .split_whitespace()
            .next()
            .and_then(key_algorithm_from_ssh_identifier)
        {
            return Some(algorithm);
        }
    }

    None
}

fn key_algorithm_from_ssh_identifier(identifier: &str) -> Option<KeyAlgorithm> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return None;
    }

    let lower = identifier.to_ascii_lowercase();
    let looks_like_algorithm = lower.contains("ed25519")
        || lower.contains("rsa")
        || lower.contains("ecdsa")
        || lower.contains("ssh-dss")
        || lower.contains("sk-ssh")
        || lower.contains("cert-v01@openssh.com");

    looks_like_algorithm.then(|| KeyAlgorithm::from_ssh_algorithm(identifier))
}
