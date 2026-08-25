use anyhow::{Context, Result};
use axum::{
    Router,
    extract::Query,
    response::Html,
    routing::{get, post},
};
use kobeat::{control, entity::player_command::PlayerCommand};
use maud::{DOCTYPE, Markup, html};
use serde::Deserialize;
use tokio::net::TcpListener;

const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3000";

#[derive(Debug, Default, Deserialize)]
struct IndexParams {
    message: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let bind_addr =
        std::env::var("KOBEATUI_BIND").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string());
    let listener = TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind web UI to {bind_addr}"))?;

    println!("kobeat web UI listening on http://{bind_addr}");

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

async fn play() -> Html<String> {
    render_command_result(PlayerCommand::Resume).await
}

async fn pause() -> Html<String> {
    render_command_result(PlayerCommand::Pause).await
}

async fn render_command_result(command: PlayerCommand) -> Html<String> {
    let message = tokio::task::spawn_blocking(move || control::send_command(&command))
        .await
        .map(|result| match result {
            Ok(response) => format!("OK: {}", response.trim()),
            Err(error) => format!("Error: {error:#}"),
        })
        .unwrap_or_else(|error| format!("Error: failed to join command task: {error}"));

    Html(render_page(Some(&message)).into_string())
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
                        "This minimal screen talks to the daemon through the shared control library."
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_BIND_ADDR, render_page};

    #[test]
    fn page_contains_controls() {
        let page = render_page(Some("Play button pressed")).into_string();

        assert!(page.contains("Minimal control page for Kobo rendering tests."));
        assert!(page.contains("form method=\"post\" action=\"/play\""));
        assert!(page.contains("form method=\"post\" action=\"/pause\""));
        assert!(page.contains("Play button pressed"));
        assert!(page.contains("Play / Resume"));
    }

    #[test]
    fn default_bind_addr_is_wildcard() {
        assert_eq!(DEFAULT_BIND_ADDR, "0.0.0.0:3000");
    }
}
