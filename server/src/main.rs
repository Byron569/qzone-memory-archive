use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, FromRow, PgPool};
use subtle::ConstantTimeEq;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    registration_token: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "需要有效的设备令牌")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(error = %error, "database request failed");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "服务器数据库操作失败")
    }
}

type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    database: &'static str,
}

#[derive(Debug, Deserialize)]
struct RegisterDeviceRequest {
    device_id: String,
    public_key: String,
    label: Option<String>,
}

#[derive(Debug, Serialize)]
struct RegisterDeviceResponse {
    device_id: Uuid,
    device_token: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
struct DeviceResponse {
    id: Uuid,
    device_id: String,
    public_key: String,
    label: Option<String>,
    created_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct ClaimPairingRequest {
    code: String,
}

#[derive(Debug, Serialize, FromRow)]
struct PairingResponse {
    id: Uuid,
    initiator_device_id: Uuid,
    claimant_device_id: Option<Uuid>,
    status: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    accepted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct CreatePairingResponse {
    pairing_id: Uuid,
    code: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct PairingListResponse {
    pairings: Vec<PairingResponse>,
}

#[derive(Debug, Deserialize)]
struct PushChangesRequest {
    changes: Vec<EncryptedChangeInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptedChangeInput {
    record_id: String,
    operation: String,
    revision: i64,
    key_version: i32,
    ciphertext_b64: String,
    nonce_b64: String,
    aad_b64: Option<String>,
    payload_digest: String,
    deleted_at: Option<i64>,
}

#[derive(Debug, Serialize)]
struct SyncCursorResponse {
    #[serde(rename = "streamId")]
    stream_id: Uuid,
    position: String,
    #[serde(rename = "issuedAt")]
    issued_at: i64,
}

#[derive(Debug, Serialize)]
struct SyncConflictResponse {
    #[serde(rename = "recordId")]
    record_id: String,
    #[serde(rename = "localRevision")]
    local_revision: i64,
    #[serde(rename = "remoteRevision")]
    remote_revision: i64,
    #[serde(rename = "localDigest")]
    local_digest: String,
    #[serde(rename = "remoteDigest")]
    remote_digest: String,
}

#[derive(Debug, Serialize)]
struct PushChangesResponse {
    accepted: usize,
    rejected: usize,
    #[serde(rename = "nextCursor")]
    next_cursor: SyncCursorResponse,
    conflicts: Vec<SyncConflictResponse>,
}

#[derive(Debug, Deserialize)]
struct PullChangesQuery {
    cursor: Option<String>,
    limit: Option<u16>,
}

#[derive(Debug, Serialize)]
struct EncryptedChangeResponse {
    #[serde(rename = "recordId")]
    record_id: String,
    operation: String,
    revision: i64,
    #[serde(rename = "keyVersion")]
    key_version: i32,
    #[serde(rename = "ciphertextB64")]
    ciphertext_b64: String,
    #[serde(rename = "nonceB64")]
    nonce_b64: String,
    #[serde(rename = "aadB64", skip_serializing_if = "Option::is_none")]
    aad_b64: Option<String>,
    #[serde(rename = "payloadDigest")]
    payload_digest: String,
    #[serde(rename = "changedAt")]
    changed_at: i64,
    #[serde(rename = "deletedAt", skip_serializing_if = "Option::is_none")]
    deleted_at: Option<i64>,
}

#[derive(Debug, Serialize)]
struct PullChangesResponse {
    changes: Vec<EncryptedChangeResponse>,
    #[serde(rename = "nextCursor")]
    next_cursor: SyncCursorResponse,
    #[serde(rename = "hasMore")]
    has_more: bool,
}

#[derive(Debug, Deserialize)]
struct AckChangesRequest {
    cursor: String,
}

#[derive(Debug, FromRow)]
struct ExistingSyncItem {
    revision: i64,
    content_digest: String,
}

#[derive(Debug, FromRow)]
struct SyncItemRow {
    event_key: String,
    operation: String,
    revision: i64,
    key_version: i32,
    payload: Vec<u8>,
    payload_nonce: Vec<u8>,
    aad: Option<Vec<u8>>,
    content_digest: String,
    created_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    stream_position: i64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,qzone_sync_server=debug".into()),
        )
        .init();

    let database_url = required_env("DATABASE_URL")?;
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let registration_token = required_env("REGISTRATION_TOKEN")?;
    if registration_token.len() < 24 {
        return Err("REGISTRATION_TOKEN 至少需要 24 个字符".into());
    }

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let state = AppState {
        pool,
        registration_token,
    };
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/devices", post(register_device))
        .route("/v1/devices/me", get(get_current_device))
        .route("/v1/pairings", get(list_pairings).post(create_pairing))
        .route("/v1/pairings/claim", post(claim_pairing))
        .route(
            "/v1/pairings/{pairing_id}/changes",
            get(pull_changes).post(push_changes),
        )
        .route("/v1/pairings/{pairing_id}/ack", post(ack_changes))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!(address = %bind_addr, "qzone sync server started");
    axum::serve(listener, app).await?;
    Ok(())
}

fn required_env(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    std::env::var(name).map_err(|_| format!("缺少环境变量 {name}").into())
}

async fn healthz(State(state): State<AppState>) -> ApiResult<Json<HealthResponse>> {
    sqlx::query("SELECT 1").execute(&state.pool).await?;
    Ok(Json(HealthResponse {
        status: "ok",
        database: "ok",
    }))
}

async fn register_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterDeviceRequest>,
) -> ApiResult<(StatusCode, Json<RegisterDeviceResponse>)> {
    require_registration_token(&headers, &state.registration_token)?;
    validate_device_request(&request)?;

    let device_id = Uuid::new_v4();
    let device_token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let token_hash = hash_secret(&device_token);
    let created_at = Utc::now();

    let registered = sqlx::query_as::<_, (Uuid, DateTime<Utc>)>(
        "INSERT INTO devices (id, device_id, public_key, token_hash, label, created_at, last_seen_at)
         VALUES ($1, $2, $3, $4, $5, $6, $6)
         ON CONFLICT (device_id) DO UPDATE SET
           public_key = EXCLUDED.public_key,
           token_hash = EXCLUDED.token_hash,
           label = EXCLUDED.label,
           last_seen_at = EXCLUDED.last_seen_at,
           revoked_at = NULL
         RETURNING id, created_at",
    )
    .bind(device_id)
    .bind(&request.device_id)
    .bind(&request.public_key)
    .bind(token_hash)
    .bind(request.label.as_deref())
    .bind(created_at)
    .fetch_one(&state.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterDeviceResponse {
            device_id: registered.0,
            device_token,
            created_at: registered.1,
        }),
    ))
}

async fn get_current_device(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<DeviceResponse>> {
    let device = authenticate(&headers, &state.pool).await?;
    Ok(Json(device))
}

async fn create_pairing(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<(StatusCode, Json<CreatePairingResponse>)> {
    let device = authenticate(&headers, &state.pool).await?;
    let pairing_id = Uuid::new_v4();
    let code = Uuid::new_v4().simple().to_string()[..10].to_uppercase();
    let expires_at = Utc::now() + Duration::minutes(15);

    sqlx::query(
        "INSERT INTO pairings (id, initiator_device_id, code_hash, status, expires_at)
         VALUES ($1, $2, $3, 'pending', $4)",
    )
    .bind(pairing_id)
    .bind(device.id)
    .bind(hash_secret(&code))
    .bind(expires_at)
    .execute(&state.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(CreatePairingResponse {
            pairing_id,
            code,
            expires_at,
        }),
    ))
}

async fn claim_pairing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ClaimPairingRequest>,
) -> ApiResult<Json<PairingResponse>> {
    let claimant = authenticate(&headers, &state.pool).await?;
    let code = request.code.trim().to_uppercase();
    if code.len() != 10 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "配对码格式无效"));
    }

    let mut transaction = state.pool.begin().await?;
    let pairing = sqlx::query_as::<_, PairingResponse>(
        "SELECT id, initiator_device_id, claimant_device_id, status, expires_at, created_at, accepted_at
         FROM pairings
         WHERE code_hash = $1
         FOR UPDATE",
    )
    .bind(hash_secret(&code))
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "配对码不存在或已失效"))?;

    if pairing.initiator_device_id == claimant.id {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "不能用同一设备完成双方配对",
        ));
    }
    if pairing.status != "pending" || pairing.expires_at <= Utc::now() {
        return Err(ApiError::new(StatusCode::GONE, "配对码已过期或已使用"));
    }

    let accepted_at = Utc::now();
    let updated = sqlx::query_as::<_, PairingResponse>(
        "UPDATE pairings
         SET claimant_device_id = $1, status = 'accepted', accepted_at = $2
         WHERE id = $3
         RETURNING id, initiator_device_id, claimant_device_id, status, expires_at, created_at, accepted_at",
    )
    .bind(claimant.id)
    .bind(accepted_at)
    .bind(pairing.id)
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO pairing_streams (pairing_id, next_position)
         VALUES ($1, 1)
         ON CONFLICT (pairing_id) DO NOTHING",
    )
    .bind(pairing.id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(Json(updated))
}

