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
