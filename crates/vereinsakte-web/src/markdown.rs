//! Markdown für Aufgabenbeschreibungen und Kommentare. Die Eingabe ist nicht
//! vertrauenswürdig: eingebettetes HTML wird als Text ausgegeben, Links nur mit
//! http, https oder mailto, Bilder gar nicht (Bilder sind Anhänge).

use pulldown_cmark::{CowStr, Event, LinkType, Options, Parser, Tag, TagEnd, html};

fn allowed_link(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")
}

/// Rendert Markdown zu sicherem HTML (Ergebnis mit `|safe` einzusetzen).
pub fn render(source: &str) -> String {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH;
    let mut image_depth = 0usize;
    let events = Parser::new_ext(source, options).filter_map(|event| match event {
        // Rohes HTML wird zu Text (und damit beim Ausgeben maskiert).
        Event::Html(text) | Event::InlineHtml(text) => Some(Event::Text(text)),
        // Bilder werden nicht gerendert; ihr Alternativtext bleibt als Text.
        Event::Start(Tag::Image { .. }) => {
            image_depth += 1;
            None
        }
        Event::End(TagEnd::Image) => {
            image_depth = image_depth.saturating_sub(1);
            None
        }
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => {
            let ok = link_type == LinkType::Email || allowed_link(&dest_url);
            let dest = if ok { dest_url } else { CowStr::from("") };
            Some(Event::Start(Tag::Link {
                link_type,
                dest_url: dest,
                title,
                id,
            }))
        }
        other => Some(other),
    });
    let mut out = String::new();
    html::push_html(&mut out, events);
    // Links bekommen Schutzattribute; leere Ziele (verbotene Schemas) werden zu Text.
    out = out.replace("<a href=\"\"", "<a");
    out.replace("<a href=", "<a rel=\"noopener noreferrer nofollow\" href=")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_common_markdown() {
        let html = render("**Wichtig:** Fock\n\n- [x] bestellt\n- [ ] genäht\n\n> Zitat\n\n`code`");
        assert!(html.contains("<strong>Wichtig:</strong>"));
        assert!(html.contains("type=\"checkbox\""));
        assert!(html.contains("<blockquote>") && html.contains("<code>code</code>"));
    }

    #[test]
    fn raw_html_is_text() {
        let html = render("<script>alert(1)</script> und <img src=x onerror=alert(1)>");
        assert!(!html.contains("<script") && !html.contains("<img"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn dangerous_links_are_not_links() {
        let html = render(
            "[klick](javascript:alert(1)) [data](data:text/html;base64,AAAA) [ok](https://example.org)",
        );
        assert!(!html.contains("javascript:") && !html.contains("data:text"));
        assert!(html.contains(
            "<a rel=\"noopener noreferrer nofollow\" href=\"https://example.org\">ok</a>"
        ));
        assert!(html.contains("klick") && html.contains("data"));
        assert!(render("[m](mailto:a@b.de)").contains("mailto:a@b.de"));
    }

    #[test]
    fn markdown_images_are_not_rendered() {
        let html = render("![Alt](https://example.org/x.png) Text");
        assert!(!html.contains("<img"));
        assert!(html.contains("Alt"));
    }
}
