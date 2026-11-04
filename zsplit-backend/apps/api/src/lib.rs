//! Authenticated HTTP surface. No handler constructs, signs, or proves Zcash transactions.
use axum::{
    Json, Router,
    extract::{ConnectInfo, DefaultBodyLimit, Path, Request, State, rejection::JsonRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use futures_util::Stream;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    convert::Infallible,
    net::IpAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tower_http::cors::CorsLayer;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};
use zsplit_claims::{ReceiverCipher, TokenService, validate_receiver};
use zsplit_domain::{DEFAULT_FEE_RESERVE, DomainError, MAX_TICKETS, Network, PayoutEngine};
use zsplit_store::{
    ClaimRecord, NewClaim, NewRoom, RedeemCommand, Redemption, Room, RoomSummary, Store,
};

#[derive(Clone)]
pub struct ApiConfig {
    pub network: Network,
    pub confirmations: u32,
    pub organizer_id: Uuid,
    pub allowed_origin: String,
    pub public_claim_origin: String,
    pub docs_enabled: bool,
}
pub struct AppState {
    pub store: Store,
    pub engine: Arc<dyn PayoutEngine>,
    pub tokens: TokenService,
    pub cipher: Arc<ReceiverCipher>,
    pub config: ApiConfig,
    pub worker_alive: Arc<AtomicBool>,
    organizer_digest: [u8; 32],
    fingerprint_key: Zeroizing<[u8; 32]>,
    limiter: Mutex<Limiter>,
    sse_slots: Arc<tokio::sync::Semaphore>,
}
impl AppState {
    pub fn new(
        store: Store,
        engine: Arc<dyn PayoutEngine>,
        config: ApiConfig,
        organizer_secret: &str,
        token_key: [u8; 32],
        cipher_key: [u8; 32],
    ) -> Result<Arc<Self>, DomainError> {
        if organizer_secret.len() < 32
            || config.confirmations == 0
            || config.network == Network::Mainnet
            || token_key == cipher_key
            || token_key == [0; 32]
            || cipher_key == [0; 32]
        {
            return Err(DomainError::InvalidInput);
        }
        validate_origin(&config.allowed_origin)?;
        validate_origin(&config.public_claim_origin)?;
        let mut derivation = Hmac::<Sha256>::new_from_slice(&cipher_key).expect("HMAC key length");
        derivation.update(b"zsplit-request-fingerprint-key-v1");
        let fingerprint_key: [u8; 32] = derivation.finalize().into_bytes().into();
        Ok(Arc::new(Self {
            store,
            engine,
            tokens: TokenService::new(token_key),
            cipher: Arc::new(ReceiverCipher::new(cipher_key)),
            config,
            worker_alive: Arc::new(AtomicBool::new(true)),
            organizer_digest: Sha256::digest(organizer_secret.as_bytes()).into(),
            fingerprint_key: Zeroizing::new(fingerprint_key),
            limiter: Mutex::new(Limiter::default()),
            sse_slots: Arc::new(tokio::sync::Semaphore::new(32)),
        }))
    }
    fn authorize(&self, headers: &HeaderMap) -> Result<Uuid, ApiError> {
        let token = bearer(headers).ok_or_else(|| {
            ApiError::new(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Organizer authentication required",
            )
        })?;
        let digest: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        if !bool::from(digest.ct_eq(&self.organizer_digest)) {
            return Err(ApiError::new(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Organizer authentication required",
            ));
        }
        Ok(self.config.organizer_id)
    }
    fn claim_digest(&self, headers: &HeaderMap) -> Result<[u8; 32], ApiError> {
        self.tokens
            .digest(bearer(headers).ok_or(DomainError::NotFound)?)
            .map_err(|_| DomainError::NotFound.into())
    }
    fn fingerprint(&self, id: Uuid, receiver: &str, amount: u64, network: Network) -> Vec<u8> {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.fingerprint_key.as_ref())
            .expect("HMAC accepts a 32-byte key");
        mac.update(b"zsplit-redemption-request-v1\0");
        mac.update(id.as_bytes());
        mac.update(&amount.to_be_bytes());
        mac.update(network.to_string().as_bytes());
        mac.update(receiver.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }
}
fn validate_origin(value: &str) -> Result<(), DomainError> {
    let uri: axum::http::Uri = value.parse().map_err(|_| DomainError::InvalidInput)?;
    let authority = uri.authority().ok_or(DomainError::InvalidInput)?;
    if value.ends_with('/')
        || value.contains(['#', '?', '@'])
        || uri.path_and_query().is_some_and(|p| p.as_str() != "/")
        || !(uri.scheme_str() == Some("https")
            || (uri.scheme_str() == Some("http")
                && matches!(authority.host(), "localhost" | "127.0.0.1")))
    {
        return Err(DomainError::InvalidInput);
    }
    Ok(())
}
fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|s| !s.is_empty() && s.len() <= 256)
}
#[derive(Default)]
struct Limiter {
    global_window: Option<Instant>,
    global_count: u32,
    buckets: HashMap<IpAddr, (Instant, u32)>,
}
impl Limiter {
    fn allow(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        if self
            .global_window
            .is_none_or(|w| now.duration_since(w) >= Duration::from_secs(1))
        {
            self.global_window = Some(now);
            self.global_count = 0;
        }
        self.global_count += 1;
        if self.global_count > 500 {
            return false;
        }
        if self.buckets.len() >= 4096 {
            self.buckets
                .retain(|_, (at, _)| now.duration_since(*at) < Duration::from_secs(60));
            if !self.buckets.contains_key(&ip) && self.buckets.len() >= 4096 {
                return false;
            }
        }
        let (at, count) = self.buckets.entry(ip).or_insert((now, 0));
        if now.duration_since(*at) >= Duration::from_secs(60) {
            *at = now;
            *count = 0
        }
        *count += 1;
        *count <= 240
    }
}
#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    pub request_id: Uuid,
}
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
}
impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: &'static str) -> Self {
        Self {
            status,
            code,
            message,
        }
    }
}
impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::InvalidInput => {
                Self::new(StatusCode::BAD_REQUEST, "invalid_input", "Invalid request")
            }
            DomainError::NotFound => {
                Self::new(StatusCode::NOT_FOUND, "not_found", "Resource unavailable")
            }
            DomainError::Unauthorized => Self::new(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Authentication required",
            ),
            DomainError::Conflict => Self::new(
                StatusCode::CONFLICT,
                "conflict",
                "Operation conflicts with current state",
            ),
            DomainError::InsufficientFunds => Self::new(
                StatusCode::CONFLICT,
                "insufficient_funds",
                "Confirmed funds do not cover this operation",
            ),
            DomainError::Expired => Self::new(StatusCode::CONFLICT, "expired", "Claim unavailable"),
            DomainError::NetworkMismatch => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "network_mismatch",
                "Network incompatible",
            ),
            DomainError::Unavailable
            | DomainError::Storage
            | DomainError::ReconciliationRequired => Self::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "Service temporarily unavailable",
            ),
        }
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                code: self.code.into(),
                message: self.message.into(),
                request_id: Uuid::nil(),
            }),
        )
            .into_response()
    }
}
fn json<T>(value: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    value
        .map(|Json(v)| v)
        .map_err(|_| DomainError::InvalidInput.into())
}
fn id(value: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value).map_err(|_| DomainError::NotFound.into())
}

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
}
#[derive(Serialize, ToSchema)]
pub struct MetaResponse {
    pub network: String,
    pub confirmations: u32,
    pub max_tickets: usize,
    pub min_payout_zat: u64,
    pub fee_budget_per_ticket_zat: u64,
    pub sweep_reports_verified: bool,
    pub recipient_wallet_requirement: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateRoomRequest {
    pub network: String,
    pub amounts_zat: Vec<u64>,
    pub expires_at: DateTime<Utc>,
}
#[derive(Serialize, ToSchema)]
pub struct RoomResponse {
    pub id: Uuid,
    pub network: String,
    pub status: String,
    pub amounts_zat: Vec<u64>,
    pub fee_budget_per_ticket_zat: u64,
    pub confirmed_zat: u64,
    pub reserved_zat: u64,
    pub claims_issued: bool,
    pub expires_at: DateTime<Utc>,
}
impl From<Room> for RoomResponse {
    fn from(r: Room) -> Self {
        Self {
            id: r.id,
            network: r.network,
            status: r.state,
            amounts_zat: r.payout_amounts.0,
            fee_budget_per_ticket_zat: r.fee_reserve_zat as u64,
            confirmed_zat: r.confirmed_zat as u64,
            reserved_zat: r.reserved_zat as u64,
            claims_issued: r.claims_issued,
            expires_at: r.expires_at,
        }
    }
}
#[derive(Serialize, ToSchema)]
pub struct DepositResponse {
    pub room_id: Uuid,
    pub network: String,
    pub address: String,
    pub birthday_height: u32,
    pub confirmed_zat: u64,
    pub spendable_zat: u64,
    pub scanned_height: u32,
    pub chain_height: u32,
}
#[derive(Serialize, ToSchema)]
pub struct TicketResponse {
    pub claim_id: Uuid,
    pub amount_zat: u64,
    pub token: String,
    pub claim_url: String,
    pub expires_at: DateTime<Utc>,
}
impl Drop for TicketResponse {
    fn drop(&mut self) {
        self.token.zeroize();
        self.claim_url.zeroize()
    }
}
#[derive(Serialize, ToSchema)]
pub struct IssueResponse {
    pub tickets: Vec<TicketResponse>,
}
#[derive(Serialize, ToSchema)]
pub struct SummaryResponse {
    pub room_id: Uuid,
    pub confirmed_zat: u64,
    pub reserved_zat: u64,
    pub spendable_zat: u64,
    pub reserved_count: u64,
    pub pending_count: u64,
    pub funded_confirmed_count: u64,
    pub expired_count: u64,
    pub sweep_reported_count: u64,
    pub manual_review_count: u64,
    pub sweep_reports_verified: bool,
}
impl From<RoomSummary> for SummaryResponse {
    fn from(s: RoomSummary) -> Self {
        Self {
            room_id: s.room_id,
            confirmed_zat: s.confirmed_zat as u64,
            reserved_zat: s.reserved_zat as u64,
            spendable_zat: s.spendable_zat as u64,
            reserved_count: s.reserved_count as u64,
            pending_count: s.pending_count as u64,
            funded_confirmed_count: s.funded_confirmed_count as u64,
            expired_count: s.expired_count as u64,
            sweep_reported_count: s.sweep_reported_count as u64,
            manual_review_count: s.manual_review_count as u64,
            sweep_reports_verified: false,
        }
    }
}
#[derive(Serialize, ToSchema)]
pub struct ClaimResponse {
    pub claim_id: Uuid,
    pub amount_zat: u64,
    pub network: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub txid: Option<String>,
    pub confirmations: u32,
    pub sweep_reported: bool,
    pub sweep_verified: bool,
}
impl From<ClaimRecord> for ClaimResponse {
    fn from(c: ClaimRecord) -> Self {
        Self {
            claim_id: c.id,
            amount_zat: c.amount_zat as u64,
            network: c.network,
            status: c.state,
            expires_at: c.expires_at,
            txid: c.txid,
            confirmations: c.confirmations as u32,
            sweep_reported: c.sweep_reported,
            sweep_verified: false,
        }
    }
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RedeemRequest {
    pub ephemeral_receiver: String,
    pub idempotency_key: Uuid,
    pub client_network: String,
    pub wallet_persisted: bool,
}
impl Drop for RedeemRequest {
    fn drop(&mut self) {
        self.ephemeral_receiver.zeroize()
    }
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WithdrawRequest {
    pub organizer_receiver: String,
    pub amount_zat: u64,
    pub idempotency_key: Uuid,
    pub network: String,
}
impl Drop for WithdrawRequest {
    fn drop(&mut self) {
        self.organizer_receiver.zeroize()
    }
}
#[derive(Serialize, ToSchema)]
pub struct OperationResponse {
    pub claim_id: Uuid,
    pub payout_id: Uuid,
    pub status: String,
    pub poll_after_ms: u32,
}
impl From<Redemption> for OperationResponse {
    fn from(r: Redemption) -> Self {
        Self {
            claim_id: r.claim_id,
            payout_id: r.payout_id,
            status: r.status,
            poll_after_ms: 2000,
        }
    }
}
#[derive(Serialize, ToSchema)]
pub struct SweepReportResponse {
    pub reported: bool,
    pub verified: bool,
}

#[utoipa::path(get,path="/healthz",responses((status=200,body=HealthResponse)))]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
    })
}
#[utoipa::path(get,path="/readyz",responses((status=200,body=HealthResponse),(status=503,body=ErrorBody)))]
async fn ready(State(s): State<Arc<AppState>>) -> Result<Json<HealthResponse>, ApiError> {
    if !s.worker_alive.load(Ordering::Acquire) {
        return Err(DomainError::Unavailable.into());
    }
    s.store.ready().await?;
    if s.store.rooms().await?.iter().any(|r| {
        r.balance_updated_at
            .is_none_or(|at| Utc::now().signed_duration_since(at).num_seconds() > 120)
            || r.chain_height - r.scanned_height > 2
    }) {
        return Err(DomainError::Unavailable.into());
    }
    Ok(Json(HealthResponse {
        status: "ready".into(),
    }))
}
#[utoipa::path(get,path="/api/v1/meta",responses((status=200,body=MetaResponse)))]
async fn meta(State(s): State<Arc<AppState>>) -> Json<MetaResponse> {
    Json(MetaResponse{network:s.config.network.to_string(),confirmations:s.config.confirmations,max_tickets:MAX_TICKETS,min_payout_zat:10_000,fee_budget_per_ticket_zat:DEFAULT_FEE_RESERVE,sweep_reports_verified:false,recipient_wallet_requirement:"Client must persist an ephemeral Orchard wallet and acknowledge recovery before redemption. Spending keys and personal sweep destinations must stay client-side.".into()})
}
#[utoipa::path(post,path="/api/v1/rooms",request_body=CreateRoomRequest,security(("bearer_auth"=[])),responses((status=201,body=RoomResponse),(status=400,body=ErrorBody),(status=401,body=ErrorBody),(status=503,body=ErrorBody)))]
async fn create_room(
    State(s): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<CreateRoomRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<RoomResponse>), ApiError> {
    let owner = s.authorize(&headers)?;
    let req = json(body)?;
    let network: Network = req.network.parse()?;
    if network != s.config.network {
        return Err(DomainError::NetworkMismatch.into());
    }
    zsplit_store::validate_amounts(&req.amounts_zat, DEFAULT_FEE_RESERVE)?;
    if req.expires_at <= Utc::now() || req.expires_at > Utc::now() + chrono::Duration::days(30) {
        return Err(DomainError::InvalidInput.into());
    }
    let room_id = Uuid::new_v4();
    let deposit = s.engine.deposit_address(room_id).await?;
    if deposit.network != network {
        return Err(DomainError::NetworkMismatch.into());
    }
    let room = s
        .store
        .create_room(NewRoom {
            id: room_id,
            organizer_id: owner,
            network,
            deposit_address: deposit.address,
            birthday_height: deposit.birthday_height,
            amounts: req.amounts_zat,
            fee_reserve_zat: DEFAULT_FEE_RESERVE,
            expires_at: req.expires_at,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(room.into())))
}
#[utoipa::path(get,path="/api/v1/rooms/{room_id}",params(("room_id"=Uuid,Path)),security(("bearer_auth"=[])),responses((status=200,body=RoomResponse),(status=401,body=ErrorBody),(status=404,body=ErrorBody)))]
async fn get_room(
    State(s): State<Arc<AppState>>,
    Path(room): Path<String>,
    headers: HeaderMap,
) -> Result<Json<RoomResponse>, ApiError> {
    let owner = s.authorize(&headers)?;
    Ok(Json(s.store.room(id(&room)?, owner).await?.into()))
}
#[utoipa::path(get,path="/api/v1/rooms/{room_id}/deposit",params(("room_id"=Uuid,Path)),security(("bearer_auth"=[])),responses((status=200,body=DepositResponse),(status=401,body=ErrorBody),(status=404,body=ErrorBody)))]
async fn deposit(
    State(s): State<Arc<AppState>>,
    Path(room): Path<String>,
    headers: HeaderMap,
) -> Result<Json<DepositResponse>, ApiError> {
    let owner = s.authorize(&headers)?;
    let r = s.store.room(id(&room)?, owner).await?;
    Ok(Json(DepositResponse {
        room_id: r.id,
        network: r.network,
        address: r.deposit_address,
        birthday_height: r.birthday_height as u32,
        confirmed_zat: r.confirmed_zat as u64,
        spendable_zat: r.spendable_zat as u64,
        scanned_height: r.scanned_height as u32,
        chain_height: r.chain_height as u32,
    }))
}
#[utoipa::path(post,path="/api/v1/rooms/{room_id}/claims",params(("room_id"=Uuid,Path)),security(("bearer_auth"=[])),responses((status=201,body=IssueResponse),(status=401,body=ErrorBody),(status=404,body=ErrorBody),(status=409,body=ErrorBody),(status=503,body=ErrorBody)))]
async fn issue(
    State(s): State<Arc<AppState>>,
    Path(room): Path<String>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<IssueResponse>), ApiError> {
    let owner = s.authorize(&headers)?;
    let r = s.store.room(id(&room)?, owner).await?;
    let mut persisted = Vec::with_capacity(r.payout_amounts.len());
    let mut tickets = Vec::with_capacity(r.payout_amounts.len());
    for amount in r.payout_amounts.iter() {
        let issued = s.tokens.issue();
        let claim_id = Uuid::new_v4();
        persisted.push(NewClaim {
            id: claim_id,
            amount_zat: *amount,
            token_digest: issued.digest,
        });
        let token = issued.token.to_string();
        let claim_url = format!(
            "{}/claim/{claim_id}#t={token}",
            s.config.public_claim_origin
        );
        tickets.push(TicketResponse {
            claim_id,
            amount_zat: *amount,
            token,
            claim_url,
            expires_at: r.expires_at,
        });
    }
    s.store.issue_claims(r.id, owner, &persisted).await?;
    Ok((StatusCode::CREATED, Json(IssueResponse { tickets })))
}
#[utoipa::path(get,path="/api/v1/rooms/{room_id}/summary",params(("room_id"=Uuid,Path)),security(("bearer_auth"=[])),responses((status=200,body=SummaryResponse),(status=401,body=ErrorBody),(status=404,body=ErrorBody)))]
async fn summary(
    State(s): State<Arc<AppState>>,
    Path(room): Path<String>,
    headers: HeaderMap,
) -> Result<Json<SummaryResponse>, ApiError> {
    let owner = s.authorize(&headers)?;
    Ok(Json(s.store.summary(id(&room)?, owner).await?.into()))
}
#[utoipa::path(get,path="/api/v1/rooms/{room_id}/events",params(("room_id"=Uuid,Path)),security(("bearer_auth"=[])),responses((status=200,description="Owner-authenticated SSE summary stream",content_type="text/event-stream",body=SummaryResponse),(status=401,body=ErrorBody),(status=404,body=ErrorBody)))]
async fn events(
    State(s): State<Arc<AppState>>,
    Path(room): Path<String>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let owner = s.authorize(&headers)?;
    let room = id(&room)?;
    s.store.room(room, owner).await?;
    let permit = s.sse_slots.clone().try_acquire_owned().map_err(|_| {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "stream_limit",
            "Too many active event streams",
        )
    })?;
    let stream = futures_util::stream::unfold(
        (
            permit,
            s,
            room,
            owner,
            tokio::time::interval(Duration::from_secs(2)),
            0_u64,
        ),
        |(permit, s, room, owner, mut interval, seq)| async move {
            interval.tick().await;
            if !s.worker_alive.load(Ordering::Acquire) {
                return None;
            }
            let summary = s.store.summary(room, owner).await.ok()?;
            let event = Event::default()
                .event("room_summary")
                .id(seq.to_string())
                .json_data(SummaryResponse::from(summary))
                .ok()?;
            Some((Ok(event), (permit, s, room, owner, interval, seq + 1)))
        },
    );
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}
#[utoipa::path(post,path="/api/v1/rooms/{room_id}/withdrawals",params(("room_id"=Uuid,Path)),request_body=WithdrawRequest,security(("bearer_auth"=[])),responses((status=202,body=OperationResponse),(status=400,body=ErrorBody),(status=401,body=ErrorBody),(status=409,body=ErrorBody),(status=503,body=ErrorBody)))]
async fn withdraw(
    State(s): State<Arc<AppState>>,
    Path(room): Path<String>,
    headers: HeaderMap,
    body: Result<Json<WithdrawRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<OperationResponse>), ApiError> {
    let owner = s.authorize(&headers)?;
    let room = id(&room)?;
    s.store.room(room, owner).await?;
    let req = json(body)?;
    let network: Network = req.network.parse()?;
    if network != s.config.network {
        return Err(DomainError::NetworkMismatch.into());
    }
    validate_receiver(&req.organizer_receiver, network)?;
    let encrypted = s
        .cipher
        .encrypt(req.idempotency_key, &req.organizer_receiver)?;
    let fingerprint = s.fingerprint(
        req.idempotency_key,
        &req.organizer_receiver,
        req.amount_zat,
        network,
    );
    let op = s
        .store
        .enqueue_withdrawal(
            room,
            owner,
            req.amount_zat,
            req.idempotency_key,
            encrypted,
            fingerprint,
        )
        .await?;
    Ok((StatusCode::ACCEPTED, Json(op.into())))
}
#[utoipa::path(get,path="/api/v1/claims/{claim_id}",params(("claim_id"=Uuid,Path)),security(("bearer_auth"=[])),responses((status=200,body=ClaimResponse),(status=404,body=ErrorBody)))]
async fn claim(
    State(s): State<Arc<AppState>>,
    Path(claim): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ClaimResponse>, ApiError> {
    let digest = s.claim_digest(&headers)?;
    Ok(Json(s.store.claim(id(&claim)?, &digest).await?.into()))
}
#[utoipa::path(get,path="/api/v1/claims/{claim_id}/status",params(("claim_id"=Uuid,Path)),security(("bearer_auth"=[])),responses((status=200,body=ClaimResponse),(status=404,body=ErrorBody)))]
async fn claim_status(
    state: State<Arc<AppState>>,
    path: Path<String>,
    headers: HeaderMap,
) -> Result<Json<ClaimResponse>, ApiError> {
    claim(state, path, headers).await
}
#[utoipa::path(post,path="/api/v1/claims/{claim_id}/redeem",params(("claim_id"=Uuid,Path)),request_body=RedeemRequest,security(("bearer_auth"=[])),responses((status=202,body=OperationResponse),(status=400,body=ErrorBody),(status=404,body=ErrorBody),(status=409,body=ErrorBody),(status=422,body=ErrorBody),(status=429,body=ErrorBody),(status=503,body=ErrorBody)))]
async fn redeem(
    State(s): State<Arc<AppState>>,
    Path(claim): Path<String>,
    headers: HeaderMap,
    body: Result<Json<RedeemRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<OperationResponse>), ApiError> {
    let digest = s.claim_digest(&headers)?;
    let claim = id(&claim)?;
    let record = s.store.claim(claim, &digest).await?;
    let req = json(body)?;
    if !req.wallet_persisted {
        return Err(DomainError::InvalidInput.into());
    }
    let network: Network = req.client_network.parse()?;
    if network != s.config.network {
        return Err(DomainError::NetworkMismatch.into());
    }
    validate_receiver(&req.ephemeral_receiver, network)?;
    let encrypted = s.cipher.encrypt(claim, &req.ephemeral_receiver)?;
    let fingerprint = s.fingerprint(
        claim,
        &req.ephemeral_receiver,
        record.amount_zat as u64,
        network,
    );
    let result = s
        .store
        .enqueue_redemption(RedeemCommand {
            claim_id: claim,
            token_digest: digest,
            network,
            idempotency_key: req.idempotency_key,
            encrypted_receiver: encrypted,
            request_fingerprint: fingerprint,
        })
        .await?;
    Ok((StatusCode::ACCEPTED, Json(result.into())))
}
#[utoipa::path(post,path="/api/v1/claims/{claim_id}/sweep-report",params(("claim_id"=Uuid,Path)),security(("bearer_auth"=[])),responses((status=200,body=SweepReportResponse),(status=404,body=ErrorBody),(status=409,body=ErrorBody)))]
async fn sweep_report(
    State(s): State<Arc<AppState>>,
    Path(claim): Path<String>,
    headers: HeaderMap,
) -> Result<Json<SweepReportResponse>, ApiError> {
    let digest = s.claim_digest(&headers)?;
    let claim = id(&claim)?;
    s.store.claim(claim, &digest).await?;
    s.store.report_sweep(claim, &digest).await?;
    Ok(Json(SweepReportResponse {
        reported: true,
        verified: false,
    }))
}
async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
#[derive(OpenApi)]
#[openapi(paths(health,ready,meta,create_room,get_room,deposit,issue,summary,events,withdraw,claim,claim_status,redeem,sweep_report),components(schemas(ErrorBody,HealthResponse,MetaResponse,CreateRoomRequest,RoomResponse,DepositResponse,TicketResponse,IssueResponse,SummaryResponse,ClaimResponse,RedeemRequest,WithdrawRequest,OperationResponse,SweepReportResponse)),modifiers(&Security),info(title="Z-Split API",version="0.1.0",description="Reservation-first custodial Orchard payouts. Bearer tickets are returned once. Sweep reports are unverified. Recipient spending keys and personal sweep destinations must never reach this service."))]
pub struct ApiDoc;
struct Security;
impl utoipa::Modify for Security {
    fn modify(&self, api: &mut utoipa::openapi::OpenApi) {
        if let Some(c) = api.components.as_mut() {
            c.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::Http::new(
                        utoipa::openapi::security::HttpAuthScheme::Bearer,
                    ),
                ),
            );
        }
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(
            state
                .config
                .allowed_origin
                .parse::<HeaderValue>()
                .expect("validated origin"),
        )
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);
    let mut app = Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/api/v1/meta", get(meta))
        .route("/openapi.json", get(openapi))
        .route("/api/v1/rooms", post(create_room))
        .route("/api/v1/rooms/{room_id}", get(get_room))
        .route("/api/v1/rooms/{room_id}/deposit", get(deposit))
        .route("/api/v1/rooms/{room_id}/claims", post(issue))
        .route("/api/v1/rooms/{room_id}/summary", get(summary))
        .route("/api/v1/rooms/{room_id}/events", get(events))
        .route("/api/v1/rooms/{room_id}/withdrawals", post(withdraw))
        .route("/api/v1/claims/{claim_id}", get(claim))
        .route("/api/v1/claims/{claim_id}/status", get(claim_status))
        .route("/api/v1/claims/{claim_id}/redeem", post(redeem))
        .route("/api/v1/claims/{claim_id}/sweep-report", post(sweep_report))
        .fallback(|| async { ApiError::from(DomainError::NotFound) });
    if state.config.docs_enabled {
        app = app.merge(
            utoipa_swagger_ui::SwaggerUi::new("/docs").url("/docs/openapi.json", ApiDoc::openapi()),
        );
    }
    app.layer(DefaultBodyLimit::max(16 * 1024))
        .layer(cors)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            secure_request,
        ))
        .with_state(state)
}
async fn secure_request(State(s): State<Arc<AppState>>, mut req: Request, next: Next) -> Response {
    let request_id = Uuid::new_v4();
    let is_docs = req.uri().path().starts_with("/docs");
    let started = Instant::now();
    let method = req.method().clone();
    let ip = req
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|c| c.0.ip())
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
    let allowed = s.limiter.lock().is_ok_and(|mut l| l.allow(ip));
    // No endpoint accepts URL query data. Reject it before tracing to prevent accidental token exposure.
    let mut response = if req.uri().query().is_some() {
        ApiError::from(DomainError::InvalidInput).into_response()
    } else if !allowed {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "Try again later",
        )
        .into_response()
    } else {
        req.headers_mut().remove("x-forwarded-for");
        req.headers_mut().remove("x-real-ip");
        match tokio::time::timeout(Duration::from_secs(30), next.run(req)).await {
            Ok(r) => r,
            Err(_) => ApiError::from(DomainError::Unavailable).into_response(),
        }
    };
    // API-generated JSON errors get the same correlation ID as response headers and structured logs.
    if response.status().is_client_error() || response.status().is_server_error() {
        let status = response.status();
        let (parts, body) = response.into_parts();
        let error = match axum::body::to_bytes(body, 16 * 1024).await {
            Ok(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes).ok(),
            Err(_) => None,
        };
        let code = error
            .as_ref()
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_str())
            .unwrap_or("invalid_request")
            .to_string();
        let message = error
            .as_ref()
            .and_then(|e| e.get("message"))
            .and_then(|c| c.as_str())
            .unwrap_or("Request unavailable")
            .to_string();
        response = (
            status,
            Json(ErrorBody {
                code,
                message,
                request_id,
            }),
        )
            .into_response();
        *response.headers_mut() = parts.headers;
        response.headers_mut().remove(header::CONTENT_LENGTH);
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
    }
    let headers = response.headers_mut();
    headers.insert(
        "x-request-id",
        request_id.to_string().parse().expect("UUID header"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    if !is_docs {
        headers.insert(
            "content-security-policy",
            HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
        );
    }
    tracing::info!(%request_id,%method,status=response.status().as_u16(),elapsed_ms=started.elapsed().as_millis(),"http_request");
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn payloads_reject_secret_fields() {
        let value = r#"{"ephemeral_receiver":"u...","idempotency_key":"00000000-0000-0000-0000-000000000000","client_network":"testnet","wallet_persisted":true,"spending_key":"forbidden"}"#;
        assert!(serde_json::from_str::<RedeemRequest>(value).is_err());
    }
    #[test]
    fn abuse_limiter_is_bounded() {
        let mut limiter = Limiter::default();
        let ip = "127.0.0.1".parse().unwrap();
        for _ in 0..240 {
            assert!(limiter.allow(ip))
        }
        assert!(!limiter.allow(ip));
    }
    #[test]
    fn origins_require_explicit_origin() {
        for value in [
            "*",
            "http://localhost.evil:3000",
            "https://example.com/path",
            "https://a.example?token=bad",
            "javascript:alert(1)",
        ] {
            assert!(validate_origin(value).is_err())
        }
        assert!(validate_origin("http://localhost:5173").is_ok());
        assert!(validate_origin("https://app.example").is_ok());
    }
    #[test]
    fn schema_covers_contract() {
        let doc = ApiDoc::openapi();
        assert_eq!(doc.paths.paths.len(), 14);
        let serialized = serde_json::to_string(&doc).unwrap();
        assert!(!serialized.contains("spending_key"));
        assert!(serialized.contains("wallet_persisted"));
        let snapshot: serde_json::Value =
            serde_json::from_str(include_str!("../../../docs/openapi.json")).unwrap();
        assert_eq!(
            serde_json::to_value(doc).unwrap(),
            snapshot,
            "OpenAPI snapshot drift; regenerate using --export-openapi"
        );
    }
}
