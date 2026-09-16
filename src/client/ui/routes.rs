use axum::{
    Form, Router,
    extract::Query,
    response::Html,
    routing::{get, post},
};
use serde::Deserialize;
use std::path::PathBuf;

use crate::{client::client, client::directory_snapshot::DirectoryTree, domain::player_command::*};

use super::page::render_page;

#[derive(Debug, Default, Deserialize)]
struct IndexParams {
    message: Option<String>,
    /// Directory currently being browsed. Defaults to the library root.
    path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct PlayForm {
    #[serde(default)]
    path: Option<String>, // Path to the track to play. If not provided, resumes playback.
}

#[derive(Debug, Default, Deserialize)]
struct SeekForm {
    #[serde(default)]
    position_secs: u64,
}

#[derive(Debug, Default, Deserialize)]
struct QueueForm {
    /// Directory whose listing should become the queue.
    dir: PathBuf,
    /// Whether to also include files from every scanned sub-directory.
    #[serde(default)]
    include_subdirs: bool,
}

pub fn app() -> Router {
    println!("app func");
    let router = Router::new()
        .route("/", get(index))
        .route("/ping", get(ping))
        .route("/play", post(play))
        .route("/pause", post(pause))
        .route("/status", get(status))
        .route("/seek", post(seek))
        .route("/queue", post(queue))
        .route("/next", post(next))
        .route("/previous", post(previous))
        .route("/shutdown", post(shutdown));
    println!("created routes");
    return router;
}

/// Bare-bones route that touches nothing else (no daemon socket, no
/// rendering). If this hangs or never logs, the axum server itself isn't
/// receiving/handling the request. If this works but `/` doesn't, the
/// problem is downstream (daemon socket, rendering, etc).
async fn ping() -> &'static str {
    println!("pinggg!!");
    tracing::info!("ping handler reached");
    "hello"
}

async fn index(Query(params): Query<IndexParams>) -> Html<String> {
    println!("index!");
    let response = send_command(PlayerCommand::GetPlayerState).await;
    println!("fetched player state");
    let state = match &response {
        PlayerResponse::State(state) => Some(state),
        _ => None,
    };

    Html(render_page(params.message.as_deref(), state, params.path.as_deref()).into_string())
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

async fn seek(Form(form): Form<SeekForm>) -> Html<String> {
    let response = send_command(PlayerCommand::Seek {
        position_secs: form.position_secs,
    })
    .await;
    render_command_result(response).await
}

async fn queue(Form(form): Form<QueueForm>) -> Html<String> {
    let tree = match DirectoryTree::scan(&form.dir, 3) {
        Ok(tree) => tree,
        Err(error) => {
            return render_command_result(PlayerResponse::Error(format!(
                "Failed to scan {}: {error:#}",
                form.dir.display()
            )))
            .await;
        }
    };

    let paths = tree
        .files(&form.dir, form.include_subdirs)
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();

    let response = send_command(PlayerCommand::SetQueue { paths }).await;
    render_command_result(response).await
}

async fn next() -> Html<String> {
    let response = send_command(PlayerCommand::PlayNext).await;
    render_command_result(response).await
}

async fn shutdown() -> Html<String> {
    let response = send_command(PlayerCommand::Shutdown).await;
    render_command_result(response).await
}

async fn previous() -> Html<String> {
    let response = send_command(PlayerCommand::PlayPrevious).await;
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

    // Most commands (Seek, Play, Pause, Next, Previous, ...) return a plain
    // Ok/Error response, not the full PlayerState. Always fetch fresh state
    // to render with, rather than only using it when the command happened
    // to return one — otherwise the UI (in particular the seek slider)
    // renders as if playback were stopped at position 0 after every action
    // except an explicit status check, even though the action succeeded.
    let state_response = send_command(PlayerCommand::GetPlayerState).await;
    let state = match &state_response {
        PlayerResponse::State(state) => Some(state),
        _ => None,
    };

    Html(render_page(Some(&message), state, None).into_string())
}
