use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
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
