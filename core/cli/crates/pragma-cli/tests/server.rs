use pragma_cli::server::{self, Event, Mode, Site};
use std::collections::HashMap;
use std::io::Read;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dist/client")
}

async fn start(mode: Mode) -> (SocketAddr, Arc<Site>) {
    let site = Site::new(fixtures(), mode);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(server::serve(listener, site.clone()));
    (addr, site)
}

struct Res {
    status: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Res {
    fn header(&self, name: &str) -> &str {
        self.headers.get(name).map(String::as_str).unwrap_or("")
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

async fn request(addr: SocketAddr, method: &str, path: &str, extra: &[(&str, &str)]) -> Res {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut head = format!("{method} {path} HTTP/1.1\r\nhost: localhost\r\nconnection: close\r\n");
    for (k, v) in extra {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let mut lines = head.lines();
    let status = lines.next().unwrap().split(' ').nth(1).unwrap().parse().unwrap();
    let headers = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    Res { status, headers, body: raw[split + 4..].to_vec() }
}

async fn get(addr: SocketAddr, path: &str) -> Res {
    request(addr, "GET", path, &[]).await
}

fn fixture(rel: &str) -> String {
    std::fs::read_to_string(fixtures().join(rel)).unwrap()
}

#[tokio::test]
async fn serves_static_files_with_mime() {
    let (addr, _) = start(Mode::Serve).await;
    let cases = [
        ("/", "text/html; charset=utf-8"),
        ("/assets/app-3f2a91.css", "text/css; charset=utf-8"),
        ("/assets/core/boot-9c1d22.js", "text/javascript; charset=utf-8"),
        ("/_pragma/manifest.json", "application/json"),
        ("/logo.png", "image/png"),
    ];
    for (path, mime) in cases {
        let res = get(addr, path).await;
        assert_eq!(res.status, 200, "{path}");
        assert_eq!(res.header("content-type"), mime, "{path}");
    }
    assert_eq!(get(addr, "/").await.text(), fixture("index.html"));
}

#[tokio::test]
async fn routes_by_manifest() {
    let (addr, _) = start(Mode::Serve).await;
    let res = get(addr, "/shop/42").await;
    assert_eq!((res.status, res.text()), (200, fixture("shop/[id].html")));
    let res = get(addr, "/shop/caf%C3%A9").await;
    assert_eq!((res.status, res.text()), (200, fixture("shop/[id].html")));
    // static beats param, even when listed later in the manifest
    assert_eq!(get(addr, "/shop/new").await.text(), fixture("shop/new.html"));
    assert_eq!(get(addr, "/shop").await.text(), fixture("shop/index.html"));
    assert_eq!(get(addr, "/shop/").await.text(), fixture("shop/index.html"));
    // ssr without a node server: falls back to the prebuilt HTML
    assert_eq!(get(addr, "/about").await.text(), fixture("about/index.html"));
}

#[tokio::test]
async fn unknown_paths_get_404_page() {
    let (addr, _) = start(Mode::Serve).await;
    for path in ["/nothing", "/shop/1/extra", "/assets/gone.js", "/a/../../Cargo.toml"] {
        let res = get(addr, path).await;
        assert!(res.status == 404 || res.status == 400, "{path}: {}", res.status);
        if res.status == 404 {
            assert_eq!(res.text(), fixture("_404.html"), "{path}");
            assert_eq!(res.header("content-type"), "text/html; charset=utf-8");
        }
    }
    assert_eq!(request(addr, "POST", "/", &[]).await.status, 405);
}

#[tokio::test]
async fn serve_cache_headers_and_conditional_requests() {
    let (addr, _) = start(Mode::Serve).await;
    let css = get(addr, "/assets/app-3f2a91.css").await;
    assert_eq!(css.header("cache-control"), "public, max-age=31536000, immutable");
    let html = get(addr, "/shop/7").await;
    assert_eq!(html.header("cache-control"), "no-cache");
    assert_eq!(get(addr, "/nothing").await.header("cache-control"), "no-cache");

    let etag = html.header("etag").to_owned();
    assert!(!etag.is_empty());
    let res = request(addr, "GET", "/shop/7", &[("if-none-match", &etag)]).await;
    assert_eq!(res.status, 304);
    assert!(res.body.is_empty());

    let head = request(addr, "HEAD", "/", &[]).await;
    assert_eq!(head.status, 200);
    assert!(head.body.is_empty());
}

#[tokio::test]
async fn serve_compresses_text() {
    let (addr, _) = start(Mode::Serve).await;
    let original = fixture("assets/app-3f2a91.css");

    let br = request(addr, "GET", "/assets/app-3f2a91.css", &[("accept-encoding", "gzip, deflate, br")]).await;
    assert_eq!(br.header("content-encoding"), "br");
    assert_eq!(br.header("vary"), "accept-encoding");
    let mut decoded = String::new();
    brotli::Decompressor::new(&br.body[..], 4096).read_to_string(&mut decoded).unwrap();
    assert_eq!(decoded, original);

    let gz = request(addr, "GET", "/assets/app-3f2a91.css", &[("accept-encoding", "gzip")]).await;
    assert_eq!(gz.header("content-encoding"), "gzip");
    let mut decoded = String::new();
    flate2::read::GzDecoder::new(&gz.body[..]).read_to_string(&mut decoded).unwrap();
    assert_eq!(decoded, original);

    let plain = get(addr, "/assets/app-3f2a91.css").await;
    assert_eq!(plain.header("content-encoding"), "");
    assert_eq!(plain.text(), original);
    let png = request(addr, "GET", "/logo.png", &[("accept-encoding", "br")]).await;
    assert_eq!(png.header("content-encoding"), "");
}

#[tokio::test]
async fn dev_disables_cache_and_injects_client() {
    let (addr, _) = start(Mode::Dev).await;
    let res = get(addr, "/shop/1").await;
    assert_eq!(res.header("cache-control"), "no-store");
    assert!(res.text().contains("<script type=\"module\" src=\"/_pragma/dev.js\"></script></head>"));
    assert_eq!(get(addr, "/assets/app-3f2a91.css").await.header("cache-control"), "no-store");
    let missing = get(addr, "/nothing").await;
    assert_eq!(missing.status, 404);
    assert!(missing.text().contains("/_pragma/dev.js"));

    let js = get(addr, "/_pragma/dev.js").await;
    assert_eq!(js.status, 200);
    assert_eq!(js.header("content-type"), "text/javascript; charset=utf-8");
    assert!(js.text().contains("new WebSocket("));
    assert!(js.text().contains("/_pragma/reload"));

    let logs = r#"{"logs":[{"severityText":"INFO","body":"hi"}]}"#;
    let len = logs.len().to_string();
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let head = format!("POST /_otel HTTP/1.1\r\nhost: x\r\ncontent-length: {len}\r\nconnection: close\r\n\r\n{logs}");
    stream.write_all(head.as_bytes()).await.unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).await.unwrap();
    assert!(raw.starts_with("HTTP/1.1 204"), "{raw}");

    let (serve_addr, _) = start(Mode::Serve).await;
    assert_eq!(get(serve_addr, "/_pragma/dev.js").await.status, 404);
}

const WS_HANDSHAKE: &[u8] = b"GET /_pragma/reload HTTP/1.1\r\nhost: localhost\r\nupgrade: websocket\r\nconnection: Upgrade\r\nsec-websocket-version: 13\r\nsec-websocket-key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n";

async fn ws_connect(addr: SocketAddr) -> TcpStream {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(WS_HANDSHAKE).await.unwrap();
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        head.push(stream.read_u8().await.unwrap());
    }
    let head = String::from_utf8_lossy(&head).to_ascii_lowercase();
    assert!(head.starts_with("http/1.1 101"), "{head}");
    assert!(head.contains("sec-websocket-accept: s3pplmbitxaq9kygzzhzrbk+xoo="), "{head}");
    stream
}

/// Next text frame (unmasked, from the server) as `serde_json::Value`.
async fn ws_message(stream: &mut TcpStream) -> serde_json::Value {
    let read = async {
        let op = stream.read_u8().await.unwrap();
        assert_eq!(op, 0x81, "text frame");
        let len = match stream.read_u8().await.unwrap() {
            126 => stream.read_u16().await.unwrap() as usize,
            127 => stream.read_u64().await.unwrap() as usize,
            n => n as usize,
        };
        let mut payload = vec![0; len];
        stream.read_exact(&mut payload).await.unwrap();
        serde_json::from_slice(&payload).unwrap()
    };
    tokio::time::timeout(Duration::from_secs(5), read).await.expect("no WebSocket message")
}

#[tokio::test]
async fn websocket_emits_reload_events() {
    let (addr, site) = start(Mode::Dev).await;
    let mut stream = ws_connect(addr).await;
    assert_eq!(ws_message(&mut stream).await["type"], "hello");

    site.send(Event::Css);
    assert_eq!(ws_message(&mut stream).await["type"], "css");
    site.send(Event::Reload);
    assert_eq!(ws_message(&mut stream).await["type"], "reload");
    let errors = r#"[{"file":"routes/index.html","line":1}]"#;
    site.send(Event::Error(errors.into()));
    assert_eq!(ws_message(&mut stream).await, serde_json::json!({ "type": "error", "data": errors }));

    // a client connecting after a failed build gets the error right away
    let mut late = ws_connect(addr).await;
    assert_eq!(ws_message(&mut late).await["type"], "hello");
    assert_eq!(ws_message(&mut late).await["type"], "error");

    // close frame from the client (masked, empty) gets a close frame back
    late.write_all(&[0x88, 0x80, 1, 2, 3, 4]).await.unwrap();
    let mut reply = [0u8; 2];
    late.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply, [0x88, 0]);

    // plain GET (old EventSource client): no stream that would hold a connection
    assert_eq!(get(addr, "/_pragma/reload").await.status, 426);
}

#[tokio::test]
async fn many_reload_sockets_leave_http_working() {
    let (addr, site) = start(Mode::Dev).await;
    let mut sockets = Vec::new();
    for _ in 0..12 {
        let mut s = ws_connect(addr).await;
        ws_message(&mut s).await;
        sockets.push(s);
    }
    assert_eq!(get(addr, "/_pragma/dev.js").await.status, 200);
    site.send(Event::Reload);
    for s in &mut sockets {
        assert_eq!(ws_message(s).await["type"], "reload");
    }
}

async fn fake_node() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 1024];
                while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    let n = stream.read(&mut chunk).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                }
                let line = String::from_utf8_lossy(&buf).lines().next().unwrap().to_owned();
                let (mime, body) = if line.contains("/about") {
                    ("text/html", "<html><head></head><body>ssr</body></html>".to_owned())
                } else {
                    ("application/json", format!("{{\"data\":\"{line}\"}}"))
                };
                let res = format!("HTTP/1.1 200 OK\r\ncontent-type: {mime}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
                let _ = stream.write_all(res.as_bytes()).await;
            });
        }
    });
    port
}

