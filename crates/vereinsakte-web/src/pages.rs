//! Layout und Seiten: „Meine Daten“, Mitgliederverzeichnis, Health-Check und
//! statische Dateien.

use askama::Template;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use tower_sessions::Session;
use vereinsakte_app::directory::{DirectoryFilter, DirectoryView};
use vereinsakte_app::members::{membership_types, parse_membership_type};
use vereinsakte_app::my_data::MyDataView;
use vereinsakte_app::service::LoginChoiceGroup;
use vereinsakte_app::{CurrentUser, Principal};

use crate::error::WebError;
use crate::list::{Sort, SortCtx, SortKey, date_key, num_key, sort_rows, text_key};
use crate::session::{FlashKind, RequireUser, csrf_token, take_flash};
use crate::state::AppState;

/// Gemeinsame Angaben des Basislayouts.
pub struct Layout {
    pub title: String,
    pub csrf: String,
    pub user: Option<UserBar>,
    pub dev_bar: Option<DevBar>,
    pub flash: Option<FlashView>,
    /// Formularseite: blendet am Handy die untere Leiste aus.
    pub has_form: bool,
}

pub struct UserBar {
    pub name: String,
    pub member: u32,
    pub initials: String,
    pub avatar: String,
    pub offices: Vec<String>,
    pub groups: Vec<NavGroup>,
    /// Schnellzugriff der unteren Leiste (ohne „Mehr“).
    pub tabs: Vec<NavItem>,
}

/// Gruppe der Hauptnavigation.
pub struct NavGroup {
    pub title: &'static str,
    pub items: Vec<NavItem>,
}

/// Eintrag der Hauptnavigation.
#[derive(Clone)]
pub struct NavItem {
    pub href: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub current: bool,
}

pub struct FlashView {
    pub class: &'static str,
    pub icon: &'static str,
    pub text: String,
}

/// Leiste für den schnellen Benutzerwechsel im Entwicklungsmodus.
pub struct DevBar {
    pub current: u32,
    pub current_name: String,
    pub next: String,
    pub groups: Vec<LoginChoiceGroup>,
}

/// Initialen (Vor- und Nachname) für den Namenskreis.
pub fn initials(name: &str) -> String {
    let mut parts = name.split_whitespace();
    let first = parts.next().and_then(|p| p.chars().next());
    let last = parts.next_back().and_then(|p| p.chars().next());
    first
        .into_iter()
        .chain(last)
        .flat_map(char::to_uppercase)
        .collect()
}

/// Farbklasse des Namenskreises, stabil je Mitglied.
pub fn avatar_class(member: u32) -> String {
    format!("av-{}", member % 6)
}

