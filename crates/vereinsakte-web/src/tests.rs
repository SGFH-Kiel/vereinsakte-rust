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
    assert!(
        partial.body.starts_with("<div id=\"mitglieder-tabelle\""),
        "nur die Tabelle"
    );
    assert!(!partial.body.contains("<html"));
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

// --- 3.2 bis 3.5 Designsystem, Layout, Formulare, Listen, Start -----------------

#[tokio::test]
async fn navigation_marks_current_area_and_has_menu_without_javascript() {
    let h = dev();
    let mut client = Client::new(&h);
    client.dev_login(107).await;
    let page = client.get("/mitglieder").await;
    assert!(
        page.body.contains("id=\"nav-toggle\""),
        "Menü ohne JavaScript"
    );
    assert!(page.body.contains("for=\"nav-toggle\""));
    assert!(
        page.body
            .contains("href=\"/mitglieder\" aria-current=\"page\"")
    );
    assert!(!page.body.contains("href=\"/meine-daten\" aria-current"));
    assert!(page.body.contains("class=\"skip-link\""));
    assert!(page.body.contains("name=\"viewport\""));
}

#[tokio::test]
async fn pages_use_no_inline_scripts_or_styles() {
    let h = dev();
    let mut client = Client::new(&h);
    client.dev_login(105).await;
    for path in ["/", "/meine-daten", "/mitglieder", "/gibt-es-nicht"] {
        let page = client.get(path).await;
        let body = page.body.to_lowercase();
        assert!(!body.contains("<style"), "{path}: Inline-Stil");
        assert!(!body.contains(" style=\""), "{path}: style-Attribut");
        assert!(!body.contains(" onclick="), "{path}: Inline-Ereignis");
        assert!(!body.contains("<script>"), "{path}: Inline-Skript");
        for resource in body
            .split("src=\"")
            .skip(1)
            .chain(body.split("href=\"").skip(1))
        {
            let url = &resource[..resource.find('"').unwrap()];
            assert!(
                !url.starts_with("http") && !url.starts_with("//"),
                "{path}: externe Ressource {url}"
            );
        }
    }
}

#[tokio::test]
async fn directory_full_page_and_partial_show_same_rows() {
    let h = dev();
    let mut client = Client::new(&h);
    client.dev_login(105).await;
    let full = client.get("/mitglieder?q=peter").await;
    let partial = client.get_htmx("/mitglieder?q=peter").await;
    assert!(partial.status == StatusCode::OK && !partial.body.contains("<html"));
    assert!(
        full.body.contains(partial.body.trim()),
        "die Vollseite enthält genau die Teilantwort"
    );
}

#[tokio::test]
async fn start_page_shows_next_steps() {
    let h = dev();
    let mut clara = Client::new(&h);
    clara.dev_login(103).await;
    let page = clara.get("/").await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(
        page.body.contains("es fehlen 4,5 Arbeitsstunden"),
        "{}",
        page.body
    );
    assert!(page.body.contains("20,5"));
    let mut anna = Client::new(&h);
    anna.dev_login(101).await;
    let page = anna.get("/").await;
    assert!(page.body.contains("Guthaben: 10,00"));
    assert!(page.body.contains("erteilt"));
    assert!(!page.body.contains("es fehlen"));
}

#[tokio::test]
async fn session_of_former_member_ends_and_asks_to_log_in() {
    use tower_sessions::session::Record;
    // Inès scheidet zum 30.11.2026 aus; im Dezember ist sie nicht mehr aktiv.
    let h = harness(true, None, club_time(2026, 12, 1, 12, 0));
    let mut record = Record {
        id: Id::default(),
        data: [("member".to_string(), serde_json::json!(109))].into(),
        expiry_date: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
    };
    h.sessions.create(&mut record).await.unwrap();
    let mut client = Client::new(&h);
    client.cookie = Some(format!("id={}", record.id));
    let response = client.get("/meine-daten").await;
    assert_eq!(response.status, StatusCode::SEE_OTHER);
    assert!(
        response.location().starts_with("/login"),
        "{}",
        response.location()
    );
    assert!(
        h.sessions.load(&record.id).await.unwrap().is_none(),
        "Sitzung beendet"
    );
}

