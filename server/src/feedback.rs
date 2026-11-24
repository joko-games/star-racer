//! Feedback and bug reports from the beta panel (since v0.1.5, docs/feedback.md): every one is
//! appended to a file beside the ladder, and mailed to the game's mailbox when the machine holds
//! its login.
//!
//! The mail goes out through `curl`, already on the machine, to the SMTP server the login file
//! names: no TLS or mail library in the server, which is cross-compiled without a C toolchain, and
//! the password never on a command line. The login file is a netrc file the user writes on the
//! machine (`machine smtp.example.org login feedback@example.org password ...`), the mailbox
//! sending to itself; without it, feedback is only kept. No address is written in this repository
//! (since v1.0.0-rc1): it is only in that file.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use star_racer_protocol::{FEEDBACK_EVERY_SECONDS, MAX_FEEDBACK_CHARS, MAX_FEEDBACK_CONTEXT_CHARS};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{info, warn};

/// Where feedback goes, shared by every connection.
#[derive(Clone)]
pub struct Feedback(Arc<Places>);

struct Places {
    /// The file every feedback is appended to.
    log: PathBuf,
    /// The netrc file holding the mailbox's login: its `machine` is the SMTP server.
    login: PathBuf,
}

impl Feedback {
    pub fn new(log: PathBuf, login: PathBuf) -> Self {
        Self(Arc::new(Places { log, login }))
    }

    /// The desk one connection hands its feedback to, signed with the player's name.
    pub fn desk(&self, from: String) -> Desk {
        Desk {
            feedback: self.clone(),
            from,
            last: None,
        }
    }
}

/// One connection's way to send feedback: who it is from, and when it last sent some.
pub struct Desk {
    feedback: Feedback,
    from: String,
    last: Option<Instant>,
}

impl Desk {
    /// Keeps `text` and mails it on; `false` when it is empty, too long, or too soon after the
    /// last one from this connection.
    pub async fn submit(&mut self, text: &str, context: &str) -> bool {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > MAX_FEEDBACK_CHARS {
            return false;
        }
        let every = Duration::from_secs(FEEDBACK_EVERY_SECONDS);
        if self.last.is_some_and(|last| last.elapsed() < every) {
            return false;
        }
        self.last = Some(Instant::now());
        let context: String = context.chars().take(MAX_FEEDBACK_CONTEXT_CHARS).collect();
        let entry = entry(&self.from, &context, text, SystemTime::now());

        let places = &self.feedback.0;
        let kept = async {
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&places.log)
                .await?;
            file.write_all(entry.as_bytes()).await?;
            // A tokio file hands the write to a blocking thread and may return before it is
            // done: without this, the entry could still be on its way when the file is next read
            // (the test below failed so, now and then, in a full run).
            file.flush().await
        }
        .await;
        if let Err(error) = kept {
            warn!(file = %places.log.display(), "cannot keep feedback: {error}");
            return false;
        }
        info!(from = %self.from, chars = text.chars().count(), "feedback kept");

        // Mailed on in the background: the player is answered at once.
        let login = places.login.clone();
        let subject = format!("[Star Racer beta] Feedback from {}", self.from);
        tokio::spawn(async move {
            match mail(&login, &subject, &entry).await {
                Ok(true) => info!("feedback mailed"),
                Ok(false) => {}
                Err(error) => warn!("cannot mail feedback: {error}"),
            }
        });
        true
    }
}

/// One feedback as kept in the file and mailed: a header line, the context, the text.
fn entry(from: &str, context: &str, text: &str, at: SystemTime) -> String {
    let seconds = at.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
    format!(
        "=== {} UTC  from {from}\n{context}\n\n{text}\n\n",
        utc(seconds)
    )
}

/// `2026-11-23 14:05:09`, from seconds since 1970.
fn utc(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    // Days to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}

/// The SMTP server and the login a netrc file names: its first `machine` and `login`.
fn smtp_login(netrc: &str) -> Option<(String, String)> {
    let mut words = netrc.split_whitespace();
    let (mut machine, mut login) = (None, None);
    while let Some(word) = words.next() {
        match word {
            "machine" if machine.is_none() => machine = words.next().map(str::to_owned),
            "login" if login.is_none() => login = words.next().map(str::to_owned),
            _ => {}
        }
    }
    Some((machine?, login?))
}

/// Mails `body` to the mailbox the netrc file at `login` names, through its SMTP server,
/// over TLS on port 465. `Ok(false)` when there is no such file: feedback is then only kept.
async fn mail(login: &std::path::Path, subject: &str, body: &str) -> anyhow::Result<bool> {
    let Ok(netrc) = tokio::fs::read_to_string(login).await else {
        return Ok(false);
    };
    let Some((server, sender)) = smtp_login(&netrc) else {
        anyhow::bail!("{} names no machine or no login", login.display());
    };
    let message = format!(
        "From: Star Racer <{sender}>\r\nTo: {sender}\r\nSubject: {subject}\r\n\
         MIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\n\
         Content-Transfer-Encoding: 8bit\r\n\r\n{}",
        body.replace("\r\n", "\n").replace('\n', "\r\n")
    );
    let mut curl = Command::new("curl")
        .args(["--silent", "--show-error", "--ssl-reqd", "--max-time", "30"])
        .arg("--url")
        .arg(format!("smtps://{server}:465"))
        .arg("--netrc-file")
        .arg(login)
        .args(["--mail-from", &sender, "--mail-rcpt", &sender, "--upload-file", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = curl.stdin.take() {
        stdin.write_all(message.as_bytes()).await?;
    }
    let output = curl.wait_with_output().await?;
    if !output.status.success() {
        anyhow::bail!(
            "curl: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_written_in_utc() {
        assert_eq!(utc(0), "1970-01-01 00:00:00");
        // 2027-01-01 00:00:00 UTC, the release.
        assert_eq!(utc(1_798_761_600), "2027-01-01 00:00:00");
    }

    #[test]
    fn the_login_file_names_the_server_and_the_sender() {
        let netrc = "machine smtp.example.org\n  login feedback@example.org\n  password secret\n";
        assert_eq!(
            smtp_login(netrc),
            Some(("smtp.example.org".into(), "feedback@example.org".into()))
        );
        assert_eq!(smtp_login("password only"), None);
    }

    #[tokio::test]
    async fn feedback_is_kept_and_limited() {
        let dir = std::env::temp_dir().join(format!("star-racer-feedback-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("feedback.log");
        let _ = std::fs::remove_file(&log);
        let feedback = Feedback::new(log.clone(), dir.join("no-login"));
        let mut desk = feedback.desk("Ada".into());
        assert!(!desk.submit("   ", "v0.1.5").await, "empty");
        assert!(!desk.submit(&"x".repeat(MAX_FEEDBACK_CHARS + 1), "v0.1.5").await, "too long");
        assert!(desk.submit("The canyon is great", "v0.1.5 native").await);
        assert!(!desk.submit("Again at once", "v0.1.5").await, "too soon");
        let kept = std::fs::read_to_string(&log).unwrap();
        assert!(kept.contains("from Ada") && kept.contains("The canyon is great"), "{kept}");
        assert!(kept.contains("v0.1.5 native"));
        assert!(!kept.contains("Again at once"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