#[tokio::test]
async fn proxies_server_paths_to_node() {
    let (addr, site) = start(Mode::Dev).await;
    site.set_backend(Some(fake_node().await));
    let data = get(addr, "/_data/shop/1?x=2").await;
    assert_eq!(data.text(), r#"{"data":"GET /_data/shop/1?x=2 HTTP/1.1"}"#);
    assert_eq!(request(addr, "POST", "/_action/shop/1/save", &[("content-length", "0")]).await.status, 200);
    let ssr = get(addr, "/about").await;
    assert_eq!(ssr.text(), "<html><head><script type=\"module\" src=\"/_pragma/dev.js\"></script></head><body>ssr</body></html>");
    assert_eq!(get(addr, "/shop/1").await.text(), r#"{"data":"GET /shop/1 HTTP/1.1"}"#);
    // The prebuilt HTML the router fetches is a file, even when it matches the SSR route /shop/:id.
    assert!(get(addr, "/shop/index.html").await.text().contains("<title>Shop</title>"));

    site.set_backend(None);
    assert!(get(addr, "/about").await.text().contains("<h1>about</h1>"));
}

#[tokio::test]
async fn negotiates_markdown_in_serve_and_dev() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(root.join("index.html"), "<h1>Home</h1>").unwrap();
    std::fs::write(root.join("index.md"), "# Home\n").unwrap();
    std::fs::create_dir_all(root.join("about")).unwrap();
    std::fs::write(root.join("about/index.html"), "<h1>About</h1>").unwrap();
    std::fs::write(root.join("about.md"), "# About\n").unwrap();
    std::fs::create_dir_all(root.join("plain")).unwrap();
    std::fs::write(root.join("plain/index.html"), "<h1>Plain</h1>").unwrap();

    for mode in [Mode::Serve, Mode::Dev] {
        let site = Site::new(root.to_path_buf(), mode);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(server::serve(listener, site));

        let md = request(addr, "GET", "/", &[("accept", "text/markdown, text/html")]).await;
        assert_eq!(md.status, 200);
        assert_eq!(md.header("content-type"), "text/markdown; charset=utf-8");
        assert!(md.header("vary").to_ascii_lowercase().contains("accept"), "{}", md.header("vary"));
        assert_eq!(md.text(), "# Home\n");

        let about = request(addr, "GET", "/about", &[("accept", "text/markdown;q=0.9, text/html;q=0.8")]).await;
        assert_eq!(about.status, 200);
        assert_eq!(about.header("content-type"), "text/markdown; charset=utf-8");
        assert_eq!(about.text(), "# About\n");

        let html = request(addr, "GET", "/", &[("accept", "text/html, text/markdown;q=0.5")]).await;
        assert_eq!(html.status, 200);
        assert_eq!(html.header("content-type"), "text/html; charset=utf-8");
        assert!(html.header("vary").to_ascii_lowercase().contains("accept"), "{}", html.header("vary"));
        assert!(html.text().contains("<h1>Home</h1>"));

        let head = request(addr, "HEAD", "/", &[("accept", "text/markdown")]).await;
        assert_eq!(head.status, 200);
        assert_eq!(head.header("content-type"), "text/markdown; charset=utf-8");
        assert!(head.body.is_empty());

        let plain = request(addr, "GET", "/plain", &[("accept", "text/markdown")]).await;
        assert_eq!(plain.status, 200);
        assert_eq!(plain.header("content-type"), "text/html; charset=utf-8");
        assert!(!plain.header("vary").to_ascii_lowercase().contains("accept"), "{}", plain.header("vary"));
    }
}

#[tokio::test]
async fn serve_uses_precompressed_sidecar_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("assets")).unwrap();
    let raw = "console.log('hello');\n".repeat(40);
    let br_sidecar = b"PRECOMPRESSED-BR-SENTINEL".to_vec();
    let gz_sidecar = b"PRECOMPRESSED-GZ-SENTINEL".to_vec();
    std::fs::write(root.join("assets/app.js"), &raw).unwrap();
    std::fs::write(root.join("assets/app.js.br"), &br_sidecar).unwrap();
    std::fs::write(root.join("assets/app.js.gz"), &gz_sidecar).unwrap();
    // File without sidecars still falls back to in-memory precompression.
    std::fs::write(root.join("assets/fallback.js"), &raw).unwrap();

    let site = Site::new(root.to_path_buf(), Mode::Serve);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(server::serve(listener, site));

    let br = request(addr, "GET", "/assets/app.js", &[("accept-encoding", "gzip, br")]).await;
    assert_eq!(br.status, 200);
    assert_eq!(br.header("content-encoding"), "br");
    assert_eq!(br.header("content-type"), "text/javascript; charset=utf-8");
    assert_eq!(br.header("vary"), "accept-encoding");
    assert_eq!(br.body, br_sidecar);

    let gz = request(addr, "GET", "/assets/app.js", &[("accept-encoding", "br;q=0, gzip")]).await;
    assert_eq!(gz.status, 200);
    assert_eq!(gz.header("content-encoding"), "gzip");
    assert_eq!(gz.header("content-type"), "text/javascript; charset=utf-8");
    assert_eq!(gz.header("vary"), "accept-encoding");
    assert_eq!(gz.body, gz_sidecar);

    let plain = get(addr, "/assets/app.js").await;
    assert_eq!(plain.status, 200);
    assert_eq!(plain.header("content-encoding"), "");
    assert_eq!(plain.header("vary"), "accept-encoding");
    assert_eq!(plain.text(), raw);

    let fallback = request(addr, "GET", "/assets/fallback.js", &[("accept-encoding", "br")]).await;
    assert_eq!(fallback.status, 200);
    assert_eq!(fallback.header("content-encoding"), "br");
    let mut decoded = String::new();
    brotli::Decompressor::new(&fallback.body[..], 4096).read_to_string(&mut decoded).unwrap();
    assert_eq!(decoded, raw);
}

