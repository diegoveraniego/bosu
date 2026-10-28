//! Optional browser laboratory; the listener only exposes embedded, read-only assets.
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::OnceLock,
};

pub fn open() -> Result<(), String> {
    static URL: OnceLock<String> = OnceLock::new();
    let url = if let Some(url) = URL.get() {
        url.clone()
    } else {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        let url = format!(
            "http://{}/",
            listener.local_addr().map_err(|e| e.to_string())?
        );
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
                let mut request = [0; 4096];
                let Ok(n) = stream.read(&mut request) else {
                    continue;
                };
                let line = String::from_utf8_lossy(&request[..n]);
                let (status, mime, body) = match line.lines().next().unwrap_or("") {
                    "GET / HTTP/1.1" => (
                        "200 OK",
                        "text/html; charset=utf-8",
                        include_str!("motion/index.html"),
                    ),
                    "GET /app.js HTTP/1.1" => (
                        "200 OK",
                        "text/javascript; charset=utf-8",
                        include_str!("motion/app.js"),
                    ),
                    "GET /counter.js HTTP/1.1" => (
                        "200 OK",
                        "text/javascript; charset=utf-8",
                        include_str!("motion/counter.js"),
                    ),
                    _ => ("404 Not Found", "text/plain", "Not found"),
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        let _ = URL.set(url.clone());
        url
    };
    gtk::gio::AppInfo::launch_default_for_uri(&url, None::<&gtk::gio::AppLaunchContext>)
        .map_err(|e| e.to_string())
}