/// Hauptnavigation nach Berechtigung in Gruppen; `path` bestimmt den aktuellen
/// Bereich (der Eintrag mit dem längsten passenden Adressanfang).
fn navigation(principal: &Principal, path: &str) -> (Vec<NavGroup>, Vec<NavItem>) {
    let path = path.split('?').next().unwrap_or(path);
    let mut groups: Vec<(
        &'static str,
        Vec<(&'static str, &'static str, &'static str)>,
    )> = vec![
        (
            "Persönlich",
            vec![
                ("/", "Start", "home"),
                ("/meine-daten", "Meine Daten", "user"),
            ],
        ),
        (
            "Verein",
            vec![
                ("/mitglieder", "Mitglieder", "users"),
                ("/aemter", "Ämter", "award"),
                ("/boote", "Boote", "boat"),
                ("/kalender", "Kalender", "calendar"),
                ("/arbeit/aufgaben", "Aufgaben", "check-square"),
                ("/arbeit", "Arbeit", "wrench"),
            ],
        ),
        ("Verwaltung", vec![("/protokoll", "Protokoll", "clipboard")]),
    ];
    if principal.can(&vereinsakte_core::rights::Capability::ReadAllAccounts) {
        groups[2].1.insert(0, ("/finanzen", "Finanzen", "euro"));
    }
    let matches = |href: &str| {
        if href == "/" {
            path == "/"
        } else {
            path == href || path.starts_with(&format!("{href}/"))
        }
    };
    let best = groups
        .iter()
        .flat_map(|(_, items)| items.iter())
        .filter(|(href, _, _)| matches(href))
        .map(|(href, _, _)| *href)
        .max_by_key(|href| href.len());
    let groups: Vec<NavGroup> = groups
        .into_iter()
        .map(|(title, items)| NavGroup {
            title,
            items: items
                .into_iter()
                .map(|(href, label, icon)| NavItem {
                    href,
                    label,
                    icon,
                    current: Some(href) == best,
                })
                .collect(),
        })
        .collect();
    let tabs = ["/", "/boote", "/kalender", "/arbeit"]
        .into_iter()
        .filter_map(|href| {
            groups
                .iter()
                .flat_map(|g| g.items.iter())
                .find(|i| i.href == href)
                .cloned()
        })
        .collect();
    (groups, tabs)
}

impl Layout {
    pub async fn build(
        state: &AppState,
        session: &Session,
        user: Option<&CurrentUser>,
        title: &str,
        next: &str,
    ) -> Result<Self, WebError> {
        let csrf = csrf_token(session).await;
        let (user_bar, dev_bar) = match user {
            Some(user) => {
                let principal = state.club.principal(user).await?;
                let dev_bar = if state.settings.development {
                    Some(DevBar {
                        current: user.member.get(),
                        current_name: principal.name.clone(),
                        next: next.to_string(),
                        groups: state.club.login_choices().await?,
                    })
                } else {
                    None
                };
                let (groups, tabs) = navigation(&principal, next);
                (
                    Some(UserBar {
                        initials: initials(&principal.name),
                        avatar: avatar_class(user.member.get()),
                        member: user.member.get(),
                        name: principal.name,
                        offices: principal.offices,
                        groups,
                        tabs,
                    }),
                    dev_bar,
                )
            }
            None => (None, None),
        };
        let flash = take_flash(session).await.map(|f| {
            let (class, icon) = match f.kind {
                FlashKind::Ok => ("ok", "check-circle"),
                FlashKind::Warn => ("warn", "alert"),
                FlashKind::Error => ("error", "alert-circle"),
            };
            FlashView {
                class,
                icon,
                text: f.text,
            }
        });
        Ok(Self {
            title: title.into(),
            csrf,
            user: user_bar,
            dev_bar,
            flash,
            has_form: false,
        })
    }

    /// Markiert die Seite als Formularseite.
    pub fn form(mut self) -> Self {
        self.has_form = true;
        self
    }
}

pub fn render(template: &impl Template) -> Result<Response, WebError> {
    Ok(Html(template.render()?).into_response())
}

/// Sortierung der drei Tabellen einer Mitgliedsansicht (Stunden, Forderungen, Einzahlungen).
pub struct DetailSorts {
    pub work: SortCtx,
    pub claims: SortCtx,
    pub payments: SortCtx,
}

/// Sortiert die Tabellen von `v` nach den Parametern der Adresse und liefert die Köpfe.
pub fn sort_details(
    v: &mut MyDataView,
    params: &std::collections::HashMap<String, String>,
    base: &str,
) -> DetailSorts {
    use vereinsakte_app::my_data::{ClaimLine, PaymentLine, WorkLine};
    let work = Sort::from_params(params, "std");
    let claims = Sort::from_params(params, "ford");
    let payments = Sort::from_params(params, "einz");
    sort_rows::<WorkLine>(
        &mut v.work,
        &work,
        &[
            ("datum", |l| date_key(&l.date)),
            ("stunden", |l| num_key(&l.hours)),
            ("gegenstand", |l| text_key(&l.subject)),
            ("status", |l| SortKey::Num(i64::from(l.confirmed))),
        ],
    );
    sort_rows::<ClaimLine>(
        &mut v.claims,
        &claims,
        &[
            ("forderung", |l| text_key(&l.text)),
            ("betrag", |l| num_key(&l.amount)),
            ("faellig", |l| date_key(l.due.as_deref().unwrap_or(""))),
            ("status", |l| text_key(&l.state.label())),
        ],
    );
    sort_rows::<PaymentLine>(
        &mut v.payments,
        &payments,
        &[
            ("datum", |l| date_key(&l.date)),
            ("betrag", |l| num_key(&l.amount)),
            ("zweck", |l| text_key(&l.text)),
        ],
    );
    let others = |own: &str| -> Vec<(String, String)> {
        let mut out = vec![];
        for (id, sort) in [("std", &work), ("ford", &claims), ("einz", &payments)] {
            if id != own && !sort.key.is_empty() {
                out.push((format!("sort_{id}"), sort.key.clone()));
                if sort.desc {
                    out.push((format!("dir_{id}"), "desc".to_string()));
                }
            }
        }
        out
    };
    DetailSorts {
        work: SortCtx::new(
            "std",
            base,
            others("std"),
            &work,
            &[
                ("datum", "Datum"),
                ("stunden", "Stunden"),
                ("gegenstand", "Gegenstand"),
                ("status", "Status"),
            ],
        ),
        claims: SortCtx::new(
            "ford",
            base,
            others("ford"),
            &claims,
            &[
                ("forderung", "Forderung"),
                ("betrag", "Betrag"),
                ("faellig", "Fälligkeit"),
                ("status", "Stand"),
            ],
        ),
        payments: SortCtx::new(
            "einz",
            base,
            others("einz"),
            &payments,
            &[
                ("datum", "Datum"),
                ("betrag", "Betrag"),
                ("zweck", "Verwendungszweck"),
            ],
        ),
    }
}

/// Handlungshinweis der Startseite („Was ist zu tun?“).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TodoItem {
    pub icon: &'static str,
    pub text: String,
    pub href: &'static str,
    pub label: &'static str,
}

