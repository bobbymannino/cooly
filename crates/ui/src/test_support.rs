//! Helpers shared by the views' tests.

use std::{
    io::{Read as _, Write as _},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

use cooly_api::{Client, Credentials};
use cooly_store::{Keychain, Store};
use tempfile::TempDir;

/// A client for a local server that answers every request with `status` and `body`, along with a count of the
/// requests it has answered.
pub fn serve(status: &'static str, body: &'static str) -> (Client, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let answered = requests.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: \
                 close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            answered.fetch_add(1, Ordering::SeqCst);
        }
    });
    let client = Client::new(Credentials::parse(&format!("http://{address}"), "1|token").unwrap());
    (client, requests)
}

/// The keychain is never touched: only preferences are read and written, and those aren't encrypted.
pub fn store() -> (TempDir, Arc<Store>) {
    let dir = TempDir::new().unwrap();
    let store = Arc::new(Store::new(dir.path(), Keychain));
    (dir, store)
}
