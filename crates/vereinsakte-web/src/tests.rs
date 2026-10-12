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

    /// Mehrteil-Upload mit Feldern `file` (Datei) und `alt`; das Token steht in der Adresse.
    async fn upload(
        &mut self,
        path: &str,
        token: &str,
        filename: &str,
        data: &[u8],
        alt: &str,
    ) -> Response {
        let boundary = "----testgrenze";
        let mut body: Vec<u8> = Vec::new();
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"alt\"\r\n\r\n{alt}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(data);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let request = Request::post(format!("{path}?csrf={token}"))
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
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
    assert!(
        page.body.contains("anna.petersen@example.org"),
        "Kontaktdaten sind vereinsintern sichtbar"
    );
    assert!(page.body.contains("angemeldet als Greta Lorenzen"));
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
async fn directory_shows_contact_data_to_every_member() {
    let h = dev();
    let mut client = Client::new(&h);
    client.dev_login(107).await; // Greta, ohne Amt
    let page = client.get("/mitglieder").await;
    assert!(page.body.contains("Petersen, Anna"));
    assert!(page.body.contains("anna.petersen@example.org"));
    assert!(
        !page
            .body
            .contains("Kontaktdaten sind nur für Mitglieder mit einem Amt")
    );
    let partial = client.get_htmx("/mitglieder?q=").await;
    assert!(
        partial.body.starts_with("<div id=\"mitglieder-tabelle\""),
        "nur die Tabelle"
    );
    assert!(partial.body.contains("anna.petersen@example.org"));
}

async fn set_contact_block(h: &Harness, member: u32, blocked: bool) -> Response {
    let mut client = logged_in(h, member).await;
    let page = client.get("/meine-daten").await;
    let token = csrf(&page.body);
    let mut fields = vec![("csrf", token.as_str())];
    if blocked {
        fields.push(("blocked", "1"));
    }
    client.post("/meine-daten/sperrvermerk", &fields).await
}

#[tokio::test]
async fn blocked_contact_data_is_not_in_any_page_for_others() {
    let h = dev();
    let response = set_contact_block(&h, 101, true).await; // Anna
    assert_eq!(response.location(), "/meine-daten");
    for (viewer, expects_blocked) in [
        (107, true),
        (102, true),
        (103, true),
        (105, false),
        (106, false),
        (101, false),
    ] {
        let mut client = logged_in(&h, viewer).await;
        let pages = [
            client.get("/mitglieder").await,
            client.get_htmx("/mitglieder?q=").await,
            client.get("/mitglieder/101").await,
            client.get("/aemter").await,
        ];
        let any = pages
            .iter()
            .any(|p| p.body.contains("anna.petersen@example.org"));
        if expects_blocked {
            assert!(!any, "Betrachter {viewer} darf Annas E-Mail nicht sehen");
            assert!(pages[0].body.contains("Kontaktdaten gesperrt"));
            assert!(pages[2].body.contains("Kontaktdaten gesperrt"));
        } else {
            assert!(
                pages[0].body.contains("anna.petersen@example.org"),
                "Betrachter {viewer} sieht Annas E-Mail"
            );
        }
    }
    // Zurücknehmen
    set_contact_block(&h, 101, false).await;
    let page = logged_in(&h, 107).await.get("/mitglieder").await;
    assert!(page.body.contains("anna.petersen@example.org"));
}

#[tokio::test]
async fn contact_block_switch_shows_state_and_is_only_for_oneself() {
    let h = dev();
    let mut anna = logged_in(&h, 101).await;
    let page = anna.get("/meine-daten").await;
    assert!(page.body.contains("Kontaktdaten nicht anzeigen"));
    assert!(!page.body.contains("name=\"blocked\" value=\"1\" checked"));
    set_contact_block(&h, 101, true).await;
    let page = anna.get("/meine-daten").await;
    assert!(page.body.contains("name=\"blocked\" value=\"1\" checked"));
    // Das Formular kennt kein Zielmitglied: ein fremdes lässt sich nicht setzen.
    let form_start = page
        .body
        .find("action=\"/meine-daten/sperrvermerk\"")
        .unwrap();
    let form =
        &page.body[form_start..form_start + page.body[form_start..].find("</form>").unwrap()];
    assert!(!form.contains("name=\"member\""));
    let log = logged_in(&h, 105).await.get("/protokoll").await;
    assert!(log.body.contains("Sperrvermerk geändert"));
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
        page.body.contains("id=\"more-toggle\""),
        "Menü ohne JavaScript"
    );
    assert!(page.body.contains("for=\"more-toggle\""));
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
        // Ressourcen (Bilder, Skripte, Stylesheets, Symbole) nur von der eigenen Adresse;
        // Verweise (<a href>) auf die Vereinswebseite sind Navigation, keine Ressource.
        let links = body
            .split("<link ")
            .skip(1)
            .filter_map(|tag| tag.split("href=\"").nth(1));
        for resource in body.split("src=\"").skip(1).chain(links) {
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

/// Wert eines Eingabefelds nach Name (auch wenn `type` zwischen Name und Wert steht).
fn input_value(html: &str, name: &str) -> String {
    let at = html
        .find(&format!("name=\"{name}\""))
        .unwrap_or_else(|| panic!("Feld {name} fehlt"));
    let rest = &html[at..];
    let tag_end = rest.find('>').unwrap();
    let marker = "value=\"";
    let start = rest[..tag_end]
        .find(marker)
        .map_or(tag_end, |i| i + marker.len());
    if start >= tag_end {
        return String::new();
    }
    rest[start..start + rest[start..].find('"').unwrap()].to_string()
}

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
    assert!(
        page.body.contains("clara.jensen@example.org"),
        "Kontakt ist vereinsintern sichtbar"
    );
    assert!(!page.body.contains("Kontostand"));
    assert!(!page.body.contains("Arbeitsstunden"));
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
    assert!(!page.body.contains("Mitglied ernennen") && !page.body.contains("Archiv"));
    assert!(
        page.body.contains("@example.org"),
        "Kontaktdaten der Inhaber sind sichtbar"
    );
    assert_eq!(
        greta.get("/protokoll/archiv").await.status,
        StatusCode::FORBIDDEN
    );
    let mut frank = logged_in(&h, 106).await;
    let page = frank.get("/aemter").await;
    assert!(page.body.contains("Mitglied ernennen") && page.body.contains("im Archiv"));
    // Die frühere Historienseite führt ins Archiv.
    let old = frank.get("/aemter/historie").await;
    assert_eq!(old.location(), "/protokoll/archiv?tab=aemter");
    let history = frank.get("/protokoll/archiv?tab=aemter").await;
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
    let overview = page.body.split("<main").nth(1).unwrap();
    assert!(
        !overview.contains("/mitglieder/102\">Ben Hansen"),
        "nicht mehr in der aktuellen Übersicht"
    );
}

// --- Gruppe 6: Boote -------------------------------------------------------------

fn boat_path(boat: vereinsakte_core::ids::BoatId) -> String {
    format!("/boote/{boat}")
}

#[tokio::test]
async fn boat_list_and_page_show_state_and_rights_dependent_forms() {
    use vereinsakte_core::example::FILOU;
    let h = dev();
    let mut greta = logged_in(&h, 107).await;
    let list = greta.get("/boote").await;
    assert_eq!(list.status, StatusCode::OK);
    assert!(
        list.body.contains("Filou")
            && list.body.contains("Trigon")
            && list.body.contains("freigegeben")
    );
    assert!(!list.body.contains("Boot anlegen"));
    let page = greta.get(&boat_path(FILOU)).await;
    assert!(page.body.contains("Deine Eignung") && page.body.contains("nicht möglich"));
    for hidden in ["/sperren\"", "/abnahme\"", "/widerruf\"", "/bearbeiten"] {
        assert!(!page.body.contains(hidden), "Greta sieht {hidden} nicht");
    }
    let mut david = logged_in(&h, 104).await;
    let page = david.get(&boat_path(FILOU)).await;
    assert!(
        page.body.contains("/sperren\"")
            && page.body.contains("/abnahme\"")
            && page.body.contains("/bearbeiten")
    );
    assert!(
        !page.body.contains("/widerruf\""),
        "Widerruf nur für den Vorstand"
    );
    assert!(
        logged_in(&h, 106)
            .await
            .get("/boote")
            .await
            .body
            .contains("Boot anlegen")
    );
}

#[tokio::test]
async fn warden_blocks_and_lifts_and_others_are_refused() {
    use vereinsakte_core::example::{FILOU, HARALD_BLAUZAHN};
    let h = dev();
    let mut ben = logged_in(&h, 102).await;
    let path = boat_path(HARALD_BLAUZAHN);
    let page = ben.get(&path).await;
    let token = csrf(&page.body);
    let blocked = ben
        .post(
            &format!("{path}/sperren"),
            &[("csrf", &token), ("reason", "Mast defekt")],
        )
        .await;
    assert_eq!(blocked.status, StatusCode::SEE_OTHER, "{}", blocked.body);
    let page = ben.get(&path).await;
    assert!(page.body.contains("Das Boot wurde gesperrt.") && page.body.contains("Mast defekt"));
    assert!(page.body.contains("Sperre aufheben"));
    assert!(ben.get("/boote").await.body.contains("gesperrt"));
    let foreign = ben
        .post(
            &format!("{}/sperren", boat_path(FILOU)),
            &[("csrf", &token), ("reason", "x")],
        )
        .await;
    assert_eq!(foreign.status, StatusCode::FORBIDDEN);
    let lifted = ben
        .post(&format!("{path}/entsperren"), &[("csrf", &token)])
        .await;
    assert_eq!(lifted.status, StatusCode::SEE_OTHER);
    let again = ben
        .post(&format!("{path}/entsperren"), &[("csrf", &token)])
        .await;
    assert_eq!(again.status, StatusCode::SEE_OTHER);
    assert!(
        ben.get(&path)
            .await
            .body
            .contains("Es gibt keine offene Sperre.")
    );
    let empty = ben
        .post(
            &format!("{path}/sperren"),
            &[("csrf", &token), ("reason", "")],
        )
        .await;
    assert_eq!(empty.status, StatusCode::SEE_OTHER);
    assert!(
        ben.get(&path)
            .await
            .body
            .contains("Bitte einen Grund angeben.")
    );
    let mut greta = logged_in(&h, 107).await;
    let greta_token = csrf(&greta.get(&path).await.body);
    let denied = greta
        .post(
            &format!("{path}/sperren"),
            &[("csrf", &greta_token), ("reason", "x")],
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn checkout_and_revocation_change_the_boat_permit_shown_to_the_member() {
    use vereinsakte_core::example::FILOU;
    let h = dev();
    let path = boat_path(FILOU);
    let mut david = logged_in(&h, 104).await;
    let token = csrf(&david.get(&path).await.body);
    let done = david
        .post(
            &format!("{path}/abnahme"),
            &[("csrf", &token), ("member", "107"), ("result", "passed")],
        )
        .await;
    assert_eq!(done.status, StatusCode::SEE_OTHER, "{}", done.body);
    let mut greta = logged_in(&h, 107).await;
    let page = greta.get(&path).await;
    assert!(!page.body.contains("keine bestandene Bootsabnahme"));
    let denied = david
        .post(
            &format!("{path}/widerruf"),
            &[("csrf", &token), ("member", "107"), ("reason", "x")],
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    let mut eva = logged_in(&h, 105).await;
    let eva_token = csrf(&eva.get(&path).await.body);
    let revoked = eva
        .post(
            &format!("{path}/widerruf"),
            &[
                ("csrf", &eva_token),
                ("member", "107"),
                ("reason", "Unfall"),
            ],
        )
        .await;
    assert_eq!(revoked.status, StatusCode::SEE_OTHER);
    assert!(
        greta
            .get(&path)
            .await
            .body
            .contains("Bootserlaubnis widerrufen")
    );
    let mut greta_post = logged_in(&h, 107).await;
    let denied = greta_post
        .post(
            &format!("{path}/abnahme"),
            &[
                ("csrf", &csrf(&greta.get(&path).await.body)),
                ("member", "107"),
            ],
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn boat_forms_validate_and_create() {
    let h = dev();
    let mut frank = logged_in(&h, 106).await;
    let form = frank.get("/boote/neu").await;
    assert_eq!(form.status, StatusCode::OK);
    let token = csrf(&form.body);
    let bad = frank
        .post(
            "/boote/neu",
            &[
                ("csrf", &token),
                ("name", "Neptun"),
                ("boat_type", "dinghy"),
                ("length", "abc"),
            ],
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        bad.body.contains("Bitte eine positive Zahl eingeben.")
            && bad.body.contains("value=\"Neptun\"")
    );
    let ok = frank
        .post(
            "/boote/neu",
            &[
                ("csrf", &token),
                ("name", "Neptun"),
                ("boat_type", "dinghy"),
                ("length", "4,5"),
                ("skipper_requirement", "SKS | SSS"),
                ("home_min_members", "1"),
                ("home_min_persons", "2"),
            ],
        )
        .await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER, "{}", ok.body);
    let page = frank.get(ok.location()).await;
    assert!(
        page.body.contains("Das Boot wurde angelegt.")
            && page.body.contains("SKS | SSS")
            && page.body.contains("4,5 m")
    );
    let edit = frank.get(&format!("{}/bearbeiten", ok.location())).await;
    assert_eq!(edit.status, StatusCode::OK);
    assert!(edit.body.contains("SKS | SSS"));
    assert_eq!(
        logged_in(&h, 107).await.get("/boote/neu").await.status,
        StatusCode::FORBIDDEN
    );
}

// --- Gruppe 7: Arbeit --------------------------------------------------------------

fn path_of(html: &str, prefix: &str, suffix: &str) -> String {
    let start = html
        .find(prefix)
        .unwrap_or_else(|| panic!("{prefix} fehlt"))
        + prefix.len();
    let end = start + html[start..].find(suffix).unwrap();
    html[start..end].to_string()
}

/// Kennung aus der ersten Adresse `prefix…/` nach der Textstelle `marker`.
fn id_after(html: &str, marker: &str, prefix: &str) -> String {
    let at = html
        .find(marker)
        .unwrap_or_else(|| panic!("{marker} fehlt"));
    path_of(&html[at..], prefix, "/")
}

#[tokio::test]
async fn member_records_work_and_confirmed_entries_are_locked() {
    use vereinsakte_core::example::HARALD_BLAUZAHN;
    let h = dev();
    let mut clara = logged_in(&h, 103).await;
    let page = clara.get("/arbeit").await;
    assert_eq!(page.status, StatusCode::OK);
    let token = csrf(&page.body);
    let boat = HARALD_BLAUZAHN.to_string();
    let both = clara
        .post(
            "/arbeit/einsaetze",
            &[
                ("csrf", &token),
                ("date", "2026-10-01"),
                ("hours", "3"),
                ("start", "08:00"),
                ("end", "10:00"),
                ("subject", &boat),
            ],
        )
        .await;
    assert_eq!(both.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(both.body.contains("nicht beides") && both.body.contains("value=\"2026-10-01\""));
    let ok = clara
        .post(
            "/arbeit/einsaetze",
            &[
                ("csrf", &token),
                ("date", "2026-10-01"),
                ("hours", "3"),
                ("subject", &boat),
                ("description", "Antifouling"),
            ],
        )
        .await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER, "{}", ok.body);
    let page = clara.get("/arbeit").await;
    assert!(page.body.contains("Der Einsatz wurde erfasst") && page.body.contains("Antifouling"));
    assert!(page.body.contains("unbestätigt") && page.body.contains("20,5"));
    // Bestätigte Einsätze lassen sich nicht ändern.
    let confirmed_row = path_of(&page.body, "bestätigt</span>", "</tr>");
    assert!(
        !confirmed_row.contains("/bearbeiten"),
        "bestätigt ohne Ändern-Link"
    );
    let edit = path_of(&page.body, "/arbeit/einsaetze/", "/bearbeiten");
    let form = clara
        .get(&format!("/arbeit/einsaetze/{edit}/bearbeiten"))
        .await;
    assert_eq!(form.status, StatusCode::OK);
}

#[tokio::test]
async fn warden_confirms_and_others_cannot() {
    use vereinsakte_core::example::HARALD_BLAUZAHN;
    let h = dev();
    let mut clara = logged_in(&h, 103).await;
    let token = csrf(&clara.get("/arbeit").await.body);
    clara
        .post(
            "/arbeit/einsaetze",
            &[
                ("csrf", &token),
                ("date", "2026-10-01"),
                ("hours", "3"),
                ("subject", &HARALD_BLAUZAHN.to_string()),
            ],
        )
        .await;
    let mut ben = logged_in(&h, 102).await;
    let page = ben.get("/arbeit/bestaetigungen").await;
    assert!(page.body.contains("Clara Jensen"));
    let id = path_of(&page.body, "/arbeit/einsaetze/", "/bestaetigen");
    let mut greta = logged_in(&h, 107).await;
    let greta_token = csrf(&greta.get("/arbeit").await.body);
    let denied = greta
        .post(
            &format!("/arbeit/einsaetze/{id}/bestaetigen"),
            &[("csrf", &greta_token)],
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    assert!(
        !greta
            .get("/arbeit")
            .await
            .body
            .contains("/arbeit/bestaetigungen"),
        "kein Reiter ohne Amt"
    );
    let ok = ben
        .post(
            &format!("/arbeit/einsaetze/{id}/bestaetigen"),
            &[("csrf", &csrf(&page.body))],
        )
        .await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER);
    assert!(
        ben.get("/arbeit/bestaetigungen")
            .await
            .body
            .contains("nichts zu bestätigen")
    );
    assert!(clara.get("/arbeit").await.body.contains("23,5"));
}

#[tokio::test]
async fn events_registration_and_start_page_card() {
    let h = dev();
    let mut frank = logged_in(&h, 106).await;
    let page = frank.get("/arbeit/termine/neu").await;
    assert_eq!(page.status, StatusCode::OK);
    let token = csrf(&page.body);
    let created = frank
        .post(
            "/arbeit/termine",
            &[
                ("csrf", &token),
                ("title", "Herbstputz"),
                ("date", "2026-11-07"),
                ("start", "10:00"),
                ("end", "14:00"),
                ("max", "1"),
            ],
        )
        .await;
    assert_eq!(created.status, StatusCode::SEE_OTHER, "{}", created.body);
    let detail = created.location().to_string();
    assert!(detail.starts_with("/arbeit/termine/"));
    let bad = frank
        .post(
            "/arbeit/termine",
            &[
                ("csrf", &token),
                ("title", "Falsch"),
                ("date", "2026-11-07"),
                ("start", "14:00"),
                ("end", "10:00"),
            ],
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(bad.body.contains("Das Ende muss nach dem Beginn liegen."));
    // Die Terminliste lebt im Kalender.
    let list = frank.get("/arbeit/termine").await;
    assert_eq!(list.location(), "/kalender?art=termine");
    let calendar = frank.get("/kalender?art=termine").await;
    assert!(calendar.body.contains("Herbstputz") && calendar.body.contains("cal-entry event"));
    let mut anna = logged_in(&h, 101).await;
    let anna_token = csrf(&anna.get(&detail).await.body);
    let ok = anna
        .post(&format!("{detail}/anmelden"), &[("csrf", &anna_token)])
        .await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER);
    assert_eq!(ok.location(), detail);
    assert!(
        anna.get("/")
            .await
            .body
            .contains("Anstehende Arbeitstermine")
    );
    let mut greta = logged_in(&h, 107).await;
    let greta_page = greta.get(&detail).await;
    let greta_token = csrf(&greta_page.body);
    let full = greta
        .post(&format!("{detail}/anmelden"), &[("csrf", &greta_token)])
        .await;
    assert_eq!(full.status, StatusCode::SEE_OTHER);
    let page = greta.get(&detail).await;
    assert!(page.body.contains("ausgebucht") && !page.body.contains("/bearbeiten"));
    assert!(page.body.contains("schreibt keine Stunden gut"));
    assert!(page.body.contains("/mitglieder/101"), "Teilnehmer als Link");
    let denied = greta
        .post(
            "/arbeit/termine",
            &[
                ("csrf", &greta_token),
                ("title", "x"),
                ("date", "2026-11-07"),
                ("start", "10:00"),
                ("end", "11:00"),
            ],
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    assert_eq!(
        greta.get("/arbeit/termine/neu").await.status,
        StatusCode::FORBIDDEN
    );
}

fn location(response: &Response) -> String {
    response
        .headers
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

/// Legt als Ben eine Aufgabe an und liefert ihre Kennung.
async fn create_task(ben: &mut Client, title: &str) -> String {
    let page = ben.get("/aufgaben/neu").await;
    assert_eq!(page.status, StatusCode::OK);
    let created = ben
        .post(
            "/aufgaben/neu",
            &[
                ("csrf", &csrf(&page.body)),
                ("title", title),
                ("hours", "2,5"),
            ],
        )
        .await;
    assert_eq!(created.status, StatusCode::SEE_OTHER, "{}", created.body);
    location(&created)
        .strip_prefix("/aufgaben/")
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn tasks_are_created_assigned_moved_and_shown_on_the_start_page() {
    let h = dev();
    let mut ben = logged_in(&h, 102).await;
    let id = create_task(&mut ben, "Fender flicken").await;
    let board = ben.get("/aufgaben").await;
    assert!(board.body.contains("Fender flicken") && board.body.contains("TODO"));
    for tag in ["TODO", "WIP", "WAIT", "DONE"] {
        assert!(board.body.contains(tag), "{tag}");
    }
    let token = csrf(&board.body);
    let assigned = ben
        .post(
            &format!("/aufgaben/{id}/zuweisen"),
            &[("csrf", &token), ("member", "107")],
        )
        .await;
    assert_eq!(assigned.status, StatusCode::SEE_OTHER);
    // „meine“ filtert nach Zuständigkeit.
    let mut greta = logged_in(&h, 107).await;
    let start = greta.get("/").await;
    assert!(start.body.contains("Meine Aufgaben") && start.body.contains("Fender flicken"));
    let mine = greta.get("/aufgaben?meine=1").await;
    assert!(mine.body.contains("Fender flicken"));
    assert!(
        !ben.get("/aufgaben?meine=1")
            .await
            .body
            .contains("Fender flicken"),
        "Ben ist nicht zuständig"
    );
    let greta_token = csrf(&start.body);
    let moved = greta
        .post(
            &format!("/aufgaben/{id}/status"),
            &[
                ("csrf", &greta_token),
                ("status", "waiting"),
                ("next", "board"),
            ],
        )
        .await;
    assert_eq!(moved.status, StatusCode::SEE_OTHER);
    assert_eq!(location(&moved), "/aufgaben");
    let page = greta.get(&format!("/aufgaben/{id}")).await;
    assert!(page.body.contains("Wartet"), "{}", page.body);
    let done = greta
        .post(
            &format!("/aufgaben/{id}/status"),
            &[("csrf", &greta_token), ("status", "done")],
        )
        .await;
    assert_eq!(done.status, StatusCode::SEE_OTHER);
    assert!(
        !greta.get("/").await.body.contains("Meine Aufgaben"),
        "erledigt ist nicht mehr offen"
    );
    let mut hannes = logged_in(&h, 108).await;
    let hannes_page = hannes.get(&format!("/aufgaben/{id}")).await;
    assert!(!hannes_page.body.contains("Status setzen"));
    let hannes_token = csrf(&hannes.get("/aufgaben").await.body);
    let denied = hannes
        .post(
            &format!("/aufgaben/{id}/status"),
            &[("csrf", &hannes_token), ("status", "open")],
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    // Nicht-Amtsträger sehen weder „Neue Aufgabe“ noch dürfen sie anlegen.
    assert_eq!(
        hannes.get("/aufgaben/neu").await.status,
        StatusCode::FORBIDDEN
    );
    assert!(!hannes.get("/aufgaben").await.body.contains("/aufgaben/neu"));
}

#[tokio::test]
async fn old_task_list_redirects_to_the_board() {
    let h = dev();
    let r = logged_in(&h, 107).await.get("/arbeit/aufgaben").await;
    assert_eq!(r.status, StatusCode::SEE_OTHER);
    assert_eq!(location(&r), "/aufgaben");
}

#[tokio::test]
async fn move_conflict_is_reported_and_ajax_moves_answer_204() {
    let h = dev();
    let mut ben = logged_in(&h, 102).await;
    let id = create_task(&mut ben, "Fock nähen").await;
    let board = ben.get("/aufgaben").await;
    let token = csrf(&board.body);
    let marker = format!("data-id=\"{id}\" data-version=\"");
    let at = board.body.find(&marker).unwrap() + marker.len();
    let version = &board.body[at..at + board.body[at..].find('"').unwrap()];
    let first = ben
        .post(
            &format!("/aufgaben/{id}/status"),
            &[
                ("csrf", &token),
                ("status", "in_progress"),
                ("version", version),
                ("ajax", "1"),
            ],
        )
        .await;
    assert_eq!(first.status, StatusCode::NO_CONTENT);
    let second = ben
        .post(
            &format!("/aufgaben/{id}/status"),
            &[("csrf", &token), ("status", "done"), ("version", version)],
        )
        .await;
    assert_eq!(second.status, StatusCode::CONFLICT);
    assert!(second.body.contains("zwischenzeitlich"));
    let bad = ben
        .post(
            &format!("/aufgaben/{id}/status"),
            &[("csrf", &token), ("status", "bogus")],
        )
        .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn markdown_description_preview_and_safety() {
    let h = dev();
    let mut ben = logged_in(&h, 102).await;
    let id = create_task(&mut ben, "Segel flicken").await;
    let page = ben.get(&format!("/aufgaben/{id}")).await;
    let token = csrf(&page.body);
    let version = "1";
    let evil = "**Wichtig:** Fock <script>alert(1)</script> [x](javascript:alert(1))";
    let preview = ben
        .post(
            &format!("/aufgaben/{id}/beschreibung"),
            &[
                ("csrf", &token),
                ("action", "preview"),
                ("text", evil),
                ("version", version),
            ],
        )
        .await;
    assert_eq!(preview.status, StatusCode::OK);
    assert!(preview.body.contains("<strong>Wichtig:</strong>"));
    assert!(preview.body.contains("noch nicht gespeichert"));
    assert!(
        !preview.body.contains("<script>alert") && !preview.body.contains("href=\"javascript:")
    );
    assert!(preview.body.contains("&lt;script&gt;"));
    // Vorschau speichert nicht.
    assert!(
        ben.get(&format!("/aufgaben/{id}"))
            .await
            .body
            .contains("Noch keine Beschreibung")
    );
    let saved = ben
        .post(
            &format!("/aufgaben/{id}/beschreibung"),
            &[
                ("csrf", &token),
                ("action", "save"),
                ("text", evil),
                ("version", version),
            ],
        )
        .await;
    assert_eq!(saved.status, StatusCode::SEE_OTHER);
    let page = ben.get(&format!("/aufgaben/{id}")).await;
    assert!(
        page.body.contains("<strong>Wichtig:</strong>") && !page.body.contains("<script>alert")
    );
    // Zu lang
    let long = "a".repeat(20_001);
    let too_long = ben
        .post(
            &format!("/aufgaben/{id}/beschreibung"),
            &[("csrf", &token), ("action", "save"), ("text", &long)],
        )
        .await;
    assert_eq!(too_long.status, StatusCode::UNPROCESSABLE_ENTITY);
    // Fremde: 403
    let mut greta = logged_in(&h, 107).await;
    let greta_token = csrf(&greta.get("/aufgaben").await.body);
    let denied = greta
        .post(
            &format!("/aufgaben/{id}/beschreibung"),
            &[("csrf", &greta_token), ("action", "preview"), ("text", "x")],
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn comments_are_listed_counted_and_validated() {
    let h = dev();
    let mut ben = logged_in(&h, 102).await;
    let id = create_task(&mut ben, "Segeltuch bestellen").await;
    let mut greta = logged_in(&h, 107).await;
    let token = csrf(&greta.get(&format!("/aufgaben/{id}")).await.body);
    let empty = greta
        .post(
            &format!("/aufgaben/{id}/kommentar"),
            &[("csrf", &token), ("text", "  ")],
        )
        .await;
    assert_eq!(empty.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(empty.body.contains("Bitte einen Kommentar eingeben"));
    let ok = greta
        .post(
            &format!("/aufgaben/{id}/kommentar"),
            &[
                ("csrf", &token),
                ("text", "Segeltuch ist **bestellt** <b>x</b>"),
            ],
        )
        .await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER);
    let page = greta.get(&format!("/aufgaben/{id}")).await;
    assert!(page.body.contains("<strong>bestellt</strong>") && page.body.contains("Greta"));
    assert!(!page.body.contains("<b>x</b>"));
    // Eigener Kommentar ist korrigierbar, fremder nicht.
    assert!(page.body.contains("Ändern oder löschen"));
    assert!(
        !ben.get(&format!("/aufgaben/{id}"))
            .await
            .body
            .contains("Korrigieren")
    );
    let board = ben.get("/aufgaben").await;
    let card = &board.body[board.body.find(&format!("id=\"aufgabe-{id}\"")).unwrap()..];
    assert!(card.contains("Kommentare"));
}

#[tokio::test]
async fn attachments_are_checked_served_safely_and_deleted_by_owner_or_board() {
    let h = dev();
    let mut ben = logged_in(&h, 102).await;
    let id = create_task(&mut ben, "Schaden dokumentieren").await;
    let page = ben.get(&format!("/aufgaben/{id}")).await;
    let token = csrf(&page.body);
    let up = format!("/aufgaben/{id}/anhang");
    let jpeg = [
        0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10, b'J', b'F', b'I', b'F', 0, 1,
    ];
    let ok = ben
        .upload(&up, &token, "schaden.jpg", &jpeg, "Riss im Segel")
        .await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER, "{}", ok.body);
    let pdf = ben
        .upload(
            &up,
            &token,
            "../Angebot Segeltuch.pdf",
            b"%PDF-1.4\n%%EOF",
            "",
        )
        .await;
    assert_eq!(pdf.status, StatusCode::SEE_OTHER, "{}", pdf.body);
    for (name, data) in [
        ("bild.png", b"just text".to_vec()),
        ("angebot.pdf", b"just text".to_vec()),
        (
            "x.svg",
            b"<svg xmlns='http://www.w3.org/2000/svg' onload='alert(1)'/>".to_vec(),
        ),
        ("brief.docx", b"PK\x03\x04 word".to_vec()),
    ] {
        let bad = ben.upload(&up, &token, name, &data, "").await;
        assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY, "{name}");
        assert!(bad.body.contains("Erlaubt sind Bilder"), "{name}");
    }
    let mut huge = vec![0xFF, 0xD8, 0xFF];
    huge.resize(10 * 1024 * 1024 + 10, 0);
    let big = ben.upload(&up, &token, "gross.jpg", &huge, "").await;
    assert_eq!(big.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(big.body.contains("zu groß"));
    // Ohne Token: abgelehnt.
    let no_token = ben.upload(&up, "falsch", "a.jpg", &jpeg, "").await;
    assert_eq!(no_token.status, StatusCode::FORBIDDEN);

    let page = ben.get(&format!("/aufgaben/{id}")).await;
    assert!(
        page.body.contains("alt=\"Riss im Segel\"") && page.body.contains("Angebot Segeltuch.pdf")
    );
    let image_id = path_of(
        &page.body[page.body.find("<img").unwrap()..],
        "/aufgaben/anhang/",
        "\"",
    );
    let file_id = path_of(
        &page.body[page.body.find("file-row").unwrap()..],
        "/aufgaben/anhang/",
        "\"",
    );
    let image = ben.get(&format!("/aufgaben/anhang/{image_id}")).await;
    assert_eq!(image.status, StatusCode::OK);
    assert_eq!(image.headers[header::CONTENT_TYPE], "image/jpeg");
    assert_eq!(image.headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert!(
        image.headers[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap()
            .contains("sandbox")
    );
    let download = ben.get(&format!("/aufgaben/anhang/{file_id}")).await;
    assert_eq!(download.headers[header::CONTENT_TYPE], "application/pdf");
    let disposition = download.headers[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap();
    assert!(
        disposition.starts_with("attachment;") && disposition.contains("Angebot%20Segeltuch.pdf")
    );
    // Ohne Anmeldung: Umleitung zur Anmeldung.
    let mut anonymous = Client::new(&h);
    let r = anonymous.get(&format!("/aufgaben/anhang/{file_id}")).await;
    assert!(r.status.is_redirection(), "{}", r.status);
    // Die Karte zählt Bild und Dokument.
    assert!(ben.get("/aufgaben").await.body.contains("Dokumente"));
    // Fremde löschen nicht, der Vorstand schon.
    let mut greta = logged_in(&h, 107).await;
    let greta_token = csrf(&greta.get("/aufgaben").await.body);
    let denied = greta
        .post(
            &format!("/aufgaben/anhang/{file_id}/loeschen"),
            &[("csrf", &greta_token)],
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    let mut eva = logged_in(&h, 105).await;
    let eva_token = csrf(&eva.get("/aufgaben").await.body);
    let removed = eva
        .post(
            &format!("/aufgaben/anhang/{file_id}/loeschen"),
            &[("csrf", &eva_token)],
        )
        .await;
    assert_eq!(removed.status, StatusCode::SEE_OTHER);
    assert_eq!(
        ben.get(&format!("/aufgaben/anhang/{file_id}")).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn boat_image_lifecycle_and_rights() {
    use vereinsakte_core::example::{FILOU, HARALD_BLAUZAHN, TRIGON};
    let h = dev();
    let mut ben = logged_in(&h, 102).await;
    let page = ben.get(&format!("/boote/{HARALD_BLAUZAHN}")).await;
    assert!(page.body.contains("Bild hochladen"));
    let token = csrf(&page.body);
    let up = format!("/boote/{HARALD_BLAUZAHN}/bild/hochladen");
    let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];
    let ok = ben
        .upload(&up, &token, "boot.png", &png, "Harald unter Segel")
        .await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER, "{}", ok.body);
    // Liste und Bootsseite zeigen das Bild, die Trigon einen Platzhalter.
    let list = ben.get("/boote").await;
    assert!(list.body.contains("alt=\"Harald unter Segel\"") && list.body.contains("/bild?v="));
    assert!(list.body.contains("placeholder"));
    let page = ben.get(&format!("/boote/{HARALD_BLAUZAHN}")).await;
    assert!(page.body.contains("<img src=\"/boote/") && page.body.contains("Bild ändern"));
    let image = ben.get(&format!("/boote/{HARALD_BLAUZAHN}/bild")).await;
    assert_eq!(image.headers[header::CONTENT_TYPE], "image/png");
    assert_eq!(image.headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert!(
        image.headers[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap()
            .contains("sandbox")
    );
    assert_eq!(
        ben.get(&format!("/boote/{TRIGON}/bild")).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        ben.get("/boote/kaputt/bild").await.status,
        StatusCode::NOT_FOUND
    );
    // Ohne Anmeldung: Umleitung.
    let mut anonymous = Client::new(&h);
    assert!(
        anonymous
            .get(&format!("/boote/{HARALD_BLAUZAHN}/bild"))
            .await
            .status
            .is_redirection()
    );
    // Falsches Format und zu groß: Meldung, altes Bild bleibt.
    for (name, data) in [
        ("a.jpg", b"nur text".to_vec()),
        ("a.svg", b"<svg onload=x/>".to_vec()),
    ] {
        let r = ben.upload(&up, &token, name, &data, "").await;
        assert_eq!(r.status, StatusCode::SEE_OTHER);
        assert!(
            ben.get(&format!("/boote/{HARALD_BLAUZAHN}"))
                .await
                .body
                .contains("Nur JPEG, PNG, WebP oder GIF")
        );
    }
    let mut huge = vec![0xFF, 0xD8, 0xFF];
    huge.resize(8 * 1024 * 1024 + 10, 0);
    let big = ben.upload(&up, &token, "gross.jpg", &huge, "").await;
    assert_eq!(big.status, StatusCode::SEE_OTHER);
    assert!(
        ben.get(&format!("/boote/{HARALD_BLAUZAHN}"))
            .await
            .body
            .contains("zu groß")
    );
    assert_eq!(
        ben.get(&format!("/boote/{HARALD_BLAUZAHN}/bild"))
            .await
            .status,
        StatusCode::OK
    );
    // Ben ist nicht Bootswart der Filou, Greta hat kein Amt.
    let denied = ben
        .upload(
            &format!("/boote/{FILOU}/bild/hochladen"),
            &token,
            "x.png",
            &png,
            "",
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    let mut greta = logged_in(&h, 107).await;
    let gpage = greta.get(&format!("/boote/{HARALD_BLAUZAHN}")).await;
    assert!(!gpage.body.contains("Bild ändern") && !gpage.body.contains("Bild hochladen"));
    let gtoken = csrf(&gpage.body);
    assert_eq!(
        greta.upload(&up, &gtoken, "x.png", &png, "").await.status,
        StatusCode::FORBIDDEN
    );
    let remove = greta
        .post(
            &format!("/boote/{HARALD_BLAUZAHN}/bild/entfernen"),
            &[("csrf", &gtoken)],
        )
        .await;
    assert_eq!(remove.status, StatusCode::FORBIDDEN);
    // Entfernen durch Ben.
    let removed = ben
        .post(
            &format!("/boote/{HARALD_BLAUZAHN}/bild/entfernen"),
            &[("csrf", &token)],
        )
        .await;
    assert_eq!(removed.status, StatusCode::SEE_OTHER);
    assert_eq!(
        ben.get(&format!("/boote/{HARALD_BLAUZAHN}/bild"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn board_shows_wait_column_and_archive_tab_is_for_the_board() {
    let h = dev();
    let mut greta = logged_in(&h, 107).await;
    let board = greta.get("/aufgaben").await;
    assert!(board.body.contains("Segeltuch für die Fock bestellen") && board.body.contains("WAIT"));
    assert!(board.body.contains("data-status=\"waiting\""));
    assert!(board.body.contains("/static/board.js"));
    assert_eq!(
        greta.get("/protokoll/archiv?tab=aufgaben").await.status,
        StatusCode::FORBIDDEN
    );
    let mut eva = logged_in(&h, 105).await;
    let archive = eva.get("/protokoll/archiv?tab=aufgaben").await;
    assert_eq!(archive.status, StatusCode::OK);
    assert!(archive.body.contains("Keine erledigten Aufgaben"));
    assert_eq!(eva.get("/static/board.js").await.status, StatusCode::OK);
}

#[tokio::test]
async fn board_sees_deficits_transfers_and_compensates() {
    let h = dev();
    let mut eva = logged_in(&h, 105).await;
    let page = eva.get("/arbeit/uebersicht?rueckstand=1").await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(
        page.body.contains("es fehlen 4,5 Arbeitsstunden") && page.body.contains("Jensen, Clara")
    );
    let token = csrf(&page.body);
    let same = eva
        .post(
            "/arbeit/uebertrag",
            &[
                ("csrf", &token),
                ("year", "2026"),
                ("from", "103"),
                ("to", "103"),
                ("hours", "2"),
            ],
        )
        .await;
    assert_eq!(same.status, StatusCode::UNPROCESSABLE_ENTITY);
    let ok = eva
        .post(
            "/arbeit/uebertrag",
            &[
                ("csrf", &token),
                ("year", "2026"),
                ("from", "101"),
                ("to", "103"),
                ("hours", "5"),
                ("description", "Ausgleich"),
            ],
        )
        .await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER, "{}", ok.body);
    let page = eva.get("/arbeit/uebersicht?rueckstand=1").await;
    assert!(
        !page.body.contains("Jensen, Clara"),
        "Rückstand ausgeglichen"
    );
    let comp = eva
        .post(
            "/arbeit/abgeltung",
            &[
                ("csrf", &token),
                ("year", "2026"),
                ("member", "102"),
                ("comp_hours", "5"),
            ],
        )
        .await;
    assert_eq!(comp.status, StatusCode::SEE_OTHER, "{}", comp.body);
    assert!(eva.get("/arbeit/uebersicht").await.body.contains("75,00"));
    let mut ben = logged_in(&h, 102).await;
    assert!(
        ben.get("/meine-daten").await.body.contains("75,00"),
        "Forderung im Konto"
    );
    let mut greta = logged_in(&h, 107).await;
    assert_eq!(
        greta.get("/arbeit/uebersicht").await.status,
        StatusCode::FORBIDDEN
    );
}

// --- Gruppe 8: Reservierungen -------------------------------------------------------

fn reservation_fields<'a>(boat: &'a str, token: &'a str, day: &'a str) -> Vec<(&'a str, &'a str)> {
    vec![
        ("csrf", token),
        ("boat", boat),
        ("area", "home"),
        ("from_date", day),
        ("from_time", "10:00"),
        ("to_date", day),
        ("to_time", "16:00"),
    ]
}

fn main_of(html: &str) -> String {
    html[html.find("<main").unwrap()..].to_string()
}

#[tokio::test]
async fn calendar_shows_reservations_and_work_events_in_list_month_and_week() {
    use vereinsakte_core::example::FILOU;
    let h = dev();
    let mut david = logged_in(&h, 104).await;
    let token = csrf(&david.get("/reservierungen/neu").await.body);
    let boat = FILOU.to_string();
    let created = david
        .post(
            "/reservierungen/neu",
            &reservation_fields(&boat, &token, "2026-10-17"),
        )
        .await;
    assert_eq!(created.status, StatusCode::SEE_OTHER, "{}", created.body);

    // Liste: nach Woche und Tag gruppiert, beide Arten erkennbar unterschieden.
    let mut greta = logged_in(&h, 107).await;
    let list = greta.get("/kalender").await;
    assert_eq!(list.status, StatusCode::OK);
    let body = main_of(&list.body);
    assert!(body.contains("Nächste Woche"), "{body}");
    assert!(body.contains("Samstag, 17. Oktober"));
    assert!(body.contains("cal-entry reservation") && body.contains("Reservierung"));
    assert!(body.contains("cal-entry event") && body.contains("Arbeitstermin"));
    assert!(body.contains("Boote auskranen"));
    assert!(body.contains("David Schulz"));

    // Filter nach Art, Beteiligung und Boot
    let only_events = main_of(&greta.get("/kalender?art=termine").await.body);
    assert!(
        only_events.contains("cal-entry event") && !only_events.contains("cal-entry reservation")
    );
    let only_res = main_of(&greta.get("/kalender?art=reservierungen").await.body);
    assert!(only_res.contains("cal-entry reservation") && !only_res.contains("cal-entry event"));
    let mine = main_of(&david.get("/kalender?meine=1").await.body);
    assert!(mine.contains("cal-entry reservation") && !mine.contains("cal-entry event"));
    let other_boat = vereinsakte_core::example::HARALD_BLAUZAHN;
    let by_boat = main_of(
        &greta
            .get(&format!("/kalender?art=reservierungen&boot={other_boat}"))
            .await
            .body,
    );
    assert!(!by_boat.contains("cal-entry reservation"));

    // Monat und Woche: dieselben Einträge im Raster
    let month = greta.get("/kalender?ansicht=monat&monat=2026-10").await;
    let body = main_of(&month.body);
    assert!(body.contains("<table class=\"cal\"") && body.contains("Sa, 17.10."));
    assert!(body.contains("cal-entry reservation") && body.contains("cal-entry event"));
    assert!(body.contains("monat=2026-11") && body.contains("monat=2026-09"));
    let week = greta.get("/kalender?ansicht=woche&woche=2026-10-12").await;
    let body = main_of(&week.body);
    assert!(body.contains("Woche vom 12.10.") && body.contains("Sa, 17.10."));
    assert!(body.contains("cal-entry reservation") && body.contains("cal-entry event"));

    // Vergangenes steht nur im Archiv
    let past = main_of(
        &greta
            .get("/kalender?ansicht=monat&monat=2026-06")
            .await
            .body,
    );
    assert!(!past.contains("cal-entry"), "{past}");
    assert!(past.contains("Archiv"));

    // Die alte Reservierungsliste leitet in den Kalender um.
    let old = greta
        .get(&format!("/reservierungen?boot={boat}&meine=1"))
        .await;
    assert_eq!(old.status, StatusCode::SEE_OTHER);
    assert!(old.location().starts_with("/kalender?art=reservierungen"));
    assert!(old.location().contains("boot=") && old.location().contains("meine=1"));
}

#[tokio::test]
async fn eligible_member_reserves_and_sees_it_on_the_start_page() {
    use vereinsakte_core::example::FILOU;
    let h = dev();
    let mut david = logged_in(&h, 104).await;
    let form = david.get("/reservierungen/neu").await;
    assert_eq!(form.status, StatusCode::OK);
    let token = csrf(&form.body);
    let boat = FILOU.to_string();
    let created = david
        .post(
            "/reservierungen/neu",
            &reservation_fields(&boat, &token, "2026-10-17"),
        )
        .await;
    assert_eq!(created.status, StatusCode::SEE_OTHER, "{}", created.body);
    let page = david.get(created.location()).await;
    assert!(
        page.body.contains("Die Reservierung wurde angelegt.")
            && page.body.contains("David Schulz")
    );
    assert!(david.get("/").await.body.contains("Meine Reservierungen"));
    let overlap = david
        .post(
            "/reservierungen/neu",
            &reservation_fields(&boat, &token, "2026-10-17"),
        )
        .await;
    assert_eq!(overlap.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        overlap.body.contains("bereits reserviert")
            && overlap.body.contains("value=\"2026-10-17\"")
    );
}

#[tokio::test]
async fn ineligible_skipper_gets_clear_reasons_and_nothing_is_saved() {
    use vereinsakte_core::example::FILOU;
    let h = dev();
    let mut greta = logged_in(&h, 107).await;
    let token = csrf(&greta.get("/reservierungen/neu").await.body);
    let boat = FILOU.to_string();
    let response = greta
        .post(
            "/reservierungen/neu",
            &reservation_fields(&boat, &token, "2026-10-17"),
        )
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        response
            .body
            .contains("Die Reservierung ist so nicht möglich")
            && response.body.contains("Skipper nicht geeignet")
    );
    let list = greta.get("/kalender?art=reservierungen").await;
    assert!(!main_of(&list.body).contains("cal-entry reservation"));
    let mut foreign = reservation_fields(&boat, &token, "2026-10-17");
    foreign.push(("skipper", "104"));
    let denied = greta.post("/reservierungen/neu", &foreign).await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn reservations_can_be_changed_and_cancelled_by_their_skipper_only() {
    use vereinsakte_core::example::FILOU;
    let h = dev();
    let mut david = logged_in(&h, 104).await;
    let token = csrf(&david.get("/reservierungen/neu").await.body);
    let boat = FILOU.to_string();
    let created = david
        .post(
            "/reservierungen/neu",
            &reservation_fields(&boat, &token, "2026-10-17"),
        )
        .await;
    let id = created
        .location()
        .trim_start_matches("/reservierungen/")
        .to_string();
    let mut greta = logged_in(&h, 107).await;
    let page = greta.get(&format!("/reservierungen/{id}")).await;
    assert!(!page.body.contains("/stornieren") && !page.body.contains("/bearbeiten"));
    let denied = greta
        .post(
            &format!("/reservierungen/{id}/stornieren"),
            &[("csrf", &csrf(&page.body))],
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    let edit = david.get(&format!("/reservierungen/{id}/bearbeiten")).await;
    assert_eq!(edit.status, StatusCode::OK);
    let version = field_value(&edit.body, "version");
    let mut fields = reservation_fields(&boat, &token, "2026-10-17");
    fields[6] = ("to_time", "18:00");
    fields.push(("version", &version));
    let saved = david
        .post(&format!("/reservierungen/{id}/bearbeiten"), &fields)
        .await;
    assert_eq!(saved.status, StatusCode::SEE_OTHER, "{}", saved.body);
    let stale = david
        .post(&format!("/reservierungen/{id}/bearbeiten"), &fields)
        .await;
    assert_eq!(stale.status, StatusCode::CONFLICT);
    let cancelled = david
        .post(
            &format!("/reservierungen/{id}/stornieren"),
            &[("csrf", &token)],
        )
        .await;
    assert_eq!(cancelled.status, StatusCode::SEE_OTHER);
    assert!(
        !main_of(&david.get("/kalender?art=reservierungen").await.body)
            .contains("cal-entry reservation")
    );
}

#[tokio::test]
async fn tour_page_shows_the_expected_fee() {
    let h = dev();
    let id = vereinsakte_core::example::DEHLER_TOUR;
    let page = logged_in(&h, 107)
        .await
        .get(&format!("/reservierungen/{id}"))
        .await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(
        page.body.contains("Tourgebühr")
            && page.body.contains("Max Gast")
            && page.body.contains("Summe:")
    );
    assert!(
        page.body
            .contains("Die Forderungen legt der Kassenwart nach der Fahrt an.")
    );
}

// --- Gruppe 9: Finanzen --------------------------------------------------------------

#[tokio::test]
async fn accounts_are_private_and_the_finance_menu_depends_on_rights() {
    let h = dev();
    let anna = logged_in(&h, 101).await.get("/konto").await;
    assert!(anna.body.contains("Guthaben: 10,00") && anna.body.contains("Mein Konto"));
    let lukas = logged_in(&h, 112).await.get("/konto").await;
    assert!(lukas.body.contains("offen: 276,00"));
    let mut greta = logged_in(&h, 107).await;
    assert_eq!(
        greta.get("/finanzen/konten/103").await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(greta.get("/finanzen").await.status, StatusCode::FORBIDDEN);
    let own = greta.get("/konto").await;
    assert!(
        !own.body.contains("href=\"/finanzen\""),
        "kein Menüpunkt Finanzen"
    );
    assert!(!own.body.contains("Einzahlung buchen"));
    assert!(
        logged_in(&h, 105)
            .await
            .get("/konto")
            .await
            .body
            .contains("href=\"/finanzen\"")
    );
    // Revisor liest fremde Konten ohne Schaltflächen; Schreiben wird abgelehnt.
    let mut lukas = logged_in(&h, 112).await;
    let page = lukas.get("/finanzen/konten/103").await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(!page.body.contains("Einzahlung buchen") && !page.body.contains("/storno"));
    let denied = lukas
        .post(
            "/finanzen/konten/103/einzahlung",
            &[("csrf", &csrf(&page.body)), ("amount", "5")],
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn treasurer_posts_payments_claims_and_cancellations() {
    let h = dev();
    let mut jonas = logged_in(&h, 110).await;
    let page = jonas.get("/finanzen/konten/101").await;
    let token = csrf(&page.body);
    let ok = jonas
        .post(
            "/finanzen/konten/101/einzahlung",
            &[
                ("csrf", &token),
                ("amount", "240,00"),
                ("date", "2026-04-03"),
                ("text", "Beitrag"),
            ],
        )
        .await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER, "{}", ok.body);
    let page = jonas.get("/finanzen/konten/101").await;
    assert!(
        page.body.contains("Die Einzahlung wurde gebucht.")
            && page.body.contains("Guthaben: 250,00")
    );
    let negative = jonas
        .post(
            "/finanzen/konten/101/einzahlung",
            &[("csrf", &token), ("amount", "-5")],
        )
        .await;
    assert_eq!(negative.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(negative.body.contains("größer als 0"));
    let bad = jonas
        .post(
            "/finanzen/konten/101/einzahlung",
            &[("csrf", &token), ("amount", "viel")],
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    let claim = jonas
        .post(
            "/finanzen/konten/101/forderung",
            &[
                ("csrf", &token),
                ("claim_amount", "20"),
                ("claim_text", "Schlüsselpfand"),
            ],
        )
        .await;
    assert_eq!(claim.status, StatusCode::SEE_OTHER, "{}", claim.body);
    let page = jonas.get("/finanzen/konten/101").await;
    let id = id_after(&page.body, "Schlüsselpfand", "/forderungen/");
    let storno_path = format!("/finanzen/konten/101/forderungen/{id}/storno");
    let without = jonas
        .post(&storno_path, &[("csrf", &token), ("reason", "")])
        .await;
    assert_eq!(without.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(without.body.contains("Bitte eine Begründung angeben"));
    let ok = jonas
        .post(
            &storno_path,
            &[("csrf", &token), ("reason", "Schlüssel zurück")],
        )
        .await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER);
    let page = jonas.get("/finanzen/konten/101").await;
    assert!(page.body.contains("storniert (Schlüssel zurück)"));
    assert!(
        !page.body.contains(&format!("{storno_path}\"")),
        "Storno nur einmal"
    );
}

#[tokio::test]
async fn outstanding_amounts_are_sorted_and_filterable_by_url() {
    let h = dev();
    let mut eva = logged_in(&h, 105).await;
    let page = eva.get("/finanzen").await;
    assert!(page.body.contains("Meier, Lukas") || page.body.contains("Lukas Meier"));
    assert!(page.body.contains("276,00") && page.body.contains("Summe"));
    let overdue = eva
        .get("/finanzen?stichtag=2026-10-09&ueberfaellig=1")
        .await;
    assert!(overdue.body.contains("checked") && overdue.body.contains("value=\"2026-10-09\""));
}

#[tokio::test]
async fn season_rates_are_saved_and_validated() {
    let h = dev();
    let mut jonas = logged_in(&h, 110).await;
    let page = jonas.get("/finanzen/saisons?jahr=2027").await;
    assert!(page.body.contains("Saison 2027") && page.body.contains("name=\"annual_regular\""));
    let token = csrf(&page.body);
    let mut fields: Vec<(String, String)> = vec![
        ("csrf".into(), token.clone()),
        ("year".into(), "2027".into()),
    ];
    for name in ["hours_quota", "compensation_rate"] {
        fields.push((name.into(), input_value(&page.body, name)));
    }
    for key in page
        .body
        .split("name=\"")
        .skip(1)
        .map(|r| r[..r.find('"').unwrap()].to_string())
    {
        if ["annual_", "admission_", "tour_"]
            .iter()
            .any(|p| key.starts_with(p))
        {
            fields.push((key.clone(), input_value(&page.body, &key)));
        }
    }
    for f in fields.iter_mut().filter(|f| f.0 == "annual_regular") {
        f.1 = "260,00".into();
    }
    let refs: Vec<(&str, &str)> = fields
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let ok = jonas.post("/finanzen/saisons", &refs).await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER, "{}", ok.body);
    assert!(
        jonas
            .get("/finanzen/saisons?jahr=2027")
            .await
            .body
            .contains("value=\"260,00\"")
    );
    let mut broken = refs.clone();
    broken.push(("annual_student", "abc"));
    let bad = jonas.post("/finanzen/saisons", &broken).await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(bad.body.contains("Bitte einen Betrag wie 12,50 angeben."));
}

#[tokio::test]
async fn annual_fees_are_created_once() {
    let h = dev();
    let mut jonas = logged_in(&h, 110).await;
    let page = jonas.get("/finanzen/beitraege?jahr=2027").await;
    assert!(
        page.body.contains("Jahresbeitrag 2027") && page.body.contains("Jahresbeiträge anlegen")
    );
    let token = csrf(&page.body);
    let created = jonas
        .post("/finanzen/beitraege", &[("csrf", &token), ("year", "2027")])
        .await;
    assert_eq!(created.status, StatusCode::SEE_OTHER);
    let after = jonas.get("/finanzen/beitraege?jahr=2027").await;
    assert!(
        after.body.contains("Jahresbeiträge 2027 wurden angelegt")
            && after.body.contains("bereits vorhanden")
    );
    assert!(after.body.contains("Es fehlen keine Jahresbeiträge."));
    let again = jonas
        .post("/finanzen/beitraege", &[("csrf", &token), ("year", "2027")])
        .await;
    assert_eq!(again.status, StatusCode::SEE_OTHER);
    assert!(
        jonas
            .get("/finanzen/beitraege?jahr=2027")
            .await
            .body
            .contains("nichts anzulegen")
    );
    let mut greta = logged_in(&h, 107).await;
    let greta_page = greta.get("/konto").await;
    let denied = greta
        .post(
            "/finanzen/beitraege",
            &[("csrf", &csrf(&greta_page.body)), ("year", "2027")],
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tours_and_annual_proof_pages() {
    let h = dev();
    let mut jonas = logged_in(&h, 110).await;
    let tours = jonas.get("/finanzen/touren").await;
    assert_eq!(tours.status, StatusCode::OK);
    assert!(tours.body.contains("Schruvtrekker") && tours.body.contains("abgerechnet"));
    let proof = jonas.get("/finanzen/nachweis/103").await;
    assert_eq!(proof.status, StatusCode::OK);
    assert!(
        proof.body.contains("Jahresnachweis 2026")
            && proof.body.contains("20,5 angerechnet von 25")
    );
    assert!(proof.body.contains("4,5") && proof.body.contains("no-print"));
    let mut greta = logged_in(&h, 107).await;
    assert_eq!(
        greta.get("/finanzen/nachweis/103").await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        greta.get("/finanzen/touren").await.status,
        StatusCode::FORBIDDEN
    );
}

// --- Gruppe 10: Protokoll ------------------------------------------------------------

#[tokio::test]
async fn audit_page_is_filtered_by_role_and_never_shows_values() {
    let h = dev();
    let mut ben = logged_in(&h, 102).await;
    let harald = vereinsakte_core::example::HARALD_BLAUZAHN;
    let token = csrf(&ben.get(&format!("/boote/{harald}")).await.body);
    ben.post(
        &format!("/boote/{harald}/sperren"),
        &[("csrf", &token), ("reason", "Mast defekt")],
    )
    .await;
    let mut greta = logged_in(&h, 107).await;
    let greta_token = csrf(&greta.get("/mitglieder/107/bearbeiten").await.body);
    greta
        .post(
            "/mitglieder/107/scheine",
            &[("csrf", &greta_token), ("kind", "sailing"), ("name", "SKS")],
        )
        .await;
    let mut jonas = logged_in(&h, 110).await;
    let jonas_token = csrf(&jonas.get("/finanzen/konten/101").await.body);
    jonas
        .post(
            "/finanzen/konten/101/einzahlung",
            &[
                ("csrf", &jonas_token),
                ("amount", "12,50"),
                ("text", "geheim"),
            ],
        )
        .await;
    // Nur die Tabelle: Die Filterlisten nennen alle Aktionsarten.
    let main_of = |html: &str| {
        let main = &html[html.find("<main").unwrap()..];
        main[main.find("</form>").unwrap()..].to_string()
    };
    // Revisor (Lukas) sieht alles und kann filtern.
    let mut lukas = logged_in(&h, 112).await;
    let all = main_of(&lukas.get("/protokoll").await.body);
    assert!(
        all.contains("Boot gesperrt")
            && all.contains("Einzahlung gebucht")
            && all.contains("Schein oder Lizenz hinzugefügt")
    );
    let filtered = main_of(&lukas.get("/protokoll?art=payment.post").await.body);
    assert!(filtered.contains("Einzahlung gebucht") && !filtered.contains("Boot gesperrt"));
    assert!(
        !all.contains("12,50") && !all.contains("geheim"),
        "keine Werte im Protokoll"
    );
    let by_actor = main_of(&lukas.get("/protokoll?akteur=102").await.body);
    assert!(by_actor.contains("Boot gesperrt") && !by_actor.contains("Einzahlung gebucht"));
    // Greta sieht nur die eigenen Einträge.
    let own = main_of(&greta.get("/protokoll").await.body);
    assert!(
        own.contains("Schein oder Lizenz hinzugefügt")
            && !own.contains("Boot gesperrt")
            && !own.contains("Einzahlung gebucht")
    );
    assert!(
        !own.contains("name=\"akteur\""),
        "Filter nach Akteur nur mit Einsicht in alles"
    );
    // Navigation zeigt Protokoll allen Mitgliedern.
    assert!(greta.get("/").await.body.contains("href=\"/protokoll\""));
}

#[tokio::test]
async fn contact_notice_is_shown_once_until_confirmed() {
    let h = dev();
    let mut greta = logged_in(&h, 107).await;
    let page = greta.get("/").await;
    assert!(
        page.body
            .contains("jetzt für alle Vereinsmitglieder sichtbar")
    );
    assert!(page.body.contains("action=\"/hinweis/kontakt\""));
    // Anderes Mitglied sieht ihn unabhängig von Gretas Bestätigung.
    let token = csrf(&page.body);
    let response = greta.post("/hinweis/kontakt", &[("csrf", &token)]).await;
    assert_eq!(response.location(), "/");
    let page = greta.get("/").await;
    assert!(
        !page
            .body
            .contains("jetzt für alle Vereinsmitglieder sichtbar")
    );
    let mut anna = logged_in(&h, 101).await;
    assert!(
        anna.get("/")
            .await
            .body
            .contains("jetzt für alle Vereinsmitglieder sichtbar")
    );
    // Wer den Sperrvermerk bearbeitet, hat den Hinweis ebenfalls gesehen.
    set_contact_block(&h, 101, false).await;
    assert!(
        !anna
            .get("/")
            .await
            .body
            .contains("jetzt für alle Vereinsmitglieder sichtbar")
    );
}

fn crew_form<'a>(token: &'a str, extra: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    use vereinsakte_core::example::FILOU;
    let boat: &'static str = Box::leak(FILOU.to_string().into_boxed_str());
    let mut fields = vec![
        ("csrf", token),
        ("boat", boat),
        ("area", "home"),
        ("from_date", "2026-10-17"),
        ("from_time", "10:00"),
        ("to_date", "2026-10-17"),
        ("to_time", "16:00"),
    ];
    fields.extend_from_slice(extra);
    fields
}

#[tokio::test]
async fn reservation_form_has_no_tour_or_guest_fields_and_a_large_remark() {
    use vereinsakte_core::example::FILOU;
    let h = dev();
    let mut david = logged_in(&h, 104).await;
    let form = david
        .get(&format!("/reservierungen/neu?boot={FILOU}"))
        .await;
    assert_eq!(form.status, StatusCode::OK);
    let body = main_of(&form.body);
    assert!(!body.contains("name=\"tour\"") && !body.contains("name=\"guests\""));
    assert!(body.contains("<textarea id=\"f-remark\" name=\"remark\" class=\"large\""));
    assert!(
        body.contains(&format!("value=\"{FILOU}\" selected")),
        "Boot vorausgewählt"
    );
    assert!(body.contains("Heimrevier"));
    assert!(body.contains("Mitfahrer hinzufügen"));
}

#[tokio::test]
async fn crew_list_adds_removes_and_saves_members_with_profile_links() {
    let h = dev();
    let mut anna = logged_in(&h, 104).await;
    let token = csrf(&anna.get("/reservierungen/neu").await.body);

    // Hinzufügen ohne JavaScript: Seite wird neu gezeigt, nichts gespeichert.
    let added = anna
        .post(
            "/reservierungen/neu",
            &crew_form(&token, &[("action", "crew_add"), ("add_member", "103")]),
        )
        .await;
    assert_eq!(added.status, StatusCode::OK, "{}", added.body);
    let body = main_of(&added.body);
    assert!(body.contains("href=\"/mitglieder/103\""), "Crew als Link");
    assert!(body.contains("name=\"crew_member\" value=\"103\""));
    assert!(body.contains("value=\"2026-10-17\""), "Eingaben bleiben");
    assert!(
        !main_of(&anna.get("/kalender?art=reservierungen").await.body)
            .contains("cal-entry reservation"),
        "Hinzufügen speichert nicht"
    );

    // Zweites Mitglied, dann doppelt, dann entfernen
    let two = anna
        .post(
            "/reservierungen/neu",
            &crew_form(
                &token,
                &[
                    ("crew_member", "103"),
                    ("crew_from", ""),
                    ("crew_to", ""),
                    ("action", "crew_add"),
                    ("add_member", "102"),
                ],
            ),
        )
        .await;
    let body = main_of(&two.body);
    assert!(body.contains("value=\"103\"") && body.contains("name=\"crew_member\" value=\"102\""));
    let myself = anna
        .post(
            "/reservierungen/neu",
            &crew_form(&token, &[("action", "crew_add"), ("add_member", "104")]),
        )
        .await;
    assert!(
        main_of(&myself.body).contains("bereits an Bord"),
        "der Skipper selbst"
    );
    let duplicate = anna
        .post(
            "/reservierungen/neu",
            &crew_form(
                &token,
                &[
                    ("crew_member", "103"),
                    ("action", "crew_add"),
                    ("add_member", "103"),
                ],
            ),
        )
        .await;
    assert_eq!(duplicate.status, StatusCode::OK);
    assert!(main_of(&duplicate.body).contains("bereits an Bord"));
    let removed = anna
        .post(
            "/reservierungen/neu",
            &crew_form(
                &token,
                &[
                    ("crew_member", "103"),
                    ("crew_member", "102"),
                    ("action", "crew_remove:1"),
                ],
            ),
        )
        .await;
    let body = main_of(&removed.body);
    assert!(body.contains("name=\"crew_member\" value=\"103\""));
    assert!(!body.contains("name=\"crew_member\" value=\"102\""));

    // Speichern mit Crew und Gästen in der Bemerkung; Boot ist die Filou, Anna darf sie führen?
    let save = anna
        .post(
            "/reservierungen/neu",
            &crew_form(
                &token,
                &[
                    ("crew_member", "102"),
                    ("crew_from", ""),
                    ("crew_to", ""),
                    ("remark", "Gast: Max Gast, 0160 123"),
                    ("action", "speichern"),
                ],
            ),
        )
        .await;
    assert_eq!(save.status, StatusCode::SEE_OTHER, "{}", save.body);
    let page = anna.get(save.location()).await;
    let body = main_of(&page.body);
    assert!(body.contains("href=\"/mitglieder/102\"") && body.contains("Ben Hansen"));
    assert!(body.contains("Gast: Max Gast, 0160 123"));
}

#[tokio::test]
async fn tour_follows_the_duration_of_the_reservation() {
    use vereinsakte_core::example::FILOU;
    let h = dev();
    let mut david = logged_in(&h, 104).await;
    let token = csrf(&david.get("/reservierungen/neu").await.body);
    let boat = FILOU.to_string();
    let mut fields = reservation_fields(&boat, &token, "2026-10-17");
    let one_day = david.post("/reservierungen/neu", &fields).await;
    let page = david.get(one_day.location()).await;
    assert!(
        main_of(&page.body).contains("Ausfahrt") && !main_of(&page.body).contains("Tourgebühr")
    );
    // Mehrtägig: Tour ohne Haken im Formular
    fields[5] = ("to_date", "2026-10-20");
    fields[2] = ("area", "home");
    let mut shifted = fields.clone();
    shifted[3] = ("from_date", "2026-10-25");
    shifted[5] = ("to_date", "2026-10-28");
    let tour = david.post("/reservierungen/neu", &shifted).await;
    assert_eq!(tour.status, StatusCode::SEE_OTHER, "{}", tour.body);
    let page = david.get(tour.location()).await;
    assert!(main_of(&page.body).contains("Tour"));
}

#[tokio::test]
async fn archive_collects_what_is_no_longer_current_for_board_and_auditors_only() {
    let h = dev();
    // Ben gibt sein Amt ab: weg aus Ämterübersicht und Mitgliedsseite, rein ins Archiv.
    let mut ben = logged_in(&h, 102).await;
    let page = ben.get("/aemter").await;
    let action = {
        let at = page.body.find("/mitglieder/102\">Ben Hansen").unwrap();
        let rest = &page.body[at..];
        let start = rest.find("action=\"/aemter/").unwrap() + "action=\"".len();
        rest[start..start + rest[start..].find('"').unwrap()].to_string()
    };
    let done = ben
        .post(
            &action,
            &[("csrf", &csrf(&page.body)), ("date", "09.10.2026")],
        )
        .await;
    assert_eq!(done.status, StatusCode::SEE_OTHER);
    let overview = main_of(&ben.get("/aemter").await.body);
    assert!(!overview.contains("/mitglieder/102\">Ben Hansen"));
    let member = main_of(&ben.get("/mitglieder/102").await.body);
    assert!(
        !member.contains("Bootswart (Harald Blauzahn)"),
        "nicht mehr bei Ben"
    );
    let list = main_of(&ben.get("/mitglieder").await.body);
    assert!(!list.contains("Bootswart (Harald Blauzahn)"));

    // Archiv: Vorstand (Eva) und Revisor (Lukas) sehen es, Greta und Ben nicht.
    let mut eva = logged_in(&h, 105).await;
    let archive = eva.get("/protokoll/archiv?tab=aemter").await;
    assert_eq!(archive.status, StatusCode::OK);
    assert!(
        main_of(&archive.body).contains("Ben Hansen")
            && main_of(&archive.body).contains("09.10.2026")
    );
    assert_eq!(
        logged_in(&h, 112)
            .await
            .get("/protokoll/archiv")
            .await
            .status,
        StatusCode::OK
    );
    for member in [107, 102] {
        assert_eq!(
            logged_in(&h, member)
                .await
                .get("/protokoll/archiv")
                .await
                .status,
            StatusCode::FORBIDDEN
        );
    }
    // Reiter: ausgeschiedene Mitglieder, vergangene Reservierungen und Termine mit Jahresfilter
    let members = main_of(&eva.get("/protokoll/archiv?tab=mitglieder").await.body);
    assert!(members.contains("Dupont") || members.contains("Keine ausgeschiedenen"));
    let past = main_of(
        &eva.get("/protokoll/archiv?tab=reservierungen&jahr=2026")
            .await
            .body,
    );
    assert!(past.contains("Filou") && past.contains("Schruvtrekker"));
    let none = main_of(
        &eva.get("/protokoll/archiv?tab=reservierungen&jahr=2020")
            .await
            .body,
    );
    assert!(none.contains("Keine vergangenen Reservierungen"));
    let sorted = main_of(
        &eva.get("/protokoll/archiv?tab=reservierungen&sort_arch=boot&dir_arch=desc")
            .await
            .body,
    );
    assert!(sorted.contains("aria-sort=\"descending\""));
    // Das Protokoll verweist auf das Archiv (nur Berechtigte).
    assert!(
        eva.get("/protokoll")
            .await
            .body
            .contains("/protokoll/archiv")
    );
    assert!(
        !logged_in(&h, 107)
            .await
            .get("/protokoll")
            .await
            .body
            .contains("/protokoll/archiv")
    );
    // Stornierte Forderungen bleiben im Konto sichtbar (keine Archivdaten).
    let account = main_of(
        &logged_in(&h, 110)
            .await
            .get("/finanzen/konten/102")
            .await
            .body,
    );
    assert!(account.contains("Schlüsselpfand") && account.contains("storniert"));
}
