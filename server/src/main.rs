mod accounts;
mod content;
mod directory;
mod feedback;
mod lobby;
mod matchmaking;
mod progress;
mod ranking;
mod session;
#[cfg(test)]
mod tests;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use anyhow::Result;
use axum::Router;
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::serve::ListenerExt;
use clap::Parser;
use star_racer_protocol::{DEFAULT_PORT, MAX_MESSAGE_SIZE, WS_PATH};
use tokio::net::TcpListener;
use tracing::{Instrument, info, info_span, warn};

use crate::accounts::Accounts;
use crate::content::CarTuningFile;
use crate::directory::Directory;
use crate::feedback::Feedback;
use crate::lobby::Timings;
use crate::progress::Progress;
use crate::ranking::Ratings;

#[derive(Parser)]
struct Args {
    /// Game data directory, holding `car.ron` and `tracks/` [default: `server/data` in the
    /// repository, else `data` beside the program, as the server downloaded from the site has it].
    #[arg(long)]
    data_dir: Option<PathBuf>,
    /// Port to listen on. The deployment machine already runs another game on the default one (see
    /// docs/deploy.md).
    #[arg(long, default_value_t = DEFAULT_PORT)]
    port: u16,
    /// Address to listen on: the loopback by default, behind a reverse proxy. `0.0.0.0` lets other
    /// computers join directly, by typing `ws://<this computer's address>:<port>` in the game's
    /// SETTINGS (a server downloaded from the site, docs/releases.md).
    #[arg(long, default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    listen: IpAddr,
    /// Where the ranked ladder is kept. Outside the data directory, which a deployment replaces:
    /// relative to the working directory, the service's home on the deployment machine.
    #[arg(long, default_value = "ratings.ron")]
    ratings: PathBuf,
    /// Where the player accounts are kept, beside the ladder and for the same reason.
    #[arg(long, default_value = "accounts.ron")]
    accounts: PathBuf,
    /// Where every player's experience is kept, beside the ladder and for the same reason.
    #[arg(long, default_value = "progress.ron")]
    progress: PathBuf,
    /// Where the circuits players publish from the level editor are kept, one file each, beside the
    /// ladder and for the same reason.
    #[arg(long, default_value = "user-tracks")]
    user_tracks: PathBuf,
    /// Where feedback from the beta panel is kept, beside the ladder and for the same reason
    /// (docs/feedback.md).
    #[arg(long, default_value = "feedback.log")]
    feedback: PathBuf,
    /// The netrc file holding the login of the mailbox feedback is mailed from, its `machine` the SMTP
    /// server; without it feedback is only kept (docs/feedback.md).
    #[arg(long, default_value = "mail.netrc")]
    mail_login: PathBuf,
}

/// Where the game data is when `--data-dir` does not say: `server/data` from the repository's root,
/// or `data` beside the program, the layout of the server offered for download (since v0.1.14).
fn default_data_dir() -> PathBuf {
    let repository = PathBuf::from("server/data");
    if repository.is_dir() {
        return repository;
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join("data")))
        .filter(|beside| beside.is_dir())
        .unwrap_or(repository)
}

/// Plain text: in production, on the loopback, the reverse proxy terminates TLS and forwards here.
fn listen_addr(ip: IpAddr, port: u16) -> SocketAddr {
    SocketAddr::new(ip, port)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Logs at `info` unless `RUST_LOG` says otherwise, e.g. `RUST_LOG=debug`.
    star_racer_logger::init();
    let args = Args::parse();

    let data_dir = args.data_dir.clone().unwrap_or_else(default_data_dir);
    info!(data = %data_dir.display(), "game data");
    let tracks = content::load_tracks(&data_dir)?;
    for (key, loaded) in &tracks {
        info!(
            key,
            name = loaded.track.name(),
            length = %format_args!("{:.1} m", loaded.track.length()),
            "track loaded"
        );
    }
    let (tuning_file, tuning) =
        CarTuningFile::load(data_dir.join(content::CAR_TUNING_FILE), &tracks)?;
    let ratings = Ratings::load(args.ratings)?;
    let progress = Progress::load(args.progress)?;
    let accounts = Accounts::load(args.accounts)?;
    let directory = Directory::spawn(
        tracks,
        content::UserTracks::kept_in(args.user_tracks),
        tuning_file.watch(tuning),
        Timings::default(),
        ratings,
        progress,
        accounts,
    );

    let listener = TcpListener::bind(listen_addr(args.listen, args.port)).await?;
    info!(addr = %listener.local_addr()?, path = WS_PATH, "server listening");
    // Send small messages at once instead of batching them: a snapshot held back by Nagle's
    // algorithm is a snapshot the player sees late.
    let listener = listener.tap_io(|stream| {
        if let Err(error) = stream.set_nodelay(true) {
            warn!("cannot disable Nagle's algorithm: {error}");
        }
    });

    let feedback = Feedback::new(args.feedback, args.mail_login);
    axum::serve(listener, router(directory, feedback))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            info!("shutdown requested");
        })
        .await?;

    info!("server stopped");
    Ok(())
}

fn router(directory: Directory, feedback: Feedback) -> Router {
    Router::new()
        .route(WS_PATH, get(upgrade))
        .with_state((directory, feedback))
}

async fn upgrade(
    ws: WebSocketUpgrade,
    State((directory, feedback)): State<(Directory, Feedback)>,
) -> Response {
    ws.max_message_size(MAX_MESSAGE_SIZE).on_upgrade(|socket| {
        async move {
            if let Err(error) = session::run(socket, directory, feedback).await {
                warn!("session failed: {error:#}");
            }
        }
        .instrument(info_span!("session"))
    })
}
