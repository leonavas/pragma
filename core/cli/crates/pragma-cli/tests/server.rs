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
    assert!(js.text().contains("EventSource('/_pragma/reload')"));

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

async fn read_until(stream: &mut TcpStream, buf: &mut String, needle: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !buf.contains(needle) {
        let mut chunk = [0u8; 1024];
        let n = tokio::time::timeout_at(deadline, stream.read(&mut chunk))
            .await
            .unwrap_or_else(|_| panic!("expected {needle:?}, got {buf:?}"))
            .unwrap();
        assert!(n > 0, "connection closed; got {buf:?}");
        buf.push_str(&String::from_utf8_lossy(&chunk[..n]));
    }
}

#[tokio::test]
async fn sse_emits_reload_events() {
    let (addr, site) = start(Mode::Dev).await;
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(b"GET /_pragma/reload HTTP/1.1\r\nhost: localhost\r\n\r\n").await.unwrap();
    let mut buf = String::new();
    read_until(&mut stream, &mut buf, "event: hello\n").await;
    assert!(buf.contains("content-type: text/event-stream"));

    site.send(Event::Css("/assets/app-new.css".into()));
    read_until(&mut stream, &mut buf, "event: css\ndata: /assets/app-new.css\n\n").await;
    site.send(Event::Reload);
    read_until(&mut stream, &mut buf, "event: reload\ndata: 1\n\n").await;
    site.send(Event::Error(r#"[{"file":"routes/index.html","line":1}]"#.into()));
    read_until(&mut stream, &mut buf, "event: error\ndata: [{\"file\":\"routes/index.html\",\"line\":1}]\n\n").await;

    // a client connecting after a failed build gets the error right away
    let mut late = TcpStream::connect(addr).await.unwrap();
    late.write_all(b"GET /_pragma/reload HTTP/1.1\r\nhost: localhost\r\n\r\n").await.unwrap();
    let mut buf = String::new();
    read_until(&mut late, &mut buf, "event: error\n").await;
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
