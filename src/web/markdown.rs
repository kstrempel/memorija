//! Server-side markdown → HTML rendering for the web viewer (design D2).

use pulldown_cmark::{Event, Options, Parser, html};

/// Render a memory file's markdown into HTML. Raw HTML in the source is
/// escaped instead of passed through (pulldown-cmark forwards HTML events
/// verbatim, so they are rewritten to text here, which `push_html` then
/// escapes): a `<script>` tag in a memory file renders as visible text and
/// cannot execute in the browser.
pub fn render(source: &str) -> String {
    let mut options = Options::empty();
    options
        .insert(Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS);
    let events = Parser::new_ext(source, options).map(|event| match event {
        Event::Html(html) | Event::InlineHtml(html) => Event::Text(html),
        other => other,
    });
    let mut out = String::new();
    html::push_html(&mut out, events);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_headings_lists_code_and_emphasis() {
        let html = render("# Title\n\n- one\n- two\n\n`code` and **bold**\n");
        assert!(html.contains("<h1>Title</h1>"), "{html}");
        assert!(html.contains("<ul>"), "{html}");
        assert!(html.contains("<li>one</li>"), "{html}");
        assert!(html.contains("<code>code</code>"), "{html}");
        assert!(html.contains("<strong>bold</strong>"), "{html}");
    }

    #[test]
    fn raw_html_is_escaped_to_text() {
        let html = render("before\n\n<script>alert(1)</script>\n\nafter\n");
        assert!(!html.contains("<script>"), "no live script tag: {html}");
        assert!(html.contains("&lt;script&gt;"), "escaped to text: {html}");
        assert!(
            html.contains("alert(1)"),
            "content still readable as text: {html}"
        );
    }

    #[test]
    fn inline_html_is_escaped_too() {
        let html = render("hello <img src=x onerror=alert(1)> world\n");
        assert!(!html.contains("<img"), "no live inline HTML: {html}");
        assert!(html.contains("&lt;img"), "escaped to text: {html}");
    }
}
