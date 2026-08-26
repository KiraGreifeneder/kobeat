use maud::{DOCTYPE, Markup, html};

pub fn render_page(message: Option<&str>) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "kobeat" }
                style {
                    r#"
                        body {
                            margin: 0;
                            font-family: sans-serif;
                            background: #f4f1ea;
                            color: #1f1a17;
                        }

                        main {
                            max-width: 25rem;
                            margin: 2rem auto;
                            padding: 1.5rem;
                            background: #fffdf8;
                            border: 1px solid #d8d0c4;
                            border-radius: 0.75rem;
                        }

                        h1 {
                            margin-top: 0;
                            font-size: 1.5rem;
                        }

                        p {
                            line-height: 1.4;
                        }

                        form + form {
                            margin-top: 0.75rem;
                        }

                        button {
                            width: 100%;
                            padding: 0.9rem 1rem;
                            font-size: 1rem;
                            border: 1px solid #1f1a17;
                            border-radius: 999px;
                            background: #fff;
                        }

                        .flash {
                            padding: 0.75rem 1rem;
                            background: #efe8d8;
                            border-radius: 0.5rem;
                        }

                        .note {
                            color: #5f5750;
                            font-size: 0.95rem;
                        }
                    "#
                }
            }
            body {
                main {
                    h1 { "kobeat" }
                    p { "Minimal control page for Kobo rendering tests." }

                    @if let Some(message) = message {
                        p class="flash" { (message) }
                    }

                    form method="post" action="/play" {
                        button type="submit" { "Play / Resume" }
                    }

                    form method="post" action="/pause" {
                        button type="submit" { "Pause" }
                    }

                    p class="note" {
                        "This minimal screen talks to the daemon through the shared daemon client."
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::render_page;

    #[test]
    fn page_contains_controls() {
        let page = render_page(Some("Play button pressed")).into_string();

        assert!(page.contains("Minimal control page for Kobo rendering tests."));
        assert!(page.contains("form method=\"post\" action=\"/play\""));
        assert!(page.contains("form method=\"post\" action=\"/pause\""));
        assert!(page.contains("Play button pressed"));
        assert!(page.contains("Play / Resume"));
    }
}