fn routed_app(name: &str, middleware: bool) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pragma-routing-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("client/_pragma")).unwrap();
    std::fs::create_dir_all(dir.join("client/assets")).unwrap();
    std::fs::create_dir_all(dir.join("server")).unwrap();
    std::fs::write(dir.join("client/_pragma/manifest.json"), r#"[{"pattern":"/","html":"/index.html","ssr":false}]"#).unwrap();
    std::fs::write(dir.join("client/index.html"), "<h1>static home</h1>").unwrap();
    std::fs::write(dir.join("client/assets/app.css"), "body{}").unwrap();
    let routing = format!(r#"{{"middleware":{middleware},"endpoints":["/files/:name"]}}"#);
    std::fs::write(dir.join("server/routes.json"), routing).unwrap();
    dir
}

async fn start_at(client: PathBuf) -> (SocketAddr, Arc<Site>) {
    let site = Site::new(client, Mode::Dev);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(server::serve(listener, site.clone()));
    site.set_backend(Some(fake_node().await));
    (addr, site)
}

#[tokio::test]
async fn proxies_endpoints_with_any_method() {
    let dir = routed_app("endpoints", false);
    let (addr, _site) = start_at(dir.join("client")).await;
    assert_eq!(get(addr, "/files/a.pdf").await.text(), r#"{"data":"GET /files/a.pdf HTTP/1.1"}"#);
    let put = request(addr, "PUT", "/files/a.pdf", &[("content-length", "0")]).await;
    assert_eq!(put.text(), r#"{"data":"PUT /files/a.pdf HTTP/1.1"}"#);
    assert!(get(addr, "/").await.text().contains("static home"));
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn middleware_sends_everything_but_assets_to_node() {
    let dir = routed_app("middleware", true);
    let (addr, _site) = start_at(dir.join("client")).await;
    assert_eq!(get(addr, "/").await.text(), r#"{"data":"GET / HTTP/1.1"}"#);
    assert_eq!(get(addr, "/index.html").await.text(), r#"{"data":"GET /index.html HTTP/1.1"}"#);
    assert_eq!(get(addr, "/missing").await.text(), r#"{"data":"GET /missing HTTP/1.1"}"#);
    assert_eq!(get(addr, "/assets/app.css").await.text(), "body{}");
    let _ = std::fs::remove_dir_all(dir);
}
