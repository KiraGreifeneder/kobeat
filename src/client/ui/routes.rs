use axum::{
    Form, Router,
    extract::Query,
    response::Html,
    routing::{get, post},
};
use serde::Deserialize;
use std::path::PathBuf;

use crate::{client::client, domain::player_command::PlayerCommand};

use super::page::render_page;

#[derive(Debug, Default, Deserialize)]
struct IndexParams {
    message: Option<String>,
    /// Directory currently being browsed. Defaults to the library root.
    path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct PlayForm {
    /// Track to play. Absent when the transport bar's play button is used,
    /// which resumes instead.
    #[serde(default)]
    path: Option<String>,
}

pub fn app() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/play", post(play))
        .route("/pause", post(pause))
}

async fn index(Query(params): Query<IndexParams>) -> Html<String> {
    Html(render_page(params.message.as_deref(), params.path.as_deref()).into_string())
}

async fn play(Form(form): Form<PlayForm>) -> Html<String> {
    let command = match form.path {
        Some(path) => PlayerCommand::LoadAndPlay { path },
        None => PlayerCommand::Resume,
    };

    render_command_result(command).await
}

async fn pause() -> Html<String> {
    render_command_result(PlayerCommand::Pause).await
}

async fn render_command_result(command: PlayerCommand) -> Html<String> {
    let message = tokio::task::spawn_blocking(move || client::send_command(&command))
        .await
        .map(|result| match result {
            Ok(response) => format!("OK: {}", response.trim()),
            Err(error) => format!("Error: {error:#}"),
        })
        .unwrap_or_else(|error| format!("Error: failed to join command task: {error}"));

    Html(render_page(Some(&message), None).into_string())
}
