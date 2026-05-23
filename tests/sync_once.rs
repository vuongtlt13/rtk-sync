use chrono::Utc;
use rtk_sync::config::SyncConfig;
use rtk_sync::state::State;
use rtk_sync::syncer;
use rusqlite::Connection;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread;

fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("lock tests")
}

#[test]
fn once_uploads_events_and_advances_checkpoint() {
    let _guard = test_lock();
    let dir = tempfile::tempdir().expect("create tempdir");
    let db_path = dir.path().join("history.db");
    let state_path = dir.path().join("state.json");
    create_db(&db_path);
    insert_command(&db_path, 1);
    insert_command(&db_path, 2);

    let server = TestServer::start(200, r#"{"accepted":2,"duplicates":0,"max_local_id":2}"#);
    let config = SyncConfig {
        db_path,
        state_path: state_path.clone(),
        endpoint: server.endpoint(),
        token_env: "RTK_SYNC_TOKEN".to_string(),
        token: "test-token".to_string(),
        machine_id: Some("machine-1".to_string()),
        batch_size: 100,
        interval: 60,
        dry_run: false,
    };

    syncer::run_once(&config).expect("run sync once");
    let state = State::load(&state_path).expect("load state");
    let request = server.request_rx.recv().expect("receive request");

    assert_eq!(state.last_synced_id, 2);
    assert!(request.contains("Authorization: Bearer test-token"));
    assert!(request.contains("\"source_id\":\"machine-1:1\""));
    assert!(request.contains("\"source_id\":\"machine-1:2\""));
}

#[test]
fn failed_upload_does_not_advance_checkpoint() {
    let _guard = test_lock();
    let dir = tempfile::tempdir().expect("create tempdir");
    let db_path = dir.path().join("history.db");
    let state_path = dir.path().join("state.json");
    create_db(&db_path);
    insert_command(&db_path, 1);

    let server = TestServer::start(500, r#"{"error":"nope"}"#);
    let config = SyncConfig {
        db_path,
        state_path: state_path.clone(),
        endpoint: server.endpoint(),
        token_env: "RTK_SYNC_TOKEN".to_string(),
        token: "test-token".to_string(),
        machine_id: Some("machine-1".to_string()),
        batch_size: 100,
        interval: 60,
        dry_run: false,
    };

    assert!(syncer::run_once(&config).is_err());
    let state = State::load(&state_path).expect("load state");

    assert_eq!(state.last_synced_id, 0);
}

#[test]
fn dry_run_does_not_require_server_or_advance_checkpoint() {
    let _guard = test_lock();
    let dir = tempfile::tempdir().expect("create tempdir");
    let db_path = dir.path().join("history.db");
    let state_path = dir.path().join("state.json");
    create_db(&db_path);
    insert_command(&db_path, 1);
    insert_command(&db_path, 2);

    let config = SyncConfig {
        db_path,
        state_path: state_path.clone(),
        endpoint: String::new(),
        token_env: "RTK_SYNC_TOKEN".to_string(),
        token: String::new(),
        machine_id: Some("machine-1".to_string()),
        batch_size: 100,
        interval: 60,
        dry_run: true,
    };

    syncer::run_once(&config).expect("run dry-run sync once");

    assert!(!state_path.exists());
}

fn create_db(path: &Path) {
    let conn = Connection::open(path).expect("open db");
    conn.execute(
        "CREATE TABLE commands (
            id INTEGER PRIMARY KEY,
            timestamp TEXT NOT NULL,
            original_cmd TEXT NOT NULL,
            rtk_cmd TEXT NOT NULL,
            input_tokens INTEGER NOT NULL,
            output_tokens INTEGER NOT NULL,
            saved_tokens INTEGER NOT NULL,
            savings_pct REAL NOT NULL,
            exec_time_ms INTEGER DEFAULT 0,
            project_path TEXT DEFAULT ''
        )",
        [],
    )
    .expect("create commands table");
}

fn insert_command(path: &Path, id: i64) {
    let conn = Connection::open(path).expect("open db");
    conn.execute(
        "INSERT INTO commands (id, timestamp, original_cmd, rtk_cmd, input_tokens, output_tokens, saved_tokens, savings_pct, exec_time_ms, project_path)
         VALUES (?1, ?2, 'git status', 'rtk git status', 100, 25, 75, 75.0, 12, '/repo')",
        rusqlite::params![id, Utc::now().to_rfc3339()],
    )
    .expect("insert command");
}

struct TestServer {
    addr: std::net::SocketAddr,
    request_rx: mpsc::Receiver<String>,
}

impl TestServer {
    fn start(status: u16, body: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind server");
        let addr = listener.local_addr().expect("read local addr");
        let (request_tx, request_rx) = mpsc::channel();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept connection");
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            loop {
                let size = stream.read(&mut buffer).expect("read request");
                if size == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..size]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let header_end = request
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|index| index + 4)
                .unwrap_or(request.len());
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())?
                })
                .unwrap_or(0);
            while request.len() < header_end + content_length {
                let size = stream.read(&mut buffer).expect("read request body");
                if size == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..size]);
            }
            let request = String::from_utf8_lossy(&request).to_string();
            request_tx.send(request).expect("send request");
            let response = format!(
                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        });

        Self { addr, request_rx }
    }

    fn endpoint(&self) -> String {
        format!("http://{}/api/rtk/events", self.addr)
    }
}
