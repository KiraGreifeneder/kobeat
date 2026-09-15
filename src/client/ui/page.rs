use std::path::Path;

use crate::{
    client::directory_snapshot::{DirectoryNode, DirectoryTree},
    config::MUSIC_DIR,
    domain::state::{PlaybackState, PlayerState},
};
use maud::{DOCTYPE, Markup, html};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

/// Link to the listing for `dir`. The path is percent-encoded so that spaces,
/// apostrophes, and `&` in album names survive the round trip.
fn browse_href(dir: &Path) -> String {
    format!(
        "/?path={}",
        utf8_percent_encode(&dir.to_string_lossy(), NON_ALPHANUMERIC)
    )
}

/// Label a path by its final component, falling back to the full path.
fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

pub fn render_page(
    message: Option<&str>,
    player_state: Option<&PlayerState>,
    browsing: Option<&Path>,
) -> Markup {
    let tree = DirectoryTree::scan(Path::new(MUSIC_DIR), 3).ok();

    let displayed_snapshot: Option<&DirectoryNode> = tree.as_ref().map(|tree| {
        browsing
            .and_then(|dir| tree.node(dir))
            .unwrap_or_else(|| tree.root())
    });

    let parent = tree
        .as_ref()
        .zip(displayed_snapshot)
        .and_then(|(tree, snapshot)| tree.parent(&snapshot.dir));

    html! {
        (DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "kobeat" }
                style {
                    r#"
                        *, *::before, *::after {
                            box-sizing: border-box;
                        }

                        body {
                            margin: 0;
                            font-family: sans-serif;
                            background: black;
                            color: #1f1a17;
                        }

                        main {
                            padding: 1.5rem;
                            padding-bottom: 6rem;
                            color: white;
                            max-width: 100%;
                            overflow-x: hidden;
                        }

                        list {
                            display: block;
                            max-width: 100%;
                        }

                        .controls__container {
                            width: 100%;
                            background: #8eea60;
                            position: fixed;
                            bottom: 0px;
                            height: 3rem;
                            padding: 1rem;
                            align-items: center;
                            gap: 1rem;
                        }

                        .controls__button_row {
                            width: 100%;
                            display: flex;
                            align-items: center;
                            gap: 1rem;
                        }

                        h1 {
                            margin-top: 0;
                            font-size: 1.5rem;
                        }

                        p {
                            line-height: 1.4;
                        }

                        .control_button {
                            width: 3rem;
                            padding: 0.9rem 1rem;
                            font-size: 1rem;
                            border: 1px solid #1f1a17;
                            border-radius: 999px;
                            background: #fff;
                            height: 3rem;
                            display: flex;
                            align-items: center;
                            justify-content: center;
                        }

                        .list_item {
                            display: block;
                            width: 100%;
                            border: none;
                            border-bottom: 1px solid #1f1a17;
                            padding: 0.75rem 1rem;
                            cursor: pointer;
                            background: #101010;
                            color: white;
                            transition: background-color 0.2s;
                            text-align: left;
                            font: inherit;
                            text-decoration: none;
                            white-space: nowrap;
                            overflow: hidden;
                            text-overflow: ellipsis;
                            min-width: 0;
                        }

                        .list_item__file {
                            padding-right: 1rem;
                        }

                        .list_item__folder {
                            font-weight: bold;
                        }


                        .flash {
                            padding: 0.75rem 1rem;
                            background: aqua;
                            border-radius: 0.5rem;
                            border: 1px solid white;
                            color: black;
                        }

                        .flash__success {
                            background: green;
                        }

                        .flash__error {
                            background: red;
                        }

                        .note {
                            color: gray;
                            font-size: 0.95rem;
                        }

                        .queue_form {
                            display: flex;
                            align-items: center;
                            gap: 0.75rem;
                            margin: 0.75rem 0;
                        }
                    "#
                }
            }
            body {
                main {
                    h1 { "kobeat" }
                    p { "Offline controls in the browser." }

                    form method="post" action="/shutdown" {
                        button type="submit" { "Shutdown daemon" }
                    }

                    @if let Some(message) = message {
                        p class="flash" { (message) }
                    }

                    p class="note" {
                        "This minimal screen talks to the daemon through the shared daemon client."
                    }

                    @if let Some(snapshot) = displayed_snapshot {
                        h2 { (snapshot.dir.display()) }
                    } @else {
                        h2 { "no music library found" }
                    }

                    @if let Some(snapshot) = displayed_snapshot {
                        form method="post" action="/queue" class="queue_form" {
                            input type="hidden" name="dir" value=(snapshot.dir.to_string_lossy());
                            label {
                                input type="checkbox" name="include_subdirs" value="true" {}
                                " include sub-directories"
                            }
                            button type="submit" { "set queue from this folder" }
                        }
                    }

                    list {
                        @if let Some(snapshot) = displayed_snapshot {
                            @if let Some(parent) = parent {
                                a class="list_item list_item__folder" href=(browse_href(&parent.dir)) { ".." }
                            }
                            @for directory in &snapshot.subdirs {
                                a class="list_item list_item__folder" href=(browse_href(directory)) { (format!("/{}", display_name(directory))) }
                            }
                            @for file in &snapshot.files {
                                form method="post" action="/play" {
                                    input type="hidden" name="path" value=(file.to_string_lossy());
                                    button type="submit" class="list_item list_item__file" { (display_name(file)) }
                                }
                            }
                        }
                    }
                }

                span class="controls__container" {
                    @if player_state.is_some_and(|state| matches!(state.playback_state, PlaybackState::Playing)) {
                        span class="controls__button_row" {
                            form method="post" action="/previous" {
                                button type="submit" class="control_button" { "⏮" }
                            }
                            form method="post" action="/pause" {
                                button type="submit" class="control_button" { "❚❚" }
                            }
                            form method="post" action="/next" {
                                button type="submit" class="control_button" { "⏭" }
                            }
                        }
                    } @else {
                        span class="controls__button_row" {
                            form method="post" action="/previous" {
                                button type="submit" class="control_button" { "⏮" }
                            }
                            form method="post" action="/play" {
                                button type="submit" class="control_button" { "▶︎" }
                            }
                            form method="post" action="/next" {
                                button type="submit" class="control_button" { "⏭" }
                            }
                        }
                    }

                    // form method="post" action="/seek" {
                    //     input type="range" id="location" name="position_secs" min="0" max="100" value="0" onchange="this.form.submit()" {}
                    // }


                    form method="post" action="/seek" {
                        input type="range" id="location" name="position_secs" min="0" max="100" value="0" {}
                        button type="submit" { "seek" }
                    }


                    input type="range" id="volume" name="volume" min="0" max="100" value="50" {}
                }
            }
        }
    }
}
