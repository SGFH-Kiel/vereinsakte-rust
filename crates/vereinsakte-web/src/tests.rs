//! Tests der Web-Schicht: Router mit In-Memory-Ports, fester Uhr und
//! Sitzungen im Speicher; Anfragen per `oneshot`.

use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use tower::ServiceExt;
use tower_sessions::session::Id;
use tower_sessions::{MemoryStore as SessionMemory, SessionStore};
use vereinsakte_app::ClubService;
use vereinsakte_app::auth::AuthService;
use vereinsakte_app::memory::{MemoryIdentityRepository, MemoryStore};
use vereinsakte_app::ports::{FixedClock, MemberRepository};
use vereinsakte_core::example::{self, ANNA};
use vereinsakte_core::time::{Timestamp, club_time};

use crate::oidc::{AuthRequest, EntraProvider, LoginProvider, VerifiedLogin};
use crate::{AppState, Settings, router};

const TENANT: &str = "tenant-1";

/// Fake-Anbieter: liefert feste Werte und die vorgegebenen Claims.
struct FakeProvider {
    login: Mutex<VerifiedLogin>,
}

impl FakeProvider {
    fn new(email: Option<&str>, preferred_username: Option<&str>, tenant: &str) -> Self {
        Self {
            login: Mutex::new(VerifiedLogin {
                issuer: EntraProvider::issuer(TENANT),
                tenant_id: Some(tenant.into()),
                object_id: Some("oid-1".into()),
                subject: "pairwise-sub".into(),
                email: email.map(Into::into),
                preferred_username: preferred_username.map(Into::into),
            }),
        }
    }
}

#[async_trait]
impl LoginProvider for FakeProvider {
    fn authorize(&self) -> AuthRequest {
        AuthRequest {
            url: "https://login.example/authorize?state=state-1"
                .parse()
                .unwrap(),
            state: "state-1".into(),
            nonce: "nonce-1".into(),
            pkce_verifier: "verifier-1".into(),
        }
    }

    async fn finish(
        &self,
        code: &str,
        verifier: &str,
        nonce: &str,
    ) -> Result<VerifiedLogin, String> {
        assert_eq!((code, verifier, nonce), ("code-1", "verifier-1", "nonce-1"));
        Ok(self.login.lock().unwrap().clone())
    }
}

struct Harness {
    app: Router,
    store: Arc<MemoryStore>,
    sessions: SessionMemory,
}

fn harness(development: bool, login: Option<Arc<dyn LoginProvider>>, now: Timestamp) -> Harness {
    let store = Arc::new(MemoryStore::new(example::club()));
    let clock = Arc::new(FixedClock(now));
    let state = AppState {
        club: ClubService::new(store.clone(), clock.clone()),
        auth: AuthService::new(
            store.clone(),
            Arc::new(MemoryIdentityRepository::default()),
            clock,
        ),
        settings: Arc::new(Settings {
            development,
            base_url: "http://localhost:8080".into(),
            session_idle: Duration::from_secs(3600),
            tenant_id: Some(TENANT.into()),
        }),
        login,
    };
    let sessions = SessionMemory::default();
    Harness {
        app: router(state, sessions.clone()),
        store,
        sessions,
    }
}

fn now() -> Timestamp {
    club_time(2026, 10, 9, 12, 0)
}

fn dev() -> Harness {
    harness(true, None, now())
}

fn prod(provider: FakeProvider) -> Harness {
    harness(false, Some(Arc::new(provider)), now())
}

struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

impl Response {
    fn location(&self) -> &str {
        self.headers
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
    }
}

/// Browser-Ersatz, der das Sitzungs-Cookie mitführt.
struct Client {
    app: Router,
    cookie: Option<String>,
    set_cookie: Option<String>,
}

impl Client {
    fn new(h: &Harness) -> Self {
        Self {
            app: h.app.clone(),
            cookie: None,
            set_cookie: None,
        }
    }

