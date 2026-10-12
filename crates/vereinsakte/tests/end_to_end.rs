//! Ende-zu-Ende: temporäre Datenbank migrieren, Beispielakte importieren,
//! Router mit SQLite-Ports im Entwicklungsmodus, Anmeldung als Anna,
//! Seiten abrufen, abmelden.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use tower::ServiceExt;
use vereinsakte::cli;
use vereinsakte::config::{Config, Mode};
use vereinsakte::server::build_router;
use vereinsakte_core::example;
use vereinsakte_storage::Database;

struct Client {
    app: Router,
    cookie: Option<String>,
}

struct Reply {
    status: StatusCode,
    location: String,
    body: String,
}

impl Client {
    async fn send(&mut self, mut request: Request<Body>) -> Reply {
        if let Some(cookie) = &self.cookie {
            request
                .headers_mut()
                .insert(header::COOKIE, cookie.parse().unwrap());
        }
        let response = self.app.clone().oneshot(request).await.unwrap();
        if let Some(set) = response.headers().get(header::SET_COOKIE) {
            let set = set.to_str().unwrap();
            self.cookie = if set.contains("Max-Age=0") {
                None
            } else {
                Some(set.split(';').next().unwrap().to_owned())
            };
        }
        let status = response.status();
        let location = response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        Reply {
            status,
            location,
            body: String::from_utf8_lossy(&bytes).into_owned(),
        }
    }

    async fn get(&mut self, path: &str) -> Reply {
        self.send(Request::get(path).body(Body::empty()).unwrap())
            .await
    }

    async fn post(&mut self, path: &str, body: String) -> Reply {
        let request = Request::post(path)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap();
        self.send(request).await
    }
}

fn csrf(html: &str) -> String {
    let marker = "name=\"csrf\" value=\"";
    let start = html.find(marker).expect("CSRF-Feld") + marker.len();
    html[start..start + html[start..].find('"').unwrap()].to_owned()
}

#[tokio::test]
async fn anna_logs_in_reads_pages_and_logs_out() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("akte.db");
    let source = dir.path().join("akte.json");
    std::fs::write(&source, serde_json::to_string(&example::club()).unwrap()).unwrap();
    cli::migrate(&path).await.unwrap();
    cli::import(&path, &source, false).await.unwrap();

    let config = Config {
        mode: Mode::Development,
        database: path.clone(),
        ..Config::default()
    };
    let db = Database::connect(&path).await.unwrap();
    let app = build_router(&config, db).await.unwrap();
    let mut client = Client { app, cookie: None };

    // Ohne Anmeldung: Weiterleitung zur Anmeldung.
    let reply = client.get("/meine-daten").await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert!(reply.location.starts_with("/login"), "{}", reply.location);

    // Entwicklungs-Login als Anna (101).
    let page = client.get("/dev/login?next=/meine-daten").await;
    assert_eq!(page.status, StatusCode::OK);
    let reply = client
        .post(
            "/dev/switch",
            format!("csrf={}&member=101&next=/meine-daten", csrf(&page.body)),
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);

    let reply = client.get("/meine-daten").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.body.contains("101"), "{}", reply.body);
    assert!(reply.body.contains("SKS"));
    assert!(reply.body.contains("10,00"));

    let reply = client.get("/mitglieder?q=peter").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.body.contains("Petersen"));
    assert!(reply.body.contains("Hannes") || reply.body.contains("Peter"));

    // Abmelden.
    let page = client.get("/meine-daten").await;
    let reply = client
        .post("/logout", format!("csrf={}", csrf(&page.body)))
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    let reply = client.get("/meine-daten").await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert!(reply.location.starts_with("/login"));
}

async fn sqlite_app() -> (tempfile::TempDir, std::path::PathBuf, Config) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("akte.db");
    let source = dir.path().join("akte.json");
    std::fs::write(&source, serde_json::to_string(&example::club()).unwrap()).unwrap();
    cli::migrate(&path).await.unwrap();
    cli::import(&path, &source, false).await.unwrap();
    let config = Config {
        mode: Mode::Development,
        database: path.clone(),
        ..Config::default()
    };
    (dir, path, config)
}