async fn list_pairings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<PairingListResponse>> {
    let device = authenticate(&headers, &state.pool).await?;
    let pairings = sqlx::query_as::<_, PairingResponse>(
        "SELECT id, initiator_device_id, claimant_device_id, status, expires_at, created_at, accepted_at
         FROM pairings
         WHERE initiator_device_id = $1 OR claimant_device_id = $1
         ORDER BY created_at DESC",
    )
    .bind(device.id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(PairingListResponse { pairings }))
}

async fn push_changes(
    State(state): State<AppState>,
    Path(pairing_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<PushChangesRequest>,
) -> ApiResult<Json<PushChangesResponse>> {
    let device = authenticate_pairing(&headers, &state, pairing_id).await?;
    if request.changes.is_empty() || request.changes.len() > 100 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "每次上传需要包含 1 到 100 条变更",
        ));
    }

    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO pairing_streams (pairing_id, next_position)
         VALUES ($1, 1)
         ON CONFLICT (pairing_id) DO NOTHING",
    )
    .bind(pairing_id)
    .execute(&mut *transaction)
    .await?;

    let mut accepted = 0;
    let mut rejected = 0;
    let mut conflicts = Vec::new();

    for change in request.changes {
        let decoded = decode_change(&change)?;
        let existing = sqlx::query_as::<_, ExistingSyncItem>(
            "SELECT revision, content_digest
             FROM sync_items
             WHERE pairing_id = $1 AND event_key = $2
             FOR UPDATE",
        )
        .bind(pairing_id)
        .bind(&change.record_id)
        .fetch_optional(&mut *transaction)
        .await?;

        if let Some(existing) = existing {
            if change.revision < existing.revision {
                rejected += 1;
                conflicts.push(SyncConflictResponse {
                    record_id: change.record_id,
                    local_revision: existing.revision,
                    remote_revision: change.revision,
                    local_digest: existing.content_digest,
                    remote_digest: change.payload_digest,
                });
                continue;
            }
            if change.revision == existing.revision {
                if change.payload_digest == existing.content_digest {
                    accepted += 1;
                    continue;
                }
                rejected += 1;
                conflicts.push(SyncConflictResponse {
                    record_id: change.record_id,
                    local_revision: existing.revision,
                    remote_revision: change.revision,
                    local_digest: existing.content_digest,
                    remote_digest: change.payload_digest,
                });
                continue;
            }
        }

        let stream_position = sqlx::query_scalar::<_, i64>(
            "UPDATE pairing_streams
             SET next_position = next_position + 1, updated_at = now()
             WHERE pairing_id = $1
             RETURNING next_position - 1",
        )
        .bind(pairing_id)
        .fetch_one(&mut *transaction)
        .await?;

        sqlx::query(
            "INSERT INTO sync_items (
                 id, pairing_id, event_key, source_device_id, payload, payload_nonce,
                 content_digest, revision, operation, key_version, aad, deleted_at,
                 stream_position
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (pairing_id, event_key) DO UPDATE SET
                 source_device_id = EXCLUDED.source_device_id,
                 payload = EXCLUDED.payload,
                 payload_nonce = EXCLUDED.payload_nonce,
                 content_digest = EXCLUDED.content_digest,
                 revision = EXCLUDED.revision,
                 operation = EXCLUDED.operation,
                 key_version = EXCLUDED.key_version,
                 aad = EXCLUDED.aad,
                 deleted_at = EXCLUDED.deleted_at,
                 created_at = now(),
                 stream_position = EXCLUDED.stream_position",
        )
        .bind(Uuid::new_v4())
        .bind(pairing_id)
        .bind(&change.record_id)
        .bind(device.id)
        .bind(decoded.ciphertext)
        .bind(decoded.nonce)
        .bind(&change.payload_digest)
        .bind(change.revision)
        .bind(&change.operation)
        .bind(change.key_version)
        .bind(decoded.aad)
        .bind(decoded.deleted_at)
        .bind(stream_position)
        .execute(&mut *transaction)
        .await?;

        if change.operation == "tombstone" {
            sqlx::query(
                "INSERT INTO tombstones (id, pairing_id, event_key, source_device_id)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (pairing_id, event_key) DO UPDATE SET
                   source_device_id = EXCLUDED.source_device_id,
                   created_at = now()",
            )
            .bind(Uuid::new_v4())
            .bind(pairing_id)
            .bind(&change.record_id)
            .bind(device.id)
            .execute(&mut *transaction)
            .await?;
        } else {
            sqlx::query("DELETE FROM tombstones WHERE pairing_id = $1 AND event_key = $2")
                .bind(pairing_id)
                .bind(&change.record_id)
                .execute(&mut *transaction)
                .await?;
        }
        accepted += 1;
    }

    let current_position = sqlx::query_scalar::<_, i64>(
        "SELECT next_position - 1 FROM pairing_streams WHERE pairing_id = $1",
    )
    .bind(pairing_id)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(Json(PushChangesResponse {
        accepted,
        rejected,
        next_cursor: sync_cursor(pairing_id, current_position),
        conflicts,
    }))
}

