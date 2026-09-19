//! Remote recovery synchronisation transport.
//!
//! This module owns the server endpoint, device identity, X25519 key pair and
//! bearer token.  They are kept in the operating system credential store and
//! are never returned to the Vue layer.  Evidence payloads are encrypted here
//! with an X25519-derived XChaCha20-Poly1305 key before they reach the relay.

use crate::qlogin::QLoginState;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use rand_core::{OsRng, RngCore};
use reqwest::{Client, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;
use x25519_dalek::{PublicKey, StaticSecret};

const CREDENTIAL_SERVICE: &str = "top.ehre.qzonearchive.remote-sync";
const ENDPOINT_ACCOUNT: &str = "endpoint";
const DEVICE_ID_ACCOUNT: &str = "device-identity";
const SERVER_DEVICE_ID_ACCOUNT: &str = "server-device-id";
const DEVICE_TOKEN_ACCOUNT: &str = "device-token";
const DEVICE_PRIVATE_KEY_ACCOUNT: &str = "device-private-key";
const DEVICE_PUBLIC_KEY_ACCOUNT: &str = "device-public-key";
const DEVICE_LABEL_ACCOUNT: &str = "device-label";
const ACCOUNT_UIN_ACCOUNT: &str = "account-uin";
const DEVICE_KEY_VERSION_ACCOUNT: &str = "device-key-version";

const REQUEST_TIMEOUT_SECONDS: u64 = 20;
const PROTOCOL_VERSION: i32 = 2;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSyncConfig {
    pub endpoint: Option<String>,
    pub device_id: Option<String>,
    pub server_device_id: Option<String>,
    pub label: Option<String>,
    pub account_uin: Option<String>,
    pub key_version: Option<i32>,
    pub registered: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteDeviceRegistration {
    pub endpoint: String,
    pub device_id: String,
    pub server_device_id: String,
    pub account_uin: String,
    pub key_version: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RegisterDeviceRequest {
    device_id: String,
    public_key: String,
    label: Option<String>,
    account_uin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RegisterDeviceResponse {
    device_id: Uuid,
    device_token: String,
    created_at: String,
    account_uin: Option<String>,
    key_version: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePairingInvitation {
    pub pairing_id: String,
    pub code: String,
    pub expires_at: String,
    pub initiator_device_id: String,
    pub initiator_public_key: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePairing {
    pub pairing_id: String,
    pub initiator_device_id: String,
    pub initiator_public_key: Option<String>,
    pub claimant_device_id: Option<String>,
    pub claimant_public_key: Option<String>,
    pub status: String,
    pub expires_at: String,
    pub created_at: String,
    pub accepted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteEncryptedChange {
    pub record_id: String,
    pub operation: String,
    pub revision: i64,
    pub key_version: i32,
    pub sender_key_version: i32,
    pub target_uin: String,
    pub source_uin: Option<String>,
    pub ciphertext_b64: String,
    pub nonce_b64: String,
    pub aad_b64: Option<String>,
    pub payload_digest: String,
    pub changed_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSyncCursor {
    pub stream_id: String,
    pub position: String,
    pub issued_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSyncConflict {
    pub record_id: String,
    pub local_revision: i64,
    pub remote_revision: i64,
    pub local_digest: String,
    pub remote_digest: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePushResponse {
    pub accepted: usize,
    pub rejected: usize,
    pub next_cursor: RemoteSyncCursor,
    pub conflicts: Vec<RemoteSyncConflict>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePullResponse {
    pub changes: Vec<RemoteEncryptedChange>,
    pub next_cursor: RemoteSyncCursor,
    pub has_more: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptRemotePayloadRequest {
    target_uin: String,
    peer_public_key: String,
    target_key_version: i32,
    sender_key_version: i32,
    record_id: String,
    operation: String,
    revision: i64,
    payload: serde_json::Value,
    deleted_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecryptRemotePayloadRequest {
    source_uin: String,
    change: RemoteEncryptedChange,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptRemoteBatchRequest {
    target_uin: String,
    peer_public_key: String,
    target_key_version: i32,
    sender_key_version: i32,
    package: serde_json::Value,
    observations: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct PushChangesRequest<'a> {
    changes: &'a [RemoteEncryptedChange],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullChangesResponse {
    changes: Vec<RemoteEncryptedChange>,
    next_cursor: RemoteSyncCursor,
    has_more: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PushChangesResponse {
    accepted: usize,
    rejected: usize,
    next_cursor: RemoteSyncCursor,
    conflicts: Vec<RemoteSyncConflict>,
}

#[derive(Debug, Serialize)]
struct AckChangesRequest<'a> {
    cursor: &'a str,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
struct CreatePairingResponse {
    pairing_id: Uuid,
    code: String,
    expires_at: String,
    initiator_device_id: Uuid,
    initiator_public_key: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
struct PairingResponse {
    id: Uuid,
    initiator_device_id: Uuid,
    initiator_public_key: Option<String>,
    claimant_device_id: Option<Uuid>,
    claimant_public_key: Option<String>,
    status: String,
    expires_at: String,
    created_at: String,
    accepted_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PairingListResponse {
    pairings: Vec<PairingResponse>,
}

#[derive(Debug, Clone, Serialize)]
struct ClaimPairingRequest {
    code: String,
    invitee_public_key: String,
}

#[tauri::command]
pub fn get_remote_sync_config() -> Result<RemoteSyncConfig, String> {
    let endpoint = read_secret(ENDPOINT_ACCOUNT)?;
    let device_id = read_secret(DEVICE_ID_ACCOUNT)?;
    let server_device_id = read_secret(SERVER_DEVICE_ID_ACCOUNT)?;
    let label = read_secret(DEVICE_LABEL_ACCOUNT)?;
    let account_uin = read_secret(ACCOUNT_UIN_ACCOUNT)?;
    let key_version =
        read_secret(DEVICE_KEY_VERSION_ACCOUNT)?.and_then(|value| value.parse::<i32>().ok());
    let registered = read_secret(DEVICE_TOKEN_ACCOUNT)?.is_some()
        && read_secret(DEVICE_PRIVATE_KEY_ACCOUNT)?.is_some()
        && read_secret(DEVICE_PUBLIC_KEY_ACCOUNT)?.is_some();
    Ok(RemoteSyncConfig {
        endpoint,
        device_id,
        server_device_id,
        label,
        account_uin,
        key_version,
        registered,
    })
}

#[tauri::command]
pub fn save_remote_sync_endpoint(endpoint: String) -> Result<RemoteSyncConfig, String> {
    let endpoint = normalize_endpoint(&endpoint)?;
    write_secret(ENDPOINT_ACCOUNT, &endpoint)?;
    get_remote_sync_config()
}

#[tauri::command]
pub async fn register_remote_device(
    endpoint: String,
    account_uin: String,
    label: Option<String>,
) -> Result<RemoteDeviceRegistration, String> {
    let endpoint = normalize_endpoint(&endpoint)?;
    let account_uin = account_uin.trim().to_owned();
    if !valid_account_uin(&account_uin) {
        return Err("QQ 号只能包含数字，且不能超过 32 位".into());
    }
    let label = label
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    let (private_key, public_key) = load_or_create_keypair()?;

    let mut attempts = 0;
    loop {
        attempts += 1;
        let device_id = match read_secret(DEVICE_ID_ACCOUNT)? {
            Some(value) if !value.trim().is_empty() => value,
            _ => Uuid::new_v4().to_string(),
        };
        let client = http_client()?;
        let response = client
            .post(format!("{endpoint}/v1/devices"))
            .json(&RegisterDeviceRequest {
                device_id: device_id.clone(),
                public_key: public_key.clone(),
                label: label.clone(),
                account_uin: account_uin.clone(),
            })
            .send()
            .await
            .map_err(|error| format!("连接远程同步服务器失败：{error}"))?;
        if response.status() == StatusCode::CONFLICT {
            if attempts >= 3 {
                return Err("设备标识与其他账号冲突，请清除远程同步凭据后重试".into());
            }
            continue;
        }
        let response = checked_response(response).await?;
        let registered: RegisterDeviceResponse = response
            .json()
            .await
            .map_err(|error| format!("解析服务器注册响应失败：{error}"))?;

        write_secret(ENDPOINT_ACCOUNT, &endpoint)?;
        write_secret(DEVICE_ID_ACCOUNT, &device_id)?;
        write_secret(SERVER_DEVICE_ID_ACCOUNT, &registered.device_id.to_string())?;
        write_secret(DEVICE_TOKEN_ACCOUNT, &registered.device_token)?;
        write_secret(DEVICE_PRIVATE_KEY_ACCOUNT, &private_key)?;
        write_secret(DEVICE_PUBLIC_KEY_ACCOUNT, &public_key)?;
        write_secret(ACCOUNT_UIN_ACCOUNT, &account_uin)?;
        write_secret(
            DEVICE_KEY_VERSION_ACCOUNT,
            &registered.key_version.to_string(),
        )?;
        if let Some(label) = label.as_deref() {
            write_secret(DEVICE_LABEL_ACCOUNT, label)?;
        }

        return Ok(RemoteDeviceRegistration {
            endpoint,
            device_id,
            server_device_id: registered.device_id.to_string(),
            account_uin,
            key_version: registered.key_version,
            created_at: registered.created_at,
        });
    }
}

#[tauri::command]
pub async fn create_remote_pairing() -> Result<RemotePairingInvitation, String> {
    let endpoint = require_endpoint()?;
    let token = require_device_token()?;
    let response = authenticated_client(&token)?
        .post(format!("{endpoint}/v1/pairings"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|error| format!("创建配对邀请失败：{error}"))?;
    let response = checked_response(response).await?;
    let pairing: CreatePairingResponse = response
        .json()
        .await
        .map_err(|error| format!("解析配对邀请失败：{error}"))?;
    Ok(RemotePairingInvitation {
        pairing_id: pairing.pairing_id.to_string(),
        code: pairing.code,
        expires_at: pairing.expires_at,
        initiator_device_id: pairing.initiator_device_id.to_string(),
        initiator_public_key: pairing.initiator_public_key,
    })
}

#[tauri::command]
pub async fn claim_remote_pairing(code: String) -> Result<RemotePairing, String> {
    let endpoint = require_endpoint()?;
    let token = require_device_token()?;
    let public_key = read_secret(DEVICE_PUBLIC_KEY_ACCOUNT)?
        .ok_or_else(|| "当前设备尚未完成注册".to_string())?;
    let code = code.trim().to_uppercase();
    if code.len() != 10 {
        return Err("配对码应为 10 位字符".into());
    }
    let response = authenticated_client(&token)?
        .post(format!("{endpoint}/v1/pairings/claim"))
        .bearer_auth(&token)
        .json(&ClaimPairingRequest {
            code,
            invitee_public_key: public_key,
        })
        .send()
        .await
        .map_err(|error| format!("接受配对邀请失败：{error}"))?;
    let response = checked_response(response).await?;
    let pairing: PairingResponse = response
        .json()
        .await
        .map_err(|error| format!("解析配对结果失败：{error}"))?;
    Ok(pairing.into())
}

#[tauri::command]
pub async fn list_remote_pairings() -> Result<Vec<RemotePairing>, String> {
    let endpoint = require_endpoint()?;
    let token = require_device_token()?;
    let response = authenticated_client(&token)?
        .get(format!("{endpoint}/v1/pairings"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|error| format!("读取配对列表失败：{error}"))?;
    let response = checked_response(response).await?;
    let pairings: PairingListResponse = response
        .json()
        .await
        .map_err(|error| format!("解析配对列表失败：{error}"))?;
    Ok(pairings.pairings.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub fn encrypt_remote_payload(
    request: EncryptRemotePayloadRequest,
) -> Result<RemoteEncryptedChange, String> {
    if request.record_id.trim().is_empty() || request.record_id.len() > 512 {
        return Err("记录标识无效".into());
    }
    if request.operation != "upsert" && request.operation != "tombstone" {
        return Err("同步操作类型无效".into());
    }
    if request.revision < 0
        || !(1..=100).contains(&request.target_key_version)
        || !(1..=100).contains(&request.sender_key_version)
    {
        return Err("记录版本或密钥版本无效".into());
    }
    let source_uin = require_account_uin()?;
    let key = derive_account_key(
        &request.target_uin,
        &request.peer_public_key,
        request.target_key_version,
    )?;

    let aad = format!(
        "protocolVersion={PROTOCOL_VERSION};sourceUin={source_uin};sourceKeyVersion={};targetUin={};targetKeyVersion={};recordId={};revision={};operation={}",
        request.sender_key_version,
        request.target_uin,
        request.target_key_version,
        request.record_id,
        request.revision,
        request.operation
    )
    .into_bytes();
    let plaintext = serde_json::to_vec(&request.payload)
        .map_err(|error| format!("序列化待同步记录失败：{error}"))?;
    let mut nonce = [0_u8; 24];
    OsRng.fill_bytes(&mut nonce);
    let cipher = XChaCha20Poly1305::new((&key).into());
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| "加密同步记录失败".to_string())?;
    let payload_digest = format!("{:x}", Sha256::digest(&ciphertext));

    Ok(RemoteEncryptedChange {
        record_id: request.record_id,
        operation: request.operation,
        revision: request.revision,
        key_version: request.target_key_version,
        sender_key_version: request.sender_key_version,
        target_uin: request.target_uin,
        source_uin: Some(source_uin),
        ciphertext_b64: BASE64.encode(ciphertext),
        nonce_b64: BASE64.encode(nonce),
        aad_b64: Some(BASE64.encode(aad)),
        payload_digest,
        changed_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "本机时间无效".to_string())?
            .as_millis() as i64,
        deleted_at: request.deleted_at,
    })
}

#[tauri::command]
pub async fn decrypt_remote_payload(
    request: DecryptRemotePayloadRequest,
) -> Result<serde_json::Value, String> {
    let change = request.change;
    if change.operation != "upsert" && change.operation != "tombstone" {
        return Err("同步操作类型无效".into());
    }
    if change.revision < 0
        || !(1..=100).contains(&change.key_version)
        || !(1..=100).contains(&change.sender_key_version)
    {
        return Err("记录版本或密钥版本无效".into());
    }
    let local_uin = require_account_uin()?;
    let local_key_version = current_key_version()?;
    if change.key_version != local_key_version {
        return Err("该记录加密目标不是本设备密钥版本".into());
    }
    let sender_uin = if request.source_uin.trim().is_empty() {
        change.source_uin.as_deref().unwrap_or_default()
    } else {
        request.source_uin.as_str()
    };
    if sender_uin.is_empty() {
        return Err("同步记录缺少发送方账号".into());
    }
    let aad = change
        .aad_b64
        .as_deref()
        .ok_or_else(|| "同步记录缺少 AAD".to_string())
        .and_then(|value| {
            BASE64
                .decode(value)
                .map_err(|_| "同步记录的 AAD 不是有效 Base64".to_string())
        })?;
    let expected_aad = format!(
        "protocolVersion={PROTOCOL_VERSION};sourceUin={};sourceKeyVersion={};targetUin={};targetKeyVersion={};recordId={};revision={};operation={}",
        sender_uin,
        change.sender_key_version,
        local_uin,
        change.key_version,
        change.record_id,
        change.revision,
        change.operation
    )
    .into_bytes();
    if aad != expected_aad {
        return Err("同步记录 AAD 校验失败".into());
    }
    let nonce = BASE64
        .decode(&change.nonce_b64)
        .map_err(|_| "同步记录随机数不是有效 Base64".to_string())?;
    let nonce: [u8; 24] = nonce
        .try_into()
        .map_err(|_| "同步记录随机数长度无效".to_string())?;
    let ciphertext = BASE64
        .decode(&change.ciphertext_b64)
        .map_err(|_| "同步密文不是有效 Base64".to_string())?;
    let digest = format!("{:x}", Sha256::digest(&ciphertext));
    if digest != change.payload_digest {
        return Err("同步密文摘要校验失败".into());
    }
    let sender_keys = get_account_public_keys(sender_uin.to_string()).await?;
    let sender_key = sender_keys
        .iter()
        .find(|key| key.key_version == change.sender_key_version)
        .ok_or_else(|| "找不到发送方对应密钥版本，可能已重装或换钥".to_string())?;
    let key = derive_account_key(
        &local_uin,
        &sender_key.public_key,
        change.sender_key_version,
    )?;
    let cipher = XChaCha20Poly1305::new((&key).into());
    let plaintext = cipher
        .decrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: &aad,
            },
        )
        .map_err(|_| "同步记录解密失败，可能来自其他发送方或已被篡改".to_string())?;
    serde_json::from_slice(&plaintext).map_err(|_| "同步记录内容不是有效 JSON".to_string())
}

#[tauri::command]
pub fn encrypt_recovery_sync_batch(
    request: EncryptRemoteBatchRequest,
) -> Result<Vec<RemoteEncryptedChange>, String> {
    if request.observations.is_empty() || request.observations.len() > 100 {
        return Err("每批加密记录需要包含 1 到 100 条".into());
    }
    let package_id = request
        .package
        .get("packageId")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "同步证据包缺少编号".to_string())?
        .to_owned();
    let mut changes = Vec::with_capacity(request.observations.len());
    for observation in request.observations {
        let record_id = observation
            .get("eventKey")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "同步证据记录缺少稳定编号".to_string())?
            .to_owned();
        let payload = serde_json::json!({
            "schemaVersion": request.package.get("schemaVersion").cloned().unwrap_or(serde_json::Value::from(PROTOCOL_VERSION)),
            "packageId": package_id,
            "exporterUin": request.package.get("exporterUin"),
            "targetUin": request.package.get("targetUin"),
            "createdAt": request.package.get("createdAt"),
            "observation": observation,
        });
        changes.push(encrypt_remote_payload(EncryptRemotePayloadRequest {
            target_uin: request.target_uin.clone(),
            peer_public_key: request.peer_public_key.clone(),
            target_key_version: request.target_key_version,
            sender_key_version: request.sender_key_version,
            record_id,
            operation: "upsert".into(),
            revision: 1,
            payload,
            deleted_at: None,
        })?);
    }
    Ok(changes)
}

fn derive_account_key(
    target_uin: &str,
    peer_public_key: &str,
    peer_key_version: i32,
) -> Result<[u8; 32], String> {
    let peer_bytes = BASE64
        .decode(peer_public_key.trim())
        .map_err(|_| "对端公钥不是有效 Base64".to_string())?;
    let peer_bytes: [u8; 32] = peer_bytes
        .try_into()
        .map_err(|_| "对端公钥长度无效".to_string())?;
    let private_key = read_secret(DEVICE_PRIVATE_KEY_ACCOUNT)?
        .ok_or_else(|| "当前设备尚未生成同步密钥".to_string())?;
    let private_bytes = BASE64
        .decode(private_key)
        .map_err(|_| "本机同步私钥损坏".to_string())?;
    let private_bytes: [u8; 32] = private_bytes
        .try_into()
        .map_err(|_| "本机同步私钥长度无效".to_string())?;

    let private = StaticSecret::from(private_bytes);
    let shared = private.diffie_hellman(&PublicKey::from(peer_bytes));
    let hkdf = Hkdf::<Sha256>::new(None, shared.as_bytes());
    let context =
        format!("qzonearchive/recovery-sync/v2:target={target_uin}:keyVersion={peer_key_version}");
    let mut key = [0_u8; 32];
    hkdf.expand(context.as_bytes(), &mut key)
        .map_err(|_| "派生同步密钥失败".to_string())?;
    Ok(key)
}

fn valid_account_uin(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value.chars().all(|character| character.is_ascii_digit())
}

fn require_account_uin() -> Result<String, String> {
    read_secret(ACCOUNT_UIN_ACCOUNT)?.ok_or_else(|| "当前设备尚未绑定 QQ 账号".to_string())
}

fn current_key_version() -> Result<i32, String> {
    read_secret(DEVICE_KEY_VERSION_ACCOUNT)?
        .ok_or_else(|| "当前设备缺少同步密钥版本".to_string())?
        .parse::<i32>()
        .map_err(|_| "本机同步密钥版本损坏".to_string())
}

#[tauri::command]
pub async fn push_remote_changes(
    target_uin: String,
    changes: Vec<RemoteEncryptedChange>,
) -> Result<RemotePushResponse, String> {
    if changes.is_empty() || changes.len() > 100 {
        return Err("每次同步需要包含 1 到 100 条变更".into());
    }
    if changes.iter().any(|change| change.target_uin != target_uin) {
        return Err("一批变更必须发给同一目标账号".into());
    }
    let endpoint = require_endpoint()?;
    let token = require_device_token()?;
    let response = authenticated_client(&token)?
        .post(format!("{endpoint}/v1/sync/changes"))
        .bearer_auth(&token)
        .json(&PushChangesRequest { changes: &changes })
        .send()
        .await
        .map_err(|error| format!("上传加密同步变更失败：{error}"))?;
    let response = checked_response(response).await?;
    let result: PushChangesResponse = response
        .json()
        .await
        .map_err(|error| format!("解析上传同步响应失败：{error}"))?;
    Ok(RemotePushResponse {
        accepted: result.accepted,
        rejected: result.rejected,
        next_cursor: result.next_cursor,
        conflicts: result.conflicts,
    })
}

#[tauri::command]
pub async fn pull_remote_changes(
    cursor: Option<String>,
    limit: Option<u16>,
) -> Result<RemotePullResponse, String> {
    let endpoint = require_endpoint()?;
    let token = require_device_token()?;
    let mut request = authenticated_client(&token)?
        .get(format!("{endpoint}/v1/sync/changes"))
        .bearer_auth(&token);
    let mut query = Vec::new();
    if let Some(cursor) = cursor.as_deref().filter(|value| !value.trim().is_empty()) {
        query.push(("cursor", cursor.to_owned()));
    }
    if let Some(limit) = limit {
        query.push(("limit", limit.clamp(1, 500).to_string()));
    }
    if !query.is_empty() {
        request = request.query(&query);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("拉取加密同步变更失败：{error}"))?;
    let response = checked_response(response).await?;
    let result: PullChangesResponse = response
        .json()
        .await
        .map_err(|error| format!("解析拉取同步响应失败：{error}"))?;
    Ok(RemotePullResponse {
        changes: result.changes,
        next_cursor: result.next_cursor,
        has_more: result.has_more,
    })
}

#[tauri::command]
pub async fn ack_remote_changes(cursor: String) -> Result<(), String> {
    let endpoint = require_endpoint()?;
    let token = require_device_token()?;
    let response = authenticated_client(&token)?
        .post(format!("{endpoint}/v1/sync/ack"))
        .bearer_auth(&token)
        .json(&AckChangesRequest { cursor: &cursor })
        .send()
        .await
        .map_err(|error| format!("确认同步游标失败：{error}"))?;
    checked_response(response).await?;
    Ok(())
}

#[tauri::command]
pub fn clear_remote_sync_credentials() -> Result<(), String> {
    for account in [
        ENDPOINT_ACCOUNT,
        DEVICE_ID_ACCOUNT,
        SERVER_DEVICE_ID_ACCOUNT,
        DEVICE_TOKEN_ACCOUNT,
        DEVICE_PRIVATE_KEY_ACCOUNT,
        DEVICE_PUBLIC_KEY_ACCOUNT,
        DEVICE_LABEL_ACCOUNT,
        ACCOUNT_UIN_ACCOUNT,
        DEVICE_KEY_VERSION_ACCOUNT,
    ] {
        delete_secret(account)?;
    }
    Ok(())
}

impl From<PairingResponse> for RemotePairing {
    fn from(value: PairingResponse) -> Self {
        Self {
            pairing_id: value.id.to_string(),
            initiator_device_id: value.initiator_device_id.to_string(),
            initiator_public_key: value.initiator_public_key,
            claimant_device_id: value.claimant_device_id.map(|id| id.to_string()),
            claimant_public_key: value.claimant_public_key,
            status: value.status,
            expires_at: value.expires_at,
            created_at: value.created_at,
            accepted_at: value.accepted_at,
        }
    }
}

fn normalize_endpoint(endpoint: &str) -> Result<String, String> {
    let trimmed = endpoint.trim().trim_end_matches('/');
    let parsed = Url::parse(trimmed).map_err(|_| "服务器地址格式无效".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || parsed.username() != ""
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err("服务器地址必须是没有账号、密码、查询参数的 HTTP(S) 地址".into());
    }
    Ok(trimmed.to_owned())
}

fn require_endpoint() -> Result<String, String> {
    read_secret(ENDPOINT_ACCOUNT)?.ok_or_else(|| "请先设置远程同步服务器地址".into())
}

fn require_device_token() -> Result<String, String> {
    read_secret(DEVICE_TOKEN_ACCOUNT)?.ok_or_else(|| "请先在此设备完成服务器注册".into())
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
        .user_agent("QzoneArchive Remote Sync/1")
        .build()
        .map_err(|error| format!("创建远程同步网络客户端失败：{error}"))
}

fn authenticated_client(_token: &str) -> Result<Client, String> {
    http_client()
}

async fn checked_response(response: reqwest::Response) -> Result<reqwest::Response, String> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let message = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|error| error.as_str())
                .map(str::to_owned)
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "服务器返回了未预期的错误".to_string());
    Err(format!(
        "远程同步服务器返回 {}：{}",
        status.as_u16(),
        message
    ))
}

fn load_or_create_keypair() -> Result<(String, String), String> {
    if let (Some(private_key), Some(public_key)) = (
        read_secret(DEVICE_PRIVATE_KEY_ACCOUNT)?,
        read_secret(DEVICE_PUBLIC_KEY_ACCOUNT)?,
    ) {
        return Ok((private_key, public_key));
    }
    let private = StaticSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&private);
    let private_key = BASE64.encode(private.to_bytes());
    let public_key = BASE64.encode(public.as_bytes());
    Ok((private_key, public_key))
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux"
))]
fn credential_entry(account: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(CREDENTIAL_SERVICE, account)
        .map_err(|error| format!("打开系统安全凭据库失败：{error}"))
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux"
))]
fn read_secret(account: &str) -> Result<Option<String>, String> {
    match credential_entry(account)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!("读取系统安全凭据失败：{error}")),
    }
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux"
)))]
fn read_secret(_account: &str) -> Result<Option<String>, String> {
    Err("当前平台不支持系统安全凭据库".into())
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux"
))]
fn write_secret(account: &str, value: &str) -> Result<(), String> {
    credential_entry(account)?
        .set_password(value)
        .map_err(|error| format!("保存系统安全凭据失败：{error}"))
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux"
)))]
fn write_secret(_account: &str, _value: &str) -> Result<(), String> {
    Err("当前平台不支持系统安全凭据库".into())
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux"
))]
fn delete_secret(account: &str) -> Result<(), String> {
    match credential_entry(account)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("删除系统安全凭据失败：{error}")),
    }
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "windows",
    target_os = "linux"
)))]
fn delete_secret(_account: &str) -> Result<(), String> {
    Err("当前平台不支持系统安全凭据库".into())
}

