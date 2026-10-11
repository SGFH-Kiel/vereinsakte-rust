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

/// Sortierwert einer Zelle: Zahlen nach Wert, Text nach deutscher Reihenfolge.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SortKey {
    Num(i64),
    Text(String),
}

/// Text für die deutsche Sortierreihenfolge: Groß-/Kleinschreibung ignoriert,
/// Umlaute und ß wie ihre Grundbuchstaben (DIN 5007-1).
pub fn text_key(text: &str) -> SortKey {
    let mut folded = String::with_capacity(text.len());
    for c in text.trim().chars().flat_map(char::to_lowercase) {
        match c {
            'ä' => folded.push('a'),
            'ö' => folded.push('o'),
            'ü' => folded.push('u'),
            'ß' => folded.push_str("ss"),
            'é' | 'è' | 'ê' => folded.push('e'),
            'á' | 'à' | 'â' => folded.push('a'),
            _ => folded.push(c),
        }
    }
    SortKey::Text(folded)
}

/// Zahl oder Betrag in deutscher Schreibweise („1.234,50 €“, „4,5“) als Hundertstel.
pub fn num_key(text: &str) -> SortKey {
    let cleaned: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',' || *c == '-')
        .collect();
    let value: f64 = cleaned.replace(',', ".").parse().unwrap_or(0.0);
    SortKey::Num((value * 100.0).round() as i64)
}

/// Datum „TT.MM.JJJJ“ (auch mit Uhrzeit) als sortierbare Zahl; sonst Text.
pub fn date_key(text: &str) -> SortKey {
    let part = text.split_whitespace().next().unwrap_or("");
    let numbers: Vec<&str> = part.trim_end_matches(',').split('.').collect();
    if let [d, m, y] = numbers[..]
        && let (Ok(d), Ok(m), Ok(y)) = (d.parse::<i64>(), m.parse::<i64>(), y.parse::<i64>())
    {
        return SortKey::Num(y * 10_000 + m * 100 + d);
    }
    text_key(text)
}

/// Gewählte Sortierung einer Tabelle (aus `sort_<id>` und `dir_<id>`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sort {
    pub key: String,
    pub desc: bool,
}

impl Sort {
    pub fn from_params(params: &std::collections::HashMap<String, String>, id: &str) -> Self {
        Self {
            key: params
                .get(&format!("sort_{id}"))
                .cloned()
                .unwrap_or_default(),
            desc: params
                .get(&format!("dir_{id}"))
                .is_some_and(|d| d == "desc"),
        }
    }
}

/// Sortiert `rows` nach der gewählten Spalte; unbekannte Schlüssel lassen die
/// Reihenfolge unverändert (Standardreihenfolge der Ansicht).
pub fn sort_rows<T>(rows: &mut [T], sort: &Sort, keys: &[(&str, fn(&T) -> SortKey)]) {
    let Some((_, key)) = keys.iter().find(|(name, _)| *name == sort.key) else {
        return;
    };
    rows.sort_by(|a, b| key(a).cmp(&key(b)));
    if sort.desc {
        rows.reverse();
    }
}

/// Sortierkopf einer Tabelle: erzeugt Adressen, `aria-sort` und Symbole.
#[derive(Clone, Debug)]
pub struct SortCtx {
    pub id: String,
    base: String,
    /// Übrige Parameter der Adresse (Filter usw.).
    params: Vec<(String, String)>,
    sort: Sort,
    /// Auswahl für die Kartenansicht: Schlüssel und Beschriftung.
    pub options: Vec<(String, String)>,
}