/// Dringende Hinweise nach fester Reihenfolge: Segelerlaubnis, Bestätigungen,
/// Arbeitsstunden, Konto, Aufgaben. Leer, wenn nichts zu tun ist.
pub fn todo_items(v: &MyDataView, confirmations: usize, open_tasks: usize) -> Vec<TodoItem> {
    let mut items = vec![];
    if !v.sailing_permit.granted {
        items.push(TodoItem {
            icon: "alert",
            text: format!(
                "Dir fehlt die Segelerlaubnis {}: {}",
                v.year,
                v.sailing_permit.reasons.join("; ")
            ),
            href: "/meine-daten",
            label: "Details",
        });
    }
    if confirmations > 0 {
        items.push(TodoItem {
            icon: "check-square",
            text: if confirmations == 1 {
                "1 Arbeitseinsatz wartet auf deine Bestätigung".into()
            } else {
                format!("{confirmations} Arbeitseinsätze warten auf deine Bestätigung")
            },
            href: "/arbeit/bestaetigungen",
            label: "Jetzt prüfen",
        });
    }
    if !v.missing_hours.is_empty() {
        items.push(TodoItem {
            icon: "clock",
            text: format!("Es fehlen {} Arbeitsstunden", v.missing_hours),
            href: "/arbeit",
            label: "Einsatz erfassen",
        });
    }
    if let vereinsakte_app::my_data::Balance::Due(amount) = &v.balance {
        items.push(TodoItem {
            icon: "euro",
            text: format!("Auf deinem Konto sind {amount} offen"),
            href: "/konto",
            label: "Zum Konto",
        });
    }
    if open_tasks > 0 {
        items.push(TodoItem {
            icon: "check-square",
            text: if open_tasks == 1 {
                "1 offene Aufgabe wartet auf dich".into()
            } else {
                format!("{open_tasks} offene Aufgaben warten auf dich")
            },
            href: "/arbeit/aufgaben",
            label: "Zu den Aufgaben",
        });
    }
    items
}

#[derive(Template)]
#[template(path = "start.html")]
struct StartPage {
    layout: Layout,
    v: MyDataView,
    todo: Vec<TodoItem>,
    events: Vec<vereinsakte_app::work::EventLine>,
    tasks: Vec<vereinsakte_app::work::TaskLine>,
    reservations: Vec<vereinsakte_app::reservations::ReservationRow>,
}

/// Startseite: die nächsten Schritte des angemeldeten Mitglieds.
pub async fn home(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
) -> Result<Response, WebError> {
    let v = state.club.my_data(&user).await?;
    let principal = state.club.principal(&user).await?;
    let confirmations = if principal.has_office() {
        state.club.open_confirmations(&user).await?.len()
    } else {
        0
    };
    let events: Vec<_> = state
        .club
        .events_view(&user)
        .await?
        .events
        .into_iter()
        .filter(|e| !e.past)
        .take(3)
        .collect();
    let tasks: Vec<_> = state
        .club
        .tasks_view(&user)
        .await?
        .tasks
        .into_iter()
        .filter(|t| t.status_key != "done" && t.can_update && !t.assignees.is_empty())
        .filter(|t| t.assignees.contains(&principal.name))
        .collect();
    let today = principal.now.date_naive();
    let reservations = state
        .club
        .reservation_list(
            &user,
            &vereinsakte_app::reservations::ReservationFilter {
                from: Some(today),
                to: Some(today + chrono::Duration::days(60)),
                mine: true,
                ..Default::default()
            },
        )
        .await?
        .rows;
    let layout = Layout::build(&state, &session, Some(&user), "Start", "/").await?;
    let todo = todo_items(&v, confirmations, tasks.len());
    render(&StartPage {
        layout,
        v,
        todo,
        events,
        tasks,
        reservations,
    })
}

#[derive(Template)]
#[template(path = "my_data.html")]
struct MyDataPage {
    layout: Layout,
    v: MyDataView,
    sorts: DetailSorts,
}