// Keep this helper's generic bound close to the transport code; it also makes
// it harder to accidentally deserialize a server response in the frontend.
#[allow(dead_code)]
async fn parse_json<T: DeserializeOwned>(response: reqwest::Response) -> Result<T, String> {
    checked_response(response)
        .await?
        .json()
        .await
        .map_err(|error| format!("解析服务器响应失败：{error}"))
}

#[allow(dead_code)]
fn _status_is_retryable(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAccountPublicKey {
    pub key_version: i32,
    pub public_key: String,
    pub device_id: String,
    pub label: Option<String>,
    pub registered_at: String,
}

/// 幂等确保当前登录账号已在远程服务器注册。登录后调用一次即可。
#[tauri::command]
pub async fn ensure_remote_device_registered(
    login: tauri::State<'_, QLoginState>,
) -> Result<RemoteSyncConfig, String> {
    let endpoint = match read_secret(ENDPOINT_ACCOUNT)? {
        Some(value) if !value.trim().is_empty() => value,
        _ => return get_remote_sync_config(),
    };
    let uin = login
        .qzone_auth()
        .await
        .map_err(|error| format!("读取当前登录账号失败：{error}"))?
        .uin;
    let registered_uin = read_secret(ACCOUNT_UIN_ACCOUNT)?;
    if registered_uin.as_deref() != Some(uin.as_str()) {
        let label = read_secret(DEVICE_LABEL_ACCOUNT)?;
        register_remote_device(endpoint, uin, label).await?;
    }
    get_remote_sync_config()
}

/// 读取目标账号当前公钥列表，用于端到端加密路由。
#[tauri::command]
pub async fn get_account_public_keys(
    account_uin: String,
) -> Result<Vec<RemoteAccountPublicKey>, String> {
    if !valid_account_uin(&account_uin) {
        return Err("QQ 号无效".into());
    }
    let endpoint = require_endpoint()?;
    let token = require_device_token()?;
    let response = authenticated_client(&token)?
        .get(format!("{endpoint}/v1/accounts/{account_uin}/public-keys"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|error| format!("读取账号公钥失败：{error}"))?;
    let response = checked_response(response).await?;
    let keys: Vec<RemoteAccountPublicKey> = response
        .json()
        .await
        .map_err(|error| format!("解析账号公钥失败：{error}"))?;
    Ok(keys)
}
