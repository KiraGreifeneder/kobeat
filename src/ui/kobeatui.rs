use std::process::Command;

use anyhow::{Context, Result};
use axum::{
    Router,
    extract::Query,
    response::{Html, Redirect},
    routing::{get, post},
};
use maud::{DOCTYPE, Markup, html};
use serde::Deserialize;
use tokio::net::TcpListener;

#[derive(Debug, Default, Deserialize)]
struct IndexParams {
    message: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    Command::new("ip")
        .arg("addr")
        .arg("add")
        .arg("127.0.0.1/8")
        .arg("dev")
        .arg("lo")
        .status()
        .expect("Failed to execute command to add 127.0.0.1/8 to loopback");

    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .context("failed to bind web UI to 127.0.0.1:3000")?;

    println!("kobeat web UI listening on http://127.0.0.1:3000");

    axum::serve(listener, app())
        .await
        .context("web UI server stopped unexpectedly")?;

    Ok(())
}

fn app() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/play", post(play))
        .route("/pause", post(pause))
}

async fn index(Query(params): Query<IndexParams>) -> Html<String> {
    Html(render_page(params.message.as_deref()).into_string())
}

async fn play() -> Redirect {
    Redirect::to("/?message=Play%20button%20pressed")
}

async fn pause() -> Redirect {
    Redirect::to("/?message=Pause%20button%20pressed")
}

fn render_page(message: Option<&str>) -> Markup {
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
                            max-width: 22rem;
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
                        button type="submit" { "Play" }
                    }

                    form method="post" action="/pause" {
                        button type="submit" { "Pause" }
                    }

                    p class="note" {
                        "These buttons are placeholder actions for now and only confirm the tap worked."
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
    }
}