// --- Gruppe 4: Mitglieder -------------------------------------------------------

fn field_value(html: &str, name: &str) -> String {
    let marker = format!("name=\"{name}\" value=\"");
    let start = html
        .find(&marker)
        .unwrap_or_else(|| panic!("Feld {name} fehlt"))
        + marker.len();
    html[start..start + html[start..].find('"').unwrap()].to_string()
}

async fn logged_in(h: &Harness, member: u32) -> Client {
    let mut client = Client::new(h);
    client.dev_login(member).await;
    client
}

#[tokio::test]
async fn member_page_hides_details_from_members_without_rights() {
    let h = dev();
    let page = logged_in(&h, 107).await.get("/mitglieder/103").await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.body.contains("Clara Jensen"));
    assert!(!page.body.contains("clara.jensen@example.org"));
    assert!(!page.body.contains("Kontostand"));
    let page = logged_in(&h, 105).await.get("/mitglieder/103").await;
    assert!(
        page.body.contains("clara.jensen@example.org"),
        "Vorstand sieht Kontakt"
    );
    assert!(page.body.contains("Arbeitsstunden"));
    let own = logged_in(&h, 107).await.get("/mitglieder/107").await;
    assert!(own.body.contains("Arbeitsstunden") && own.body.contains("/mitglieder/107/bearbeiten"));
}

#[tokio::test]
async fn directory_filters_by_licence_and_links_to_member_pages() {
    let h = dev();
    let mut client = logged_in(&h, 105).await;
    let page = client.get("/mitglieder?schein=SKS").await;
    for name in [
        "Petersen, Anna",
        "Hansen, Ben",
        "Jensen, Clara",
        "Möller, Eva",
        "Koch, Frank",
    ] {
        assert!(page.body.contains(name), "{name} hat SKS");
    }
    assert!(!page.body.contains("Lorenzen, Greta"));
    assert!(page.body.contains("href=\"/mitglieder/101\""));
    let students = client.get("/mitglieder?art=student").await;
    assert!(students.body.contains("Jensen, Clara") && !students.body.contains("Möller, Eva"));
    assert!(
        page.body.contains("Mitglied aufnehmen"),
        "Vorstand darf aufnehmen"
    );
    let greta = logged_in(&h, 107).await.get("/mitglieder").await;
    assert!(!greta.body.contains("Mitglied aufnehmen"));
}

#[tokio::test]
async fn member_edits_own_contact_data_but_not_membership_type() {
    let h = dev();
    let mut anna = logged_in(&h, 101).await;
    let form = anna.get("/meine-daten/bearbeiten").await;
    assert_eq!(form.status, StatusCode::SEE_OTHER);
    let form = anna.get(form.location()).await;
    assert_eq!(form.status, StatusCode::OK);
    let (csrf_token, version) = (csrf(&form.body), field_value(&form.body, "version"));
    let saved = anna
        .post(
            "/mitglieder/101/bearbeiten",
            &[
                ("csrf", &csrf_token),
                ("version", &version),
                ("first_name", "Anna"),
                ("last_name", "Petersen"),
                ("membership_type", "regular"),
                ("street", "Neue Straße 5"),
                ("postal_code", "24103"),
                ("city", "Kiel"),
            ],
        )
        .await;
    assert_eq!(saved.status, StatusCode::SEE_OTHER, "{}", saved.body);
    let page = anna.get("/meine-daten").await;
    assert!(page.body.contains("Neue Straße 5"));
    // Veralteter Stand → 409 mit Hinweis.
    let stale = anna
        .post(
            "/mitglieder/101/bearbeiten",
            &[
                ("csrf", &csrf_token),
                ("version", &version),
                ("first_name", "Anna"),
                ("last_name", "Petersen"),
                ("membership_type", "regular"),
                ("street", "Noch neuer 1"),
                ("postal_code", "24103"),
                ("city", "Kiel"),
            ],
        )
        .await;
    assert_eq!(stale.status, StatusCode::CONFLICT);
    assert!(!anna.get("/meine-daten").await.body.contains("Noch neuer 1"));
}