async fn pull_changes(
    State(state): State<AppState>,
    Path(pairing_id): Path<Uuid>,
    Query(query): Query<PullChangesQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<PullChangesResponse>> {
    let device = authenticate_pairing(&headers, &state, pairing_id).await?;
    let cursor = parse_cursor(query.cursor.as_deref())?;
    let limit = query.limit.unwrap_or(100).clamp(1, 500) as i64;
    let rows = sqlx::query_as::<_, SyncItemRow>(
        "SELECT event_key, operation, revision, key_version, payload, payload_nonce,
                aad, content_digest, created_at, deleted_at, stream_position
         FROM sync_items
         WHERE pairing_id = $1 AND source_device_id <> $2 AND stream_position > $3
         ORDER BY stream_position ASC
         LIMIT $4",
    )
    .bind(pairing_id)
    .bind(device.id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.pool)
    .await?;

    let has_more = rows.len() > limit as usize;
    let wrapped_changes = rows
        .into_iter()
        .take(limit as usize)
        .map(encrypted_change_response)
        .collect::<Vec<_>>();
    let next_position = wrapped_changes
        .last()
        .map(|change| change.stream_position)
        .unwrap_or(cursor);
    let changes = wrapped_changes
        .into_iter()
        .map(|change| change.change)
        .collect::<Vec<_>>();

    Ok(Json(PullChangesResponse {
        changes,
        next_cursor: sync_cursor(pairing_id, next_position),
        has_more,
    }))
}

async fn ack_changes(
    State(state): State<AppState>,
    Path(pairing_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<AckChangesRequest>,
) -> ApiResult<StatusCode> {
    let device = authenticate_pairing(&headers, &state, pairing_id).await?;
    let position = parse_cursor(Some(&request.cursor))?;
    let max_position = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(stream_position), 0) FROM sync_items WHERE pairing_id = $1",
    )
    .bind(pairing_id)
    .fetch_one(&state.pool)
    .await?;
    if position > max_position {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "同步游标超出当前流位置",
        ));
    }

    sqlx::query(
        "INSERT INTO sync_cursors (pairing_id, device_id, last_revision, updated_at)
         VALUES ($1, $2, $3, now())
         ON CONFLICT (pairing_id, device_id) DO UPDATE SET
           last_revision = GREATEST(sync_cursors.last_revision, EXCLUDED.last_revision),
           updated_at = now()",
    )
    .bind(pairing_id)
    .bind(device.id)
    .bind(position)
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug)]
struct DecodedChange {
    ciphertext: Vec<u8>,
    nonce: Vec<u8>,
    aad: Option<Vec<u8>>,
    deleted_at: Option<DateTime<Utc>>,
}