impl SortCtx {
    pub fn new(
        id: &str,
        base: &str,
        params: Vec<(String, String)>,
        sort: &Sort,
        options: &[(&str, &str)],
    ) -> Self {
        Self {
            id: id.to_string(),
            base: base.to_string(),
            params: params.into_iter().filter(|(_, v)| !v.is_empty()).collect(),
            sort: sort.clone(),
            options: options
                .iter()
                .map(|(k, l)| ((*k).to_string(), (*l).to_string()))
                .collect(),
        }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub fn params(&self) -> &[(String, String)] {
        &self.params
    }

    pub fn selected(&self) -> &str {
        &self.sort.key
    }

    pub fn is_desc(&self) -> bool {
        self.sort.desc
    }

    /// Adresse, die nach `key` sortiert; ein erneuter Klick kehrt die Richtung um.
    pub fn link(&self, key: &str) -> String {
        let desc = self.sort.key == key && !self.sort.desc;
        let mut parts: Vec<String> = self
            .params
            .iter()
            .map(|(k, v)| format!("{k}={}", crate::session::encode(v)))
            .collect();
        parts.push(format!("sort_{}={}", self.id, crate::session::encode(key)));
        if desc {
            parts.push(format!("dir_{}=desc", self.id));
        }
        format!("{}?{}", self.base, parts.join("&"))
    }

    pub fn aria(&self, key: &str) -> &'static str {
        match (self.sort.key == key, self.sort.desc) {
            (true, false) => "ascending",
            (true, true) => "descending",
            _ => "none",
        }
    }

    pub fn icon(&self, key: &str) -> &'static str {
        match (self.sort.key == key, self.sort.desc) {
            (true, false) => "arrow-up",
            (true, true) => "arrow-down",
            _ => "sort",
        }
    }

    pub fn sort_name(&self) -> String {
        format!("sort_{}", self.id)
    }

    pub fn dir_name(&self) -> String {
        format!("dir_{}", self.id)
    }
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
    fn german_text_order_places_umlauts_with_base_letters() {
        let mut names = vec![
            "Petersen", "Möller", "Lorenzen", "Ärmel", "zander", "Moller",
        ];
        let sort = Sort {
            key: "name".into(),
            desc: false,
        };
        sort_rows(&mut names, &sort, &[("name", |s: &&str| text_key(s))]);
        assert_eq!(
            names,
            vec![
                "Ärmel", "Lorenzen", "Möller", "Moller", "Petersen", "zander"
            ]
        );
    }

    #[test]
    fn numbers_dates_and_direction() {
        assert!(num_key("1.234,50 €") > num_key("999,00 €"));
        assert!(num_key("-5,00 €") < num_key("0,00 €"));
        assert!(date_key("09.10.2026") > date_key("30.01.2026"));
        assert!(date_key("17.10.2026, 09:00 Uhr") > date_key("01.10.2026"));
        let mut rows = vec!["30.01.2026", "09.10.2026", "01.03.2026"];
        let desc = Sort {
            key: "datum".into(),
            desc: true,
        };
        sort_rows(&mut rows, &desc, &[("datum", |s: &&str| date_key(s))]);
        assert_eq!(rows, vec!["09.10.2026", "01.03.2026", "30.01.2026"]);
        // Unbekannter Schlüssel: Reihenfolge bleibt.
        let unknown = Sort {
            key: "xyz".into(),
            desc: false,
        };
        sort_rows(&mut rows, &unknown, &[("datum", |s: &&str| date_key(s))]);
        assert_eq!(rows, vec!["09.10.2026", "01.03.2026", "30.01.2026"]);
    }

    #[test]
    fn sort_links_toggle_direction_and_keep_filters() {
        let params = vec![("q".to_string(), "peter".to_string())];
        let ctx = SortCtx::new("verz", "/mitglieder", params.clone(), &Sort::default(), &[]);
        assert_eq!(ctx.link("name"), "/mitglieder?q=peter&sort_verz=name");
        assert_eq!(ctx.aria("name"), "none");
        let current = Sort {
            key: "name".into(),
            desc: false,
        };
        let ctx = SortCtx::new("verz", "/mitglieder", params, &current, &[]);
        assert_eq!(ctx.aria("name"), "ascending");
        assert_eq!(
            ctx.link("name"),
            "/mitglieder?q=peter&sort_verz=name&dir_verz=desc"
        );
        assert_eq!(ctx.icon("name"), "arrow-up");
        assert_eq!(ctx.icon("art"), "sort");
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
