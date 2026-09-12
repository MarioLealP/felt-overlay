use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::command::Command;

pub fn default_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        if !runtime_dir.is_empty() {
            return PathBuf::from(runtime_dir).join("felt-overlay.sock");
        }
    }
    // Fallback: check /run/user/<uid> via std / /proc
    if let Ok(status) = fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("Uid:") {
                if let Some(uid_str) = line.split_whitespace().nth(1) {
                    let fallback = PathBuf::from(format!("/run/user/{}/felt-overlay.sock", uid_str));
                    if fallback.parent().map(|p| p.exists()).unwrap_or(false) {
                        return fallback;
                    }
                }
            }
        }
    }
    PathBuf::from("/tmp/felt-overlay.sock")
}

pub struct TriggerSocket {
    path: PathBuf,
    shutdown: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl TriggerSocket {
    pub fn bind<P: AsRef<Path>>(
        path: P,
        cmd_tx: Sender<Command>,
    ) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Check if existing socket is active
        if path.exists() {
            if UnixStream::connect(&path).is_ok() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    format!("Another felt-overlay instance is already listening on {:?}", path),
                ));
            } else {
                // Stale socket, remove it
                let _ = fs::remove_file(&path);
            }
        }

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(&path)?;

        // Restrict permissions to user-only (0600)
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));

        listener.set_nonblocking(true)?;

        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();
        let path_clone = path.clone();

        let thread_handle = thread::spawn(move || {
            while !shutdown_clone.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        handle_client(stream, &cmd_tx);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(e) => {
                        if !shutdown_clone.load(Ordering::Relaxed) {
                            eprintln!("Socket accept error: {}", e);
                        }
                        break;
                    }
                }
            }
            let _ = fs::remove_file(&path_clone);
        });

        Ok(Self {
            path,
            shutdown,
            thread_handle: Some(thread_handle),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TriggerSocket {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = fs::remove_file(&self.path);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

fn handle_client(mut stream: UnixStream, cmd_tx: &Sender<Command>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

    let mut reader = BufReader::new(stream.try_clone().unwrap_or_else(|_| stream.try_clone().unwrap()));
    let mut line = String::new();

    while let Ok(n) = reader.read_line(&mut line) {
        if n == 0 {
            break;
        }
        if let Some(cmd) = Command::parse(&line) {
            let _ = cmd_tx.send(cmd);
            let _ = writeln!(stream, "ok");
            let _ = stream.flush();
        } else if !line.trim().is_empty() {
            let _ = writeln!(stream, "error: unknown command, expected show|hide|toggle|status|quit");
            let _ = stream.flush();
        }
        line.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_trigger() {
        let dir = std::env::temp_dir().join(format!("felt_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let sock_path = dir.join("test.sock");

        let (tx, rx) = channel();
        let socket = TriggerSocket::bind(&sock_path, tx).expect("bind socket");

        assert!(sock_path.exists());

        // Connect and send "show\n"
        {
            let mut client = UnixStream::connect(&sock_path).expect("connect client");
            client.write_all(b"show\n").expect("write show");
            let mut response = String::new();
            let mut reader = BufReader::new(client);
            reader.read_line(&mut response).expect("read response");
            assert_eq!(response.trim(), "ok");
        }

        assert_eq!(rx.recv_timeout(Duration::from_secs(1)), Ok(Command::Show));

        // Connect and send "hide\n"
        {
            let mut client = UnixStream::connect(&sock_path).expect("connect client");
            client.write_all(b"hide\n").expect("write hide");
            let mut response = String::new();
            let mut reader = BufReader::new(client);
            reader.read_line(&mut response).expect("read response");
            assert_eq!(response.trim(), "ok");
        }

        assert_eq!(rx.recv_timeout(Duration::from_secs(1)), Ok(Command::Hide));

        // Drop socket and verify file removed
        drop(socket);
        assert!(!sock_path.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
