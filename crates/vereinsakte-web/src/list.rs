//! Listen: Abfrageparameter in der Adresse, Seitenwechsel, Teilantworten für htmx.

use axum::http::HeaderMap;
use serde::Deserialize;

/// Anzahl der Einträge je Seite.
pub const PER_PAGE: usize = 25;

/// Gemeinsame Abfrageparameter der Listenansichten.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct ListQuery {
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub status: String,
    pub jahr: Option<i32>,
    pub seite: Option<usize>,
    #[serde(default)]
    pub sort: String,
    /// Weitere Parameter, die in Seitenadressen erhalten bleiben (nicht aus der Adresse gelesen).
    #[serde(skip)]
    pub extra: Vec<(String, String)>,
}

impl ListQuery {
    pub fn page(&self) -> usize {
        self.seite.unwrap_or(1).max(1)
    }

    /// Adresse mit denselben Filtern und anderer Seite (Parameter in fester Reihenfolge).
    pub fn url(&self, base: &str, page: usize) -> String {
        let mut params: Vec<(&str, String)> = vec![];
        if !self.q.is_empty() {
            params.push(("q", self.q.clone()));
        }
        if !self.status.is_empty() {
            params.push(("status", self.status.clone()));
        }
        if let Some(year) = self.jahr {
            params.push(("jahr", year.to_string()));
        }
        if !self.sort.is_empty() {
            params.push(("sort", self.sort.clone()));
        }
        for (key, value) in &self.extra {
            if !value.is_empty() {
                params.push((key.as_str(), value.clone()));
            }
        }
        if page > 1 {
            params.push(("seite", page.to_string()));
        }
        if params.is_empty() {
            return base.to_string();
        }
        let query: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{k}={}", crate::session::encode(v)))
            .collect();
        format!("{base}?{}", query.join("&"))
    }
}

/// Seitenwechsel einer Liste.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pager {
    pub page: usize,
    pub pages: usize,
    pub total: usize,
    pub first: usize,
    pub last: usize,
    pub previous: Option<String>,
    pub next: Option<String>,
    /// (Seitennummer, Adresse); die aktuelle Seite hat keine Adresse.
    pub links: Vec<(usize, Option<String>)>,
}

impl Pager {
    pub fn visible(&self) -> bool {
        self.pages > 1
    }
}

/// Schneidet die gewünschte Seite aus `items` heraus.
pub fn paginate<T>(items: Vec<T>, query: &ListQuery, base: &str) -> (Vec<T>, Pager) {
    let total = items.len();
    let pages = total.div_ceil(PER_PAGE).max(1);
    let page = query.page().min(pages);
    let start = (page - 1) * PER_PAGE;
    let shown: Vec<T> = items.into_iter().skip(start).take(PER_PAGE).collect();
    let pager = Pager {
        page,
        pages,
        total,
        first: if total == 0 { 0 } else { start + 1 },
        last: start + shown.len(),
        previous: (page > 1).then(|| query.url(base, page - 1)),
        next: (page < pages).then(|| query.url(base, page + 1)),
        links: (1..=pages)
            .map(|n| (n, (n != page).then(|| query.url(base, n))))
            .collect(),
    };
    (shown, pager)
}

/// Kommt die Anfrage von htmx (Teilantwort genügt)?
pub fn is_htmx(headers: &HeaderMap) -> bool {
    headers.contains_key("hx-request")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_page_starts_after_25_entries() {
        let items: Vec<u32> = (1..=30).collect();
        let query = ListQuery {
            seite: Some(2),
            q: "a b".into(),
            ..Default::default()
        };
        let (shown, pager) = paginate(items, &query, "/liste");
        assert_eq!(shown, (26..=30).collect::<Vec<_>>());
        assert_eq!(
            (pager.first, pager.last, pager.total, pager.pages),
            (26, 30, 30, 2)
        );
        assert_eq!(pager.previous.as_deref(), Some("/liste?q=a+b"));
        assert!(pager.next.is_none());
    }

    #[test]
    fn short_list_has_no_pager() {
        let (shown, pager) = paginate(vec![1, 2, 3], &ListQuery::default(), "/x");
        assert_eq!(shown.len(), 3);
        assert!(!pager.visible());
    }

    #[test]
    fn out_of_range_page_is_clamped() {
        let query = ListQuery {
            seite: Some(99),
            ..Default::default()
        };
        let (shown, pager) = paginate((1..=30).collect::<Vec<u32>>(), &query, "/x");
        assert_eq!(pager.page, 2);
        assert_eq!(shown.len(), 5);
    }
}