fn decode_change(change: &EncryptedChangeInput) -> ApiResult<DecodedChange> {
    if change.record_id.trim().is_empty() || change.record_id.len() > 512 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "记录标识无效"));
    }
    if change.operation != "upsert" && change.operation != "tombstone" {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "同步操作类型无效"));
    }
    if change.revision < 0 || change.revision > i64::from(i32::MAX) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "记录版本无效"));
    }
    if !(1..=100).contains(&change.key_version) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "加密密钥版本无效"));
    }
    if change.payload_digest.len() != 64
        || !change
            .payload_digest
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "密文摘要格式无效"));
    }
    if change.ciphertext_b64.len() > 8 * 1024 * 1024 {
        return Err(ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "单条密文过大"));
    }
    let ciphertext = BASE64
        .decode(&change.ciphertext_b64)
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "密文不是有效 Base64"))?;
    let nonce = BASE64
        .decode(&change.nonce_b64)
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "随机数不是有效 Base64"))?;
    if nonce.len() < 8 || nonce.len() > 64 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "随机数长度无效"));
    }
    let aad = change
        .aad_b64
        .as_deref()
        .map(|value| {
            BASE64
                .decode(value)
                .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "AAD 不是有效 Base64"))
        })
        .transpose()?;
    if aad.as_ref().is_some_and(|value| value.len() > 4096) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "AAD 过大"));
    }
    let deleted_at = match change.deleted_at {
        Some(value) => Some(
            DateTime::<Utc>::from_timestamp_millis(value)
                .ok_or_else(|| ApiError::new(StatusCode::BAD_REQUEST, "删除时间无效"))?,
        ),
        None => None,
    };

    Ok(DecodedChange {
        ciphertext,
        nonce,
        aad,
        deleted_at,
    })
}

