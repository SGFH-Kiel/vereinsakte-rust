//! Logging der Anfragen (eigener Testprozess, damit kein paralleler Test den
//! Zwischenspeicher von `tracing` beeinflusst).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;
use vereinsakte_app::ClubService;
use vereinsakte_app::auth::AuthService;
use vereinsakte_app::memory::{MemoryIdentityRepository, MemoryStore};
use vereinsakte_app::ports::FixedClock;
use vereinsakte_core::example;
use vereinsakte_core::time::club_time;
use vereinsakte_web::{AppState, Settings, router};

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for LogBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn requests_are_logged_with_method_path_and_status() {
    let buffer = LogBuffer::default();
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let store = Arc::new(MemoryStore::new(example::club()));
    let clock = Arc::new(FixedClock(club_time(2026, 10, 9, 12, 0)));
    let state = AppState {
        club: ClubService::new(store.clone(), clock.clone()),
        auth: AuthService::new(store, Arc::new(MemoryIdentityRepository::default()), clock),
        settings: Arc::new(Settings {
            development: true,
            base_url: "http://localhost:8080".into(),
            session_idle: Duration::from_secs(3600),
            tenant_id: None,
        }),
        login: None,
    };
    let app = router(state, tower_sessions::MemoryStore::default());
    let response = app
        .oneshot(
            Request::get("/health?code=geheim")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    let log = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    assert!(log.contains("method=GET"), "{log}");
    assert!(log.contains("path=/health"), "{log}");
    assert!(log.contains("status=200"), "{log}");
    assert!(!log.contains("geheim"), "keine Query im Log: {log}");
}