    async fn send(&mut self, mut request: Request<Body>) -> Response {
        if let Some(cookie) = &self.cookie {
            request
                .headers_mut()
                .insert(header::COOKIE, cookie.parse().unwrap());
        }
        let response = self.app.clone().oneshot(request).await.unwrap();
        if let Some(set) = response.headers().get(header::SET_COOKIE) {
            let set = set.to_str().unwrap().to_string();
            let pair = set.split(';').next().unwrap().to_string();
            self.cookie = if set.contains("Max-Age=0") {
                None
            } else {
                Some(pair)
            };
            self.set_cookie = Some(set);
        }
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        Response {
            status,
            headers,
            body: String::from_utf8_lossy(&bytes).into_owned(),
        }
    }

    async fn get(&mut self, path: &str) -> Response {
        self.send(Request::get(path).body(Body::empty()).unwrap())
            .await
    }

    async fn get_htmx(&mut self, path: &str) -> Response {
        self.send(
            Request::get(path)
                .header("hx-request", "true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    async fn post(&mut self, path: &str, fields: &[(&str, &str)]) -> Response {
        let body: String = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(fields)
            .finish();
        let request = Request::post(path)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap();
        self.send(request).await
    }

    fn session_id(&self) -> Option<Id> {
        self.cookie
            .as_ref()
            .map(|c| Id::from_str(c.trim_start_matches("id=")).unwrap())
    }

    /// Entwicklungs-Login: Auswahlliste laden, dann als `member` anmelden.
    async fn dev_login(&mut self, member: u32) -> Response {
        let page = self.get("/dev/login?next=/meine-daten").await;
        let csrf = csrf(&page.body);
        let response = self
            .post(
                "/dev/switch",
                &[
                    ("csrf", &csrf),
                    ("member", &member.to_string()),
                    ("next", "/meine-daten"),
                ],
            )
            .await;
        assert_eq!(response.status, StatusCode::SEE_OTHER, "{}", response.body);
        response
    }
}

fn csrf(html: &str) -> String {
    let marker = "name=\"csrf\" value=\"";
    let start = html.find(marker).expect("CSRF-Feld") + marker.len();
    html[start..start + html[start..].find('"').unwrap()].to_string()
}

// --- 5.1 Router, Health, statische Dateien, Logging ---------------------------

#[tokio::test]
async fn health_reports_database_state() {
    let h = dev();
    let mut client = Client::new(&h);
    let ok = client.get("/health").await;
    assert_eq!((ok.status, ok.body.as_str()), (StatusCode::OK, "ok"));
    h.store.set_available(false);
    assert_eq!(
        client.get("/health").await.status,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn htmx_is_served_with_cache_header() {
    let h = dev();
    let response = Client::new(&h).get("/static/htmx.min.js").await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(
        response.headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/javascript")
    );
    assert!(
        response.headers[header::CACHE_CONTROL]
            .to_str()
            .unwrap()
            .contains("max-age")
    );
    assert!(response.body.starts_with("var htmx="));
    assert!(
        response
            .headers
            .contains_key(header::CONTENT_SECURITY_POLICY)
    );
}

// Das Logging der Anfragen prüft der eigene Integrationstest `tests/logging.rs`.

// --- 5.2 Layout und Fehlerseiten ------------------------------------------------

#[tokio::test]
async fn unknown_route_shows_german_404() {
    let h = dev();
    let response = Client::new(&h).get("/gibt-es-nicht").await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert!(response.body.contains("Seite nicht gefunden"));
}

#[tokio::test]
async fn layout_shows_navigation_and_logout() {
    let h = dev();
    let mut client = Client::new(&h);
    client.dev_login(102).await;
    let page = client.get("/meine-daten").await;
    assert!(page.body.contains("href=\"/mitglieder\""));
    assert!(page.body.contains("Bootswart (Harald Blauzahn)"));
    assert!(page.body.contains("action=\"/logout\""));
    assert!(page.body.contains("X-CSRF-Token"));
}

// --- 5.3 Sitzung, RequireUser, CSRF ---------------------------------------------

#[tokio::test]
async fn protected_page_redirects_to_login() {
    let h = dev();
    let response = Client::new(&h).get("/meine-daten").await;
    assert_eq!(response.status, StatusCode::SEE_OTHER);
    assert_eq!(response.location(), "/login?next=%2Fmeine-daten");
}

#[tokio::test]
async fn post_without_valid_csrf_token_is_rejected() {
    let h = dev();
    let mut client = Client::new(&h);
    client.dev_login(101).await;
    assert_eq!(
        client.post("/logout", &[]).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client.post("/logout", &[("csrf", "falsch")]).await.status,
        StatusCode::FORBIDDEN
    );
    // Sitzung besteht weiter.
    assert_eq!(client.get("/meine-daten").await.status, StatusCode::OK);
}

#[tokio::test]
async fn cookie_attributes_depend_on_mode() {
    let h = dev();
    let mut client = Client::new(&h);
    client.get("/dev/login").await;
    let cookie = client.set_cookie.clone().unwrap();
    assert!(
        cookie.contains("HttpOnly") && cookie.contains("SameSite=Lax"),
        "{cookie}"
    );
    assert!(!cookie.contains("Secure"), "{cookie}");

    let h = prod(FakeProvider::new(
        Some("anna.petersen@sgfh.example"),
        None,
        TENANT,
    ));
    let mut client = Client::new(&h);
    client.get("/login").await;
    let cookie = client.set_cookie.clone().unwrap();
    assert!(
        cookie.contains("HttpOnly") && cookie.contains("SameSite=Lax") && cookie.contains("Secure"),
        "{cookie}"
    );
}

// --- 6.1 Entra-Anmeldung starten -------------------------------------------------

#[tokio::test]
async fn login_redirects_to_entra_with_pkce_and_stores_state() {
    let provider = EntraProvider::for_tests(crate::oidc::tests::metadata());
    let h = harness(false, Some(Arc::new(provider)), now());
    let mut client = Client::new(&h);
    let response = client.get("/login?next=/mitglieder").await;
    assert_eq!(response.status, StatusCode::SEE_OTHER);
    let location = url::Url::parse(response.location()).unwrap();
    assert!(
        location
            .as_str()
            .starts_with("https://login.microsoftonline.com/tenant-1/oauth2/v2.0/authorize")
    );
    let query: std::collections::HashMap<_, _> = location.query_pairs().into_owned().collect();
    assert!(!query["code_challenge"].is_empty());
    assert_eq!(query["scope"], "openid email profile");

    let record = h
        .sessions
        .load(&client.session_id().unwrap())
        .await
        .unwrap()
        .unwrap();
    let pending = &record.data["login"];
    assert_eq!(pending["state"], query["state"].as_str());
    assert_eq!(pending["nonce"], query["nonce"].as_str());
    assert_eq!(pending["next"], "/mitglieder");
}

// --- 6.2 Rückruf -------------------------------------------------------------------

async fn login_via_provider(client: &mut Client) -> Response {
    let start = client.get("/login?next=/meine-daten").await;
    assert_eq!(
        start.location(),
        "https://login.example/authorize?state=state-1"
    );
    client.get("/auth/callback?code=code-1&state=state-1").await
}

#[tokio::test]
async fn successful_login_rotates_session() {
    let h = prod(FakeProvider::new(
        Some("Anna.Petersen@sgfh.example"),
        None,
        TENANT,
    ));
    let mut client = Client::new(&h);
    client.get("/login").await;
    let before = client.session_id();
    let response = login_via_provider(&mut client).await;
    assert_eq!(
        (response.status, response.location()),
        (StatusCode::SEE_OTHER, "/meine-daten")
    );
    assert_ne!(client.session_id(), before, "neue Sitzungs-ID");
    let page = client.get("/meine-daten").await;
    assert!(page.body.contains("Anna Petersen"));
    assert!(
        !page.body.contains("devbar"),
        "keine Entwicklungsleiste im Produktivmodus"
    );
}

#[tokio::test]
async fn foreign_tenant_is_rejected() {
    let h = prod(FakeProvider::new(
        Some("anna.petersen@sgfh.example"),
        None,
        "fremder-mandant",
    ));
    let mut client = Client::new(&h);
    let response = login_via_provider(&mut client).await;
    assert_eq!(response.status, StatusCode::FORBIDDEN);
    assert_eq!(
        client.get("/meine-daten").await.status,
        StatusCode::SEE_OTHER
    );
}

#[tokio::test]
async fn preferred_username_is_used_without_email() {
    let h = prod(FakeProvider::new(
        None,
        Some("anna.petersen@sgfh.example"),
        TENANT,
    ));
    let mut client = Client::new(&h);
    assert_eq!(
        login_via_provider(&mut client).await.location(),
        "/meine-daten"
    );
}

#[tokio::test]
async fn wrong_state_creates_no_login() {
    let h = prod(FakeProvider::new(
        Some("anna.petersen@sgfh.example"),
        None,
        TENANT,
    ));
    let mut client = Client::new(&h);
    client.get("/login").await;
    let response = client.get("/auth/callback?code=code-1&state=fremd").await;
    assert_eq!(response.status, StatusCode::FORBIDDEN);
    assert_eq!(
        client.get("/meine-daten").await.status,
        StatusCode::SEE_OTHER
    );
}

#[tokio::test]
async fn unknown_account_points_to_board() {
    let h = prod(FakeProvider::new(
        Some("anna.petersen@example.org"),
        None,
        TENANT,
    ));
    let mut client = Client::new(&h);
    let response = login_via_provider(&mut client).await;
    assert_eq!(response.status, StatusCode::FORBIDDEN);
    assert!(response.body.contains("Vorstand"), "{}", response.body);
}

// --- 6.3 Abmeldung -------------------------------------------------------------------

#[tokio::test]
async fn logout_ends_session() {
    let h = dev();
    let mut client = Client::new(&h);
    client.dev_login(101).await;
    let page = client.get("/meine-daten").await;
    let response = client.post("/logout", &[("csrf", &csrf(&page.body))]).await;
    assert_eq!(response.status, StatusCode::SEE_OTHER);
    let after = client.get("/meine-daten").await;
    assert_eq!(after.status, StatusCode::SEE_OTHER);
    assert!(after.location().starts_with("/login"));
}

// --- 6.4 Entwicklungsmodus ---------------------------------------------------------

#[tokio::test]
async fn development_login_lists_members_with_offices() {
    let h = dev();
    let mut client = Client::new(&h);
    let login = client.get("/login?next=/mitglieder").await;
    assert_eq!(login.location(), "/dev/login?next=%2Fmitglieder");
    let page = client.get(login.location()).await;
    assert_eq!(page.status, StatusCode::OK);
    let entry = |name: &str| {
        let start = page
            .body
            .find(&format!(">{name}</button>"))
            .unwrap_or_else(|| panic!("{name} fehlt"));
        page.body[start..start + page.body[start..].find("</li>").unwrap()].to_string()
    };
    assert!(entry("Ben Hansen").contains("Bootswart"));
    assert!(entry("Eva Möller").contains("1. Vorsitzende(r)"));
    assert!(
        !entry("Greta Lorenzen").contains("class=\"muted\""),
        "Greta ohne Amt"
    );
}

#[tokio::test]
async fn switching_user_shows_page_from_new_perspective() {
    let h = dev();
    let mut client = Client::new(&h);
    client.dev_login(107).await; // Greta, ohne Amt
    let page = client.get("/mitglieder").await;
    assert!(!page.body.contains("@example.org"));
    let switch = client
        .post(
            "/dev/switch",
            &[
                ("csrf", &csrf(&page.body)),
                ("member", "102"),
                ("next", "/mitglieder"),
            ],
        )
        .await;
    assert_eq!(switch.location(), "/mitglieder");
    let page = client.get("/mitglieder").await;
    assert!(
        page.body.contains("anna.petersen@example.org"),
        "Ben sieht Kontaktdaten"
    );
    assert!(page.body.contains("angemeldet als Ben Hansen"));
}

#[tokio::test]
async fn development_routes_do_not_exist_in_production() {
    let h = prod(FakeProvider::new(
        Some("anna.petersen@sgfh.example"),
        None,
        TENANT,
    ));
    let mut client = Client::new(&h);
    assert_eq!(client.get("/dev/login").await.status, StatusCode::NOT_FOUND);
    assert_eq!(
        client
            .post("/dev/switch", &[("member", "101")])
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

// --- 7.1 Meine Daten -------------------------------------------------------------------

#[tokio::test]
async fn anna_sees_her_data() {
    let h = dev();
    let mut client = Client::new(&h);
    client.dev_login(101).await;
    let page = client.get("/meine-daten").await;
    for text in [
        "101",
        "ordentliches Mitglied",
        "SKS",
        "Guthaben",
        "10,00 €",
        "anna.petersen@sgfh.example",
    ] {
        assert!(page.body.contains(text), "{text} fehlt");
    }
}

#[tokio::test]
async fn clara_sees_missing_hours() {
    let h = dev();
    let mut client = Client::new(&h);
    client.dev_login(103).await;
    let page = client.get("/meine-daten").await;
    for text in [
        "es fehlen 4,5 Arbeitsstunden",
        "20,5",
        "unbestätigt",
        "Segel der Filou geflickt",
    ] {
        assert!(page.body.contains(text), "{text} fehlt");
    }
}

#[tokio::test]
async fn invalid_address_is_highlighted() {
    let h = dev();
    let anna = h.store.member(ANNA).await.unwrap().unwrap();
    let mut changed = anna.value.clone();
    changed.address_valid = false;
    h.store.save_member(&changed, anna.version).await.unwrap();
    let mut client = Client::new(&h);
    client.dev_login(101).await;
    assert!(
        client
            .get("/meine-daten")
            .await
            .body
            .contains("Meldeadresse ist als ungültig markiert")
    );
}

#[tokio::test]
async fn foreign_member_number_in_url_is_ignored() {
    let h = dev();
    let mut client = Client::new(&h);
    client.dev_login(103).await;
    let page = client.get("/meine-daten?member=101").await;
    assert!(page.body.contains("Clara Jensen"));
    assert!(
        !page.body.contains("Holtenauer Straße"),
        "keine Daten von Anna"
    );
}

// --- 7.2 Mitgliederverzeichnis ------------------------------------------------------------

#[tokio::test]
async fn directory_hides_contact_data_without_office() {
    let h = dev();
    let mut client = Client::new(&h);
    client.dev_login(107).await;
    let page = client.get("/mitglieder").await;
    assert!(page.body.contains("Petersen, Anna"));
    assert!(!page.body.contains("@example.org"));
    let partial = client.get_htmx("/mitglieder?q=").await;
    assert!(partial.body.starts_with("<table"), "nur die Tabelle");
    assert!(!partial.body.contains("@example.org"));
}

#[tokio::test]
async fn directory_search_by_name_part() {
    let h = dev();
    let mut client = Client::new(&h);
    client.dev_login(102).await;
    let partial = client.get_htmx("/mitglieder?q=peter").await;
    assert!(partial.body.contains("Petersen, Anna") && partial.body.contains("Petersen, Hannes"));
    assert_eq!(partial.body.matches("<tr>").count(), 3, "Kopf + 2 Zeilen");
    assert!(partial.body.contains("anna.petersen@example.org"));
}

#[tokio::test]
async fn directory_hides_former_members() {
    let h = harness(true, None, club_time(2026, 11, 15, 12, 0));
    let mut client = Client::new(&h);
    client.dev_login(102).await;
    let page = client.get("/mitglieder").await;
    assert!(!page.body.contains("Dupont"));
    assert!(page.body.contains("Möller, Eva"));
}