pub async fn my_data(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Response, WebError> {
    // Bewusst ohne Mitgliedsangabe: es gibt nur die Daten des angemeldeten Mitglieds.
    let mut v = state.club.my_data(&user).await?;
    let sorts = sort_details(&mut v, &params, "/meine-daten");
    let layout =
        Layout::build(&state, &session, Some(&user), "Meine Daten", "/meine-daten").await?;
    render(&MyDataPage { layout, v, sorts })
}

/// Hinweis zur Sichtbarkeit der Kontaktdaten bestätigen.
pub async fn dismiss_contact_notice(
    State(state): State<AppState>,
    RequireUser(user): RequireUser,
) -> Result<Response, WebError> {
    state.club.dismiss_contact_notice(&user).await?;
    Ok(axum::response::Redirect::to("/").into_response())
}

/// Sperrvermerk für die eigenen Kontaktdaten setzen oder entfernen.
pub async fn set_contact_block(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    axum::Form(fields): axum::Form<std::collections::HashMap<String, String>>,
) -> Result<Response, WebError> {
    let blocked = fields.contains_key("blocked");
    state
        .club
        .set_contact_blocked(&user, user.member, blocked)
        .await?;
    let text = if blocked {
        "Deine Kontaktdaten sind jetzt für andere Mitglieder gesperrt."
    } else {
        "Deine Kontaktdaten sind wieder für alle Mitglieder sichtbar."
    };
    Ok(crate::forms::redirect_with_flash(&session, "/meine-daten", text).await)
}

#[derive(Deserialize)]
pub struct DirectoryQuery {
    #[serde(default)]
    q: String,
    #[serde(default)]
    art: String,
    #[serde(default)]
    schein: String,
    #[serde(default)]
    sort_verz: String,
    #[serde(default)]
    dir_verz: String,
}

#[derive(Template)]
#[template(path = "directory.html")]
struct DirectoryPage {
    layout: Layout,
    v: DirectoryView,
    types: Vec<(&'static str, String)>,
    sort: SortCtx,
    active_filters: usize,
}

#[derive(Template)]
#[template(path = "directory_table.html")]
struct DirectoryTable {
    v: DirectoryView,
    sort: SortCtx,
}

pub async fn directory(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Query(query): Query<DirectoryQuery>,
    headers: HeaderMap,
) -> Result<Response, WebError> {
    let sort = Sort {
        key: query.sort_verz.clone(),
        desc: query.dir_verz == "desc",
    };
    let filter = DirectoryFilter {
        query: query.q,
        membership_type: parse_membership_type(&query.art),
        license: query.schein,
    };
    let mut v = state.club.directory_filtered(&user, &filter).await?;
    sort_rows(
        &mut v.entries,
        &sort,
        &[
            ("name", |e| text_key(&e.name)),
            ("nr", |e| SortKey::Num(i64::from(e.member_number))),
            ("art", |e| text_key(&e.membership_type)),
            ("email", |e| text_key(e.contact.email().unwrap_or(""))),
        ],
    );
    let params = vec![
        ("q".to_string(), v.query.clone()),
        ("art".to_string(), v.membership_type.clone()),
        ("schein".to_string(), v.license.clone()),
    ];
    let ctx = SortCtx::new(
        "verz",
        "/mitglieder",
        params.clone(),
        &sort,
        &[
            ("name", "Name"),
            ("nr", "Nummer"),
            ("art", "Mitgliedsart"),
            ("email", "E-Mail"),
        ],
    );
    if headers.contains_key("hx-request") {
        return render(&DirectoryTable { v, sort: ctx });
    }
    let active_filters = params.iter().filter(|(_, value)| !value.is_empty()).count();
    let mut next_params: Vec<String> = params
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(k, value)| format!("{k}={}", crate::session::encode(value)))
        .collect();
    if !sort.key.is_empty() {
        next_params.push(format!("sort_verz={}", crate::session::encode(&sort.key)));
        if sort.desc {
            next_params.push("dir_verz=desc".to_string());
        }
    }
    let next = if next_params.is_empty() {
        "/mitglieder".to_string()
    } else {
        format!("/mitglieder?{}", next_params.join("&"))
    };
    let layout = Layout::build(&state, &session, Some(&user), "Mitglieder", &next).await?;
    render(&DirectoryPage {
        layout,
        v,
        types: membership_types(),
        sort: ctx,
        active_filters,
    })
}

pub async fn health(State(state): State<AppState>) -> Response {
    match state.club.health().await {
        Ok(()) => (StatusCode::OK, "ok").into_response(),
        Err(error) => {
            tracing::warn!(%error, "Health-Check fehlgeschlagen");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Datenbank nicht erreichbar",
            )
                .into_response()
        }
    }
}

const HTMX: &[u8] = include_bytes!("../static/htmx.min.js");
const CSS: &str = include_str!("../static/app.css");

pub async fn htmx() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        HTMX,
    )
        .into_response()
}

pub async fn css() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        CSS,
    )
        .into_response()
}