fn encrypted_change_response(row: SyncItemRow) -> EncryptedChangeResponseWithPosition {
    EncryptedChangeResponseWithPosition {
        change: EncryptedChangeResponse {
            record_id: row.event_key,
            operation: row.operation,
            revision: row.revision,
            key_version: row.key_version,
            ciphertext_b64: BASE64.encode(row.payload),
            nonce_b64: BASE64.encode(row.payload_nonce),
            aad_b64: row.aad.map(|value| BASE64.encode(value)),
            payload_digest: row.content_digest,
            changed_at: row.created_at.timestamp_millis(),
            deleted_at: row.deleted_at.map(|value| value.timestamp_millis()),
        },
        stream_position: row.stream_position,
    }
}

#[derive(Debug)]
struct EncryptedChangeResponseWithPosition {
    change: EncryptedChangeResponse,
    stream_position: i64,
}

async fn authenticate_pairing(
    headers: &HeaderMap,
    state: &AppState,
    pairing_id: Uuid,
) -> ApiResult<DeviceResponse> {
    let device = authenticate(headers, &state.pool).await?;
    let is_member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
             SELECT 1 FROM pairings
             WHERE id = $1 AND status = 'accepted'
               AND (initiator_device_id = $2 OR claimant_device_id = $2)
         )",
    )
    .bind(pairing_id)
    .bind(device.id)
    .fetch_one(&state.pool)
    .await?;
    if !is_member {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "设备无权访问此配对"));
    }
    Ok(device)
}

fn parse_cursor(value: Option<&str>) -> ApiResult<i64> {
    let raw = value.unwrap_or("0").trim();
    let position = raw
        .parse::<i64>()
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "同步游标格式无效"))?;
    if position < 0 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "同步游标不能为负数"));
    }
    Ok(position)
}

fn sync_cursor(pairing_id: Uuid, position: i64) -> SyncCursorResponse {
    SyncCursorResponse {
        stream_id: pairing_id,
        position: position.to_string(),
        issued_at: Utc::now().timestamp_millis(),
    }
}

async fn authenticate(headers: &HeaderMap, pool: &PgPool) -> ApiResult<DeviceResponse> {
    let token = bearer_token(headers).ok_or_else(ApiError::unauthorized)?;
    let token_hash = hash_secret(&token);
    let device = sqlx::query_as::<_, DeviceResponse>(
        "UPDATE devices
         SET last_seen_at = now()
         WHERE token_hash = $1 AND revoked_at IS NULL
         RETURNING id, device_id, public_key, label, created_at, last_seen_at",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?
    .ok_or_else(ApiError::unauthorized)?;
    Ok(device)
}

fn require_registration_token(headers: &HeaderMap, expected: &str) -> ApiResult<()> {
    let provided = headers
        .get("x-registration-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if provided.as_bytes().ct_eq(expected.as_bytes()).unwrap_u8() != 1 {
        return Err(ApiError::unauthorized());
    }
    Ok(())
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get("authorization")?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn validate_device_request(request: &RegisterDeviceRequest) -> ApiResult<()> {
    if request.device_id.trim().is_empty() || request.device_id.len() > 128 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "设备标识无效"));
    }
    if request.public_key.trim().is_empty() || request.public_key.len() > 4096 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "设备公钥无效"));
    }
    if request
        .label
        .as_ref()
        .is_some_and(|label| label.len() > 128)
    {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "设备名称过长"));
    }
    Ok(())
}

fn hash_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[allow(dead_code)]
fn _warn_if_insecure_registration_token(token: &str) {
    if token.len() < 32 {
        warn!("registration token is shorter than the recommended 32 characters");
    }
}
