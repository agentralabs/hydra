//! Shell task — async shell command execution spawned from conversation.
//! Streams stdout/stderr lines as they arrive.

use tokio::sync::mpsc;

/// Updates from a running shell command.
#[derive(Debug, Clone)]
pub enum ShellUpdate {
    /// A line from stdout.
    Stdout(String),
    /// A line from stderr.
    Stderr(String),
    /// Command finished with exit code.
    ExitCode(i32),
    /// Command failed to start.
    Error(String),
}

/// Spawn a shell command. Returns a receiver for output updates.
pub fn spawn(rt: &tokio::runtime::Runtime, command: String) -> mpsc::Receiver<ShellUpdate> {
    let (tx, rx) = mpsc::channel(256);
    rt.spawn(async move { run(command, tx).await });
    rx
}

async fn run(command: String, tx: mpsc::Sender<ShellUpdate>) {
    use tokio::io::AsyncBufReadExt;

    let result = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let mut child = match result {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(ShellUpdate::Error(format!("Failed to start: {e}"))).await;
            return;
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Stream stdout
    let tx_out = tx.clone();
    let stdout_task = tokio::spawn(async move {
        if let Some(stdout) = stdout {
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if tx_out.send(ShellUpdate::Stdout(line)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Stream stderr
    let tx_err = tx.clone();
    let stderr_task = tokio::spawn(async move {
        if let Some(stderr) = stderr {
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if tx_err.send(ShellUpdate::Stderr(line)).await.is_err() {
                    break;
                }
            }
        }
    });

    let _ = stdout_task.await;
    let _ = stderr_task.await;

    match child.wait().await {
        Ok(status) => {
            let code = status.code().unwrap_or(-1);
            let _ = tx.send(ShellUpdate::ExitCode(code)).await;
        }
        Err(e) => {
            let _ = tx.send(ShellUpdate::Error(format!("Wait failed: {e}"))).await;
        }
    }
}

/// Drain shell updates into the conversation stream.
/// Returns true when the command is done.
pub fn drain_shell(
    rx: &mut mpsc::Receiver<ShellUpdate>,
    stream: &mut crate::stream::ConversationStream,
) -> bool {
    use crate::stream_types::StreamItem;

    while let Ok(update) = rx.try_recv() {
        match update {
            ShellUpdate::Stdout(line) => {
                stream.push(StreamItem::SystemNotification {
                    id: uuid::Uuid::new_v4(),
                    content: line,
                    timestamp: chrono::Utc::now(),
                });
            }
            ShellUpdate::Stderr(line) => {
                stream.push(StreamItem::SystemNotification {
                    id: uuid::Uuid::new_v4(),
                    content: format!("stderr: {line}"),
                    timestamp: chrono::Utc::now(),
                });
            }
            ShellUpdate::ExitCode(code) => {
                if code != 0 {
                    stream.push(StreamItem::SystemNotification {
                        id: uuid::Uuid::new_v4(),
                        content: format!("Exit code: {code}"),
                        timestamp: chrono::Utc::now(),
                    });
                }
                stream.scroll_to_bottom();
                return true;
            }
            ShellUpdate::Error(e) => {
                stream.push(StreamItem::SystemNotification {
                    id: uuid::Uuid::new_v4(),
                    content: format!("Shell error: {e}"),
                    timestamp: chrono::Utc::now(),
                });
                stream.scroll_to_bottom();
                return true;
            }
        }
        stream.scroll_to_bottom();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shell_echo_produces_output() {
        let (tx, mut rx) = mpsc::channel(32);
        run("echo hello".into(), tx).await;

        let mut found_hello = false;
        let mut found_exit = false;
        while let Ok(update) = rx.try_recv() {
            match update {
                ShellUpdate::Stdout(line) if line.contains("hello") => found_hello = true,
                ShellUpdate::ExitCode(0) => found_exit = true,
                _ => {}
            }
        }
        assert!(found_hello, "Should have received 'hello' on stdout");
        assert!(found_exit, "Should have received exit code 0");
    }
}