async fn login(app: &Router, member: u32) -> Client {
    let mut client = Client {
        app: app.clone(),
        cookie: None,
    };
    let page = client.get("/dev/login?next=/").await;
    let reply = client
        .post(
            "/dev/switch",
            format!("csrf={}&member={member}&next=/", csrf(&page.body)),
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    client
}

async fn token(client: &mut Client) -> String {
    csrf(&client.get("/").await.body)
}

#[tokio::test]
async fn every_page_works_on_sqlite_for_every_member() {
    use vereinsakte_core::example::{DEHLER_TOUR, FILOU, SEGELTUCH};
    let (_dir, path, config) = sqlite_app().await;
    let app = build_router(&config, Database::connect(&path).await.unwrap())
        .await
        .unwrap();
    let paths = [
        "/".to_string(),
        "/meine-daten".into(),
        "/meine-daten/bearbeiten".into(),
        "/mitglieder".into(),
        "/mitglieder/103".into(),
        "/mitglieder/103/bearbeiten".into(),
        "/mitglieder/neu".into(),
        "/boote".into(),
        format!("/boote/{FILOU}"),
        format!("/boote/{FILOU}/bearbeiten"),
        "/reservierungen".into(),
        "/reservierungen/neu".into(),
        format!("/reservierungen/{DEHLER_TOUR}"),
        "/arbeit".into(),
        "/arbeit/termine".into(),
        "/aufgaben".into(),
        "/aufgaben?meine=1".into(),
        "/aufgaben/neu".into(),
        format!("/aufgaben/{SEGELTUCH}"),
        "/protokoll/archiv?tab=aufgaben".into(),
        "/arbeit/bestaetigungen".into(),
        "/arbeit/uebersicht".into(),
        "/aemter".into(),
        "/aemter/historie".into(),
        "/konto".into(),
        "/finanzen".into(),
        "/finanzen/konten/103".into(),
        "/finanzen/saisons".into(),
        "/finanzen/beitraege".into(),
        "/finanzen/touren".into(),
        "/finanzen/nachweis/103".into(),
        "/protokoll".into(),
    ];
    for member in 101..=113u32 {
        let mut client = login(&app, member).await;
        for path in &paths {
            let reply = client.get(path).await;
            assert!(
                matches!(
                    reply.status,
                    StatusCode::OK | StatusCode::FORBIDDEN | StatusCode::SEE_OTHER
                ),
                "Mitglied {member}, {path}: {}",
                reply.status
            );
        }
    }
}

#[tokio::test]
async fn roles_work_together_on_sqlite_and_changes_survive_a_restart() {
    use vereinsakte_core::example::HARALD_BLAUZAHN;
    let (_dir, path, config) = sqlite_app().await;
    let app = build_router(&config, Database::connect(&path).await.unwrap())
        .await
        .unwrap();
    let boat = HARALD_BLAUZAHN.to_string();

    // Greta erfasst einen Einsatz, Ben (Bootswart) bestätigt ihn.
    let mut greta = login(&app, 107).await;
    let t = token(&mut greta).await;
    let added = greta
        .post(
            "/arbeit/einsaetze",
            format!("csrf={t}&date=2026-10-01&hours=3&subject={boat}&description=Antifouling"),
        )
        .await;
    assert_eq!(added.status, StatusCode::SEE_OTHER, "{}", added.body);
    let mut ben = login(&app, 102).await;
    let page = ben.get("/arbeit/bestaetigungen").await;
    assert!(page.body.contains("Greta Lorenzen"));
    let marker = "action=\"/arbeit/einsaetze/";
    let from = page.body.find(marker).unwrap() + marker.len() - "/arbeit/einsaetze/".len();
    let end = page.body[from..].find('"').unwrap();
    let action = page.body[from..from + end].to_string();
    let confirmed = ben
        .post(&action, format!("csrf={}", csrf(&page.body)))
        .await;
    assert_eq!(
        confirmed.status,
        StatusCode::SEE_OTHER,
        "{}",
        confirmed.body
    );
    assert!(greta.get("/arbeit").await.body.contains("bestätigt"));

    // Eva nimmt ein Mitglied auf, die Kassenwartin bucht eine Einzahlung.
    let mut eva = login(&app, 105).await;
    let t = token(&mut eva).await;
    let created = eva
        .post(
            "/mitglieder/neu",
            format!("csrf={t}&first_name=Nora&last_name=Neu&street=Hafenstra%C3%9Fe+1&postal_code=24103&city=Kiel&membership_type=regular"),
        )
        .await;
    assert_eq!(created.status, StatusCode::SEE_OTHER, "{}", created.body);
    assert_eq!(created.location, "/mitglieder/114");
    let mut jonas = login(&app, 110).await;
    let t = token(&mut jonas).await;
    let paid = jonas
        .post(
            "/finanzen/konten/101/einzahlung",
            format!("csrf={t}&amount=240%2C00&text=Beitrag"),
        )
        .await;
    assert_eq!(paid.status, StatusCode::SEE_OTHER, "{}", paid.body);

    // Der Revisor liest alles, scheitert aber beim Schreiben.
    let mut lukas = login(&app, 112).await;
    let t = token(&mut lukas).await;
    let account = lukas.get("/finanzen/konten/101").await;
    assert!(account.body.contains("Guthaben: 250,00"));
    let denied = lukas
        .post(
            "/finanzen/konten/101/einzahlung",
            format!("csrf={t}&amount=5"),
        )
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    let log = lukas.get("/protokoll").await.body;
    for text in [
        "Arbeitseinsatz erfasst",
        "Arbeitseinsatz bestätigt",
        "Mitglied aufgenommen",
        "Einzahlung gebucht",
    ] {
        assert!(log.contains(text), "Protokoll enthält {text}");
    }
    assert!(!log.contains("240"), "keine Beträge im Protokoll");

    // Nach einem Neustart (neuer Router auf derselben Datenbank) ist alles noch da.
    let restarted = build_router(&config, Database::connect(&path).await.unwrap())
        .await
        .unwrap();
    let mut eva = login(&restarted, 105).await;
    assert!(eva.get("/mitglieder/114").await.body.contains("Nora Neu"));
    let mut anna = login(&restarted, 101).await;
    assert!(anna.get("/konto").await.body.contains("Guthaben: 250,00"));
}
