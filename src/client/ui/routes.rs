use axum::{
    Form, Router,
    extract::Query,
    response::Html,
    routing::{get, post},
};
use serde::Deserialize;
use std::path::PathBuf;

use crate::{client::client, domain::player_command::*};

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
        .route("/status", get(status))
    //TODO more
}

async fn index(Query(params): Query<IndexParams>) -> Html<String> {
    Html(render_page(params.message.as_deref(), params.path.as_deref()).into_string())
}

async fn play(Form(form): Form<PlayForm>) -> Html<String> {
    let command = match form.path {
        Some(path) => PlayerCommand::LoadAndPlay { path },
        None => PlayerCommand::Resume,
    };

    let response = send_command(command).await;
    render_command_result(response).await
}

async fn pause() -> Html<String> {
    let response = send_command(PlayerCommand::Pause).await;
    render_command_result(response).await
}

async fn status() -> Html<String> {
    let response = send_command(PlayerCommand::GetPlayerState).await;
    render_command_result(response).await
}

async fn send_command(command: PlayerCommand) -> PlayerResponse {
    tokio::task::spawn_blocking(move || client::send_command(&command))
        .await
        .map(|result| match result {
            Ok(response) => serde_json::from_str(&response).unwrap_or_else(|error| {
                PlayerResponse::Error(format!("Failed to parse response: {error:#}"))
            }),
            Err(error) => PlayerResponse::Error(format!("Command failed: {error:#}")),
        })
        .unwrap_or_else(|error| {
            PlayerResponse::Error(format!("Failed to join command task: {error:#}"))
        })
}

async fn render_command_result(response: PlayerResponse) -> Html<String> {
    let message = match &response {
        PlayerResponse::Ok(msg) => format!("OK: {}", msg),
        PlayerResponse::State(state) => format!("State: {:?}", state),
        PlayerResponse::Error(msg) => format!("Error: {}", msg),
    };

    let state = match &response {
        PlayerResponse::State(state) => Some(state),
        _ => None,
    };

    Html(render_page(Some(&message), state, None).into_string())
}