#[tokio::test]
async fn member_cannot_change_membership_type_by_manual_post() {
    let h = dev();
    let mut greta = logged_in(&h, 107).await;
    let form = greta.get("/mitglieder/107/bearbeiten").await;
    assert!(!form.body.contains("name=\"membership_type\""));
    let saved = greta
        .post(
            "/mitglieder/107/bearbeiten",
            &[
                ("csrf", &csrf(&form.body)),
                ("version", &field_value(&form.body, "version")),
                ("street", "Hafenstraße 9"),
                ("postal_code", "24103"),
                ("city", "Kiel"),
                ("membership_type", "honorary"),
                ("first_name", "Hacker"),
            ],
        )
        .await;
    assert_eq!(saved.status, StatusCode::SEE_OTHER);
    let page = greta.get("/mitglieder/107").await;
    assert!(page.body.contains("außerordentliches Mitglied") && !page.body.contains("Hacker"));
    assert!(page.body.contains("Hafenstraße 9"));
    let foreign = greta.get("/mitglieder/103/bearbeiten").await;
    assert_eq!(foreign.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn invalid_contact_input_is_shown_at_the_field_with_422() {
    let h = dev();
    let mut anna = logged_in(&h, 101).await;
    let form = anna.get("/mitglieder/101/bearbeiten").await;
    let response = anna
        .post(
            "/mitglieder/101/bearbeiten",
            &[
                ("csrf", &csrf(&form.body)),
                ("version", &field_value(&form.body, "version")),
                ("street", ""),
                ("city", "Kiel"),
                ("postal_code", "24103"),
                ("email", "kaputt"),
            ],
        )
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.body.contains("Bitte die Straße angeben"));
    assert!(response.body.contains("gültige E-Mail-Adresse"));
    assert!(
        response.body.contains("value=\"kaputt\""),
        "Eingabe bleibt erhalten"
    );
}

#[tokio::test]
async fn board_admits_member_and_rejects_duplicates() {
    let h = dev();
    let mut eva = logged_in(&h, 105).await;
    let form = eva.get("/mitglieder/neu").await;
    assert_eq!(form.status, StatusCode::OK);
    let token = csrf(&form.body);
    let base = |number: &'static str, login: &'static str| {
        vec![
            ("csrf", token.clone()),
            ("number", number.into()),
            ("first_name", "Nora".into()),
            ("last_name", "Neu".into()),
            ("street", "Hafenstraße 1".into()),
            ("postal_code", "24103".into()),
            ("city", "Kiel".into()),
            ("membership_type", "regular".into()),
            ("login_email", login.into()),
        ]
    };
    let post = |fields: Vec<(&'static str, String)>| {
        let owned: Vec<(String, String)> = fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        owned
    };
    let as_refs = |owned: &Vec<(String, String)>| -> Vec<(String, String)> { owned.clone() };
    let _ = as_refs;
    let send = |fields: Vec<(&'static str, String)>| {
        let owned = post(fields);
        async move { owned }
    };
    let duplicate = send(base("101", "")).await;
    let refs: Vec<(&str, &str)> = duplicate
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let response = eva.post("/mitglieder/neu", &refs).await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.body.contains("bereits vergeben"));
    let fresh = send(base("", "nora.neu@sgfh.example")).await;
    let refs: Vec<(&str, &str)> = fresh
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let created = eva.post("/mitglieder/neu", &refs).await;
    assert_eq!(created.status, StatusCode::SEE_OTHER, "{}", created.body);
    assert_eq!(created.location(), "/mitglieder/114");
    let page = eva.get("/mitglieder/114").await;
    assert!(
        page.body.contains("Nora Neu") && page.body.contains("Das Mitglied wurde aufgenommen.")
    );
    let login_dup = send(base("", "NORA.NEU@SGFH.EXAMPLE")).await;
    let refs: Vec<(&str, &str)> = login_dup
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let response = eva.post("/mitglieder/neu", &refs).await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.body.contains("Anmeldekonto gehört bereits"));
    assert_eq!(
        logged_in(&h, 107).await.get("/mitglieder/neu").await.status,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn members_manage_their_licences_without_proof_field() {
    let h = dev();
    let mut greta = logged_in(&h, 107).await;
    let form = greta.get("/mitglieder/107/bearbeiten").await;
    assert!(!form.body.contains("type=\"file\""), "kein Upload");
    assert!(form.body.contains("Nachweis wird nicht verlangt"));
    let token = csrf(&form.body);
    let added = greta
        .post(
            "/mitglieder/107/scheine",
            &[("csrf", &token), ("kind", "sailing"), ("name", "SKS")],
        )
        .await;
    assert_eq!(added.status, StatusCode::SEE_OTHER, "{}", added.body);
    let page = greta.get(added.location()).await;
    assert!(
        page.body.contains("Der Eintrag wurde hinzugefügt.")
            && page.body.contains("<strong>SKS</strong>")
    );
    let again = greta
        .post(
            "/mitglieder/107/scheine",
            &[("csrf", &token), ("kind", "sailing"), ("name", "SKS")],
        )
        .await;
    assert_eq!(again.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(again.body.contains("gibt es bereits"));
    let foreign = greta
        .post(
            "/mitglieder/103/scheine",
            &[("csrf", &token), ("kind", "radio"), ("name", "SRC")],
        )
        .await;
    assert_eq!(foreign.status, StatusCode::FORBIDDEN);
    let removed = greta
        .post(
            "/mitglieder/107/scheine/sailing/0/entfernen",
            &[("csrf", &token)],
        )
        .await;
    assert_eq!(removed.status, StatusCode::SEE_OTHER);
    assert!(
        !greta
            .get("/mitglieder/107/bearbeiten")
            .await
            .body
            .contains("<strong>SKS</strong>")
    );
    // Der Vorstand darf für andere ergänzen.
    let mut eva = logged_in(&h, 105).await;
    let eva_form = eva.get("/mitglieder/103/bearbeiten").await;
    let ok = eva
        .post(
            "/mitglieder/103/scheine",
            &[
                ("csrf", &csrf(&eva_form.body)),
                ("kind", "radio"),
                ("name", "SRC"),
            ],
        )
        .await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn board_ends_membership_with_date() {
    let h = dev();
    let mut eva = logged_in(&h, 105).await;
    let form = eva.get("/mitglieder/107/bearbeiten").await;
    let bad = eva
        .post(
            "/mitglieder/107/austritt",
            &[
                ("csrf", &csrf(&form.body)),
                ("version", &field_value(&form.body, "version")),
                ("date", "gestern"),
            ],
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    let ok = eva
        .post(
            "/mitglieder/107/austritt",
            &[
                ("csrf", &csrf(&form.body)),
                ("version", &field_value(&form.body, "version")),
                ("date", "31.10.2026"),
            ],
        )
        .await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER, "{}", ok.body);
    let page = eva.get("/mitglieder/107").await;
    assert!(page.body.contains("Ende der Mitgliedschaft") && page.body.contains("31.10.2026"));
}

// --- Gruppe 5: Ämter -------------------------------------------------------------

fn resign_action(html: &str, holder: &str) -> String {
    let at = html
        .find(holder)
        .unwrap_or_else(|| panic!("{holder} fehlt"));
    let rest = &html[at..];
    let start = rest.find("action=\"/aemter/").expect("Abgeben-Formular") + "action=\"".len();
    rest[start..start + rest[start..].find('"').unwrap()].to_string()
}

#[tokio::test]
async fn offices_overview_for_everyone_with_admin_only_for_the_board() {
    let h = dev();
    let mut greta = logged_in(&h, 107).await;
    let page = greta.get("/aemter").await;
    assert_eq!(page.status, StatusCode::OK);
    for text in [
        "1. Vorsitzende(r)",
        "Eva Möller",
        "Bootswart (Harald Blauzahn)",
        "Ben Hansen",
    ] {
        assert!(page.body.contains(text), "{text}");
    }
    assert!(!page.body.contains("Mitglied ernennen") && !page.body.contains("Amtshistorie"));
    assert!(!page.body.contains("@example.org"), "keine Kontaktdaten");
    assert_eq!(
        greta.get("/aemter/historie").await.status,
        StatusCode::FORBIDDEN
    );
    let mut frank = logged_in(&h, 106).await;
    let page = frank.get("/aemter").await;
    assert!(page.body.contains("Mitglied ernennen") && page.body.contains("Amtshistorie"));
    let history = frank.get("/aemter/historie").await;
    assert!(history.body.contains("Eva Möller") && history.body.contains("laufend"));
}

#[tokio::test]
async fn board_appoints_and_non_board_cannot() {
    let h = dev();
    let filou = vereinsakte_core::example::FILOU;
    let role = format!("boatwarden:{filou}");
    let mut frank = logged_in(&h, 106).await;
    let page = frank.get("/aemter").await;
    let token = csrf(&page.body);
    let done = frank
        .post(
            "/aemter",
            &[("csrf", &token), ("member", "107"), ("role", &role)],
        )
        .await;
    assert_eq!(done.status, StatusCode::SEE_OTHER, "{}", done.body);
    let page = frank.get("/aemter").await;
    assert!(page.body.contains("Das Amt wurde vergeben."));
    let rows: Vec<&str> = page.body.split("Bootswart (Filou)</td>").skip(1).collect();
    assert!(
        rows.iter()
            .any(|r| r[..r.len().min(200)].contains("/mitglieder/107\">Greta Lorenzen")),
        "Greta ist jetzt Inhaberin"
    );
    let mut ben = logged_in(&h, 102).await;
    let page = ben.get("/aemter").await;
    let denied = ben
        .post(
            "/aemter",
            &[
                ("csrf", &csrf(&page.body)),
                ("member", "107"),
                ("role", "chair"),
            ],
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    let bad = frank
        .post(
            "/aemter",
            &[("csrf", &token), ("member", "107"), ("role", "unbekannt")],
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(bad.body.contains("Bitte ein Amt wählen"));
}

#[tokio::test]
async fn double_board_office_gives_a_hint_but_saves() {
    let h = dev();
    let mut eva = logged_in(&h, 105).await;
    let page = eva.get("/aemter").await;
    let done = eva
        .post(
            "/aemter",
            &[
                ("csrf", &csrf(&page.body)),
                ("member", "110"),
                ("role", "secretary"),
            ],
        )
        .await;
    assert_eq!(done.status, StatusCode::SEE_OTHER);
    let page = eva.get("/aemter").await;
    assert!(page.body.contains("Hinweis:"), "{}", page.body);
    assert!(page.body.contains("Brandt, Jonas") || page.body.contains("Jonas Brandt"));
}

#[tokio::test]
async fn resigning_validates_the_date_and_removes_the_office() {
    let h = dev();
    let mut ben = logged_in(&h, 102).await;
    let page = ben.get("/aemter").await;
    let action = resign_action(&page.body, "Ben Hansen");
    let token = csrf(&page.body);
    let early = ben
        .post(&action, &[("csrf", &token), ("date", "01.01.2026")])
        .await;
    assert_eq!(early.status, StatusCode::SEE_OTHER);
    assert!(
        ben.get("/aemter")
            .await
            .body
            .contains("nicht vor seinem Beginn")
    );
    let mut greta = logged_in(&h, 107).await;
    let greta_page = greta.get("/aemter").await;
    let foreign = greta
        .post(&action, &[("csrf", &csrf(&greta_page.body))])
        .await;
    assert_eq!(foreign.status, StatusCode::FORBIDDEN);
    let ok = ben
        .post(&action, &[("csrf", &token), ("date", "09.10.2026")])
        .await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER);
    let page = ben.get("/aemter").await;
    assert!(page.body.contains("Das Amt wurde abgegeben."));
    assert!(
        !page.body.contains("/mitglieder/102\">Ben Hansen"),
        "nicht mehr in der aktuellen Übersicht"
    );
}
