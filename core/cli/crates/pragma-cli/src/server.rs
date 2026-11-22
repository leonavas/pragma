use bytes::Bytes;
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::body::Incoming;
use hyper::header::{self, HeaderMap, HeaderValue};
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::client::legacy::{Client, connect::HttpConnector};
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::collections::HashMap;
use std::convert::Infallible;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use pragma_compiler::layout::{MANIFEST, SERVER_ROUTES};

pub type Body = BoxBody<Bytes, std::io::Error>;

const DEV_SCRIPT: &str = "<script type=\"module\" src=\"/_pragma/dev.js\"></script>";
const DEV_JS: &str = include_str!("dev.js");
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Dev,
    Serve,
}

#[derive(Clone, Debug)]
pub enum Event {
    Css,
    Reload,
    Error(String),
}

#[derive(Clone, Debug)]
struct Route {
    segments: Vec<Option<String>>,
    html: String,
    ssr: bool,
    endpoint: bool,
}

#[derive(Default)]
struct Routing {
    routes: Vec<Route>,
    middleware: bool,
}

struct Cached {
    body: Bytes,
    gzip: Option<Bytes>,
    brotli: Option<Bytes>,
    etag: String,
}

pub struct Site {
    root: PathBuf,
    mode: Mode,
    routing: RwLock<Arc<Routing>>,
    cache: HashMap<String, Cached>,
    events: broadcast::Sender<Event>,
    last_error: Mutex<Option<String>>,
    backend: RwLock<Option<u16>>,
    client: Client<HttpConnector, Body>,
    boot_id: String,
}

impl Site {
    /// In `Serve`, loads and precompresses text files (blocking).
    pub fn new(root: PathBuf, mode: Mode) -> Arc<Site> {
        let cache = if mode == Mode::Serve { precompress(&root) } else { HashMap::new() };
        let boot = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let site = Site {
            routing: RwLock::new(Arc::new(load_routing(&root))),
            root,
            mode,
            cache,
            events: broadcast::channel(16).0,
            last_error: Mutex::new(None),
            backend: RwLock::new(None),
            client: Client::builder(TokioExecutor::new()).build_http(),
            boot_id: format!("{boot:x}"),
        };
        Arc::new(site)
    }

    pub fn reload_manifest(&self) {
        *self.routing.write().unwrap() = Arc::new(load_routing(&self.root));
    }

    pub fn set_backend(&self, port: Option<u16>) {
        *self.backend.write().unwrap() = port;
    }

    pub fn send(&self, event: Event) {
        *self.last_error.lock().unwrap() = match &event {
            Event::Error(json) => Some(json.clone()),
            _ => None,
        };
        let _ = self.events.send(event);
    }

    fn backend(&self) -> Option<u16> {
        *self.backend.read().unwrap()
    }

    /// `raw` is the path as requested: split first, then each segment decoded (contract.js), so
    /// `/shop/a%2Fb` is one segment `a/b`.
    fn match_route(&self, raw: &str) -> Option<Route> {
        let parts: Vec<String> = raw.split('/').filter(|s| !s.is_empty()).map(percent_decode).collect::<Option<_>>()?;
        let routing = self.routing.read().unwrap().clone();
        routing
            .routes
            .iter()
            .filter(|r| {
                r.segments.len() == parts.len()
                    && r.segments.iter().zip(&parts).all(|(s, p)| s.as_deref().is_none_or(|s| s == p))
            })
            // static beats param, segment by segment
            .min_by_key(|r| r.segments.iter().map(Option::is_none).collect::<Vec<_>>())
            .cloned()
    }

    fn has_middleware(&self) -> bool {
        self.routing.read().unwrap().middleware
    }

    fn static_file(&self, path: &str) -> Option<String> {
        let rel = path.trim_matches('/');
        let full = self.root.join(rel);
        if full.is_file() {
            return Some(rel.to_owned());
        }
        if full.is_dir() && full.join("index.html").is_file() {
            return Some(if rel.is_empty() { "index.html".into() } else { format!("{rel}/index.html") });
        }
        None
    }

    fn markdown_file(&self, path: &str) -> Option<String> {
        let rel = path.trim_matches('/');
        if is_server_path(path) || rel.rsplit('/').next().is_some_and(|s| s.contains('.')) {
            return None;
        }
        let candidates = if rel.is_empty() { vec!["index.md".to_owned()] } else { vec![format!("{rel}.md"), format!("{rel}/index.md")] };
        candidates.into_iter().find(|c| self.root.join(c).is_file())
    }
}

pub async fn serve(listener: TcpListener, site: Arc<Site>) {
    loop {
        let Ok((stream, _)) = listener.accept().await else { continue };
        let _ = stream.set_nodelay(true);
        let site = site.clone();
        tokio::spawn(async move {
            let service = hyper::service::service_fn(move |req| handle(site.clone(), req));
            let _ = hyper::server::conn::http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .with_upgrades()
                .await;
        });
    }
}

async fn handle(site: Arc<Site>, req: Request<Incoming>) -> Result<Response<Body>, Infallible> {
    let raw = req.uri().path().to_owned();
    if site.mode == Mode::Dev {
        match raw.as_str() {
            "/_pragma/reload" => return Ok(reload_socket(&site, req)),
            "/_pragma/dev.js" => {
                return Ok(response(StatusCode::OK, "text/javascript; charset=utf-8", "no-store", Bytes::from_static(DEV_JS.as_bytes())));
            }
            _ => {}
        }
    }
    let Some(path) = decode_path(&raw) else {
        return Ok(response(StatusCode::BAD_REQUEST, "text/plain; charset=utf-8", "no-store", Bytes::from_static("invalid path".as_bytes())));
    };
    if let Some(port) = site.backend()
        && site.has_middleware()
        && !path.starts_with("/assets/")
        && !path.starts_with("/_pragma/")
    {
        return Ok(proxy(&site, port, req).await);
    }
    let markdown = if req.method() == Method::GET || req.method() == Method::HEAD { site.markdown_file(&path) } else { None };
    if let Some(md) = &markdown
        && prefers_markdown(req.headers())
    {
        let mut res = file_response(&site, md, StatusCode::OK, req.headers()).await;
        if req.method() == Method::HEAD {
            *res.body_mut() = empty();
        }
        vary_accept(&mut res);
        return Ok(res);
    }
    let route = site.match_route(&raw);
    if let Some(port) = site.backend()
        && (is_server_path(&path) || route.as_ref().is_some_and(|r| r.endpoint || r.ssr && !path.ends_with(".html")))
    {
        let mut res = proxy(&site, port, req).await;
        if markdown.is_some() {
            vary_accept(&mut res);
        }
        return Ok(res);
    }
    if path == "/_otel" && req.method() == Method::POST {
        if let Ok(body) = req.into_body().collect().await
            && site.mode == Mode::Dev
        {
            crate::report::print_client_logs(&body.to_bytes());
        }
        let mut res = Response::new(empty());
        *res.status_mut() = StatusCode::NO_CONTENT;
        return Ok(res);
    }
    if req.method() != Method::GET && req.method() != Method::HEAD {
        let mut res = response(StatusCode::METHOD_NOT_ALLOWED, "text/plain; charset=utf-8", "no-store", Bytes::from_static("method not allowed".as_bytes()));
        res.headers_mut().insert(header::ALLOW, HeaderValue::from_static("GET, HEAD"));
        return Ok(res);
    }
    let (rel, status) = match site.static_file(&path) {
        Some(rel) => (rel, StatusCode::OK),
        None => match route {
            Some(r) => (r.html.trim_start_matches('/').to_owned(), StatusCode::OK),
            None => ("_404.html".to_owned(), StatusCode::NOT_FOUND),
        },
    };
    let mut res = file_response(&site, &rel, status, req.headers()).await;
    if req.method() == Method::HEAD {
        *res.body_mut() = empty();
    }
    if markdown.is_some() {
        vary_accept(&mut res);
    }
    Ok(res)
}

fn prefers_markdown(headers: &HeaderMap) -> bool {
    let accept = headers.get(header::ACCEPT).and_then(|v| v.to_str().ok()).unwrap_or("");
    let (mut markdown, mut html) = (0.0f32, 0.0f32);
    for part in accept.split(',') {
        let mut it = part.split(';');
        let name = it.next().unwrap_or("").trim();
        let q = match it.find_map(|p| p.trim().strip_prefix("q=")) {
            Some(q) => q.trim().parse::<f32>().unwrap_or(0.0),
            None => 1.0,
        };
        if name.eq_ignore_ascii_case("text/markdown") {
            markdown = markdown.max(q);
        } else if name.eq_ignore_ascii_case("text/html") {
            html = html.max(q);
        }
    }
    markdown > 0.0 && markdown >= html
}

fn vary_accept(res: &mut Response<Body>) {
    let h = res.headers_mut();
    let value = match h.get(header::VARY).and_then(|v| v.to_str().ok()) {
        Some(v) if v.split(',').any(|t| t.trim().eq_ignore_ascii_case("accept")) => return,
        Some(v) => format!("{v}, accept"),
        None => "accept".to_owned(),
    };
    h.insert(header::VARY, HeaderValue::from_str(&value).unwrap());
}

async fn file_response(site: &Site, rel: &str, status: StatusCode, headers: &HeaderMap) -> Response<Body> {
    let cache_control = match site.mode {
        Mode::Dev => "no-store",
        Mode::Serve if rel.starts_with("assets/") => IMMUTABLE,
        Mode::Serve => "no-cache",
    };
    let mime = mime(rel);
    if let Some(cached) = site.cache.get(rel) {
        if status == StatusCode::OK && etag_matches(headers, &cached.etag) {
            return not_modified(cache_control, &cached.etag);
        }
        let accept = headers.get(header::ACCEPT_ENCODING).and_then(|v| v.to_str().ok()).unwrap_or("");
        let (body, encoding) = match (&cached.brotli, &cached.gzip) {
            (Some(br), _) if accepts(accept, "br") => (br.clone(), Some("br")),
            (_, Some(gz)) if accepts(accept, "gzip") => (gz.clone(), Some("gzip")),
            _ => (cached.body.clone(), None),
        };
        let mut res = response(status, mime, cache_control, body);
        let h = res.headers_mut();
        h.insert(header::ETAG, HeaderValue::from_str(&cached.etag).unwrap());
        if cached.gzip.is_some() || cached.brotli.is_some() {
            h.insert(header::VARY, HeaderValue::from_static("accept-encoding"));
        }
        if let Some(encoding) = encoding {
            h.insert(header::CONTENT_ENCODING, HeaderValue::from_static(encoding));
        }
        return res;
    }
    let full = site.root.join(rel);
    let etag = match site.mode {
        Mode::Serve => tokio::fs::metadata(&full).await.ok().map(|m| etag(&m)),
        Mode::Dev => None,
    };
    if let Some(tag) = &etag
        && status == StatusCode::OK
        && etag_matches(headers, tag)
    {
        return not_modified(cache_control, tag);
    }
    if site.mode == Mode::Serve && !is_sidecar(rel) {
        let accept = headers.get(header::ACCEPT_ENCODING).and_then(|v| v.to_str().ok()).unwrap_or("");
        let mut encoded = None;
        if accepts(accept, "br")
            && let Ok(bytes) = tokio::fs::read(sidecar_path(&full, "br")).await
        {
            encoded = Some((bytes, "br"));
        } else if accepts(accept, "gzip")
            && let Ok(bytes) = tokio::fs::read(sidecar_path(&full, "gz")).await
        {
            encoded = Some((bytes, "gzip"));
        }
        if let Some((bytes, encoding)) = encoded {
            let mut res = response(status, mime, cache_control, Bytes::from(bytes));
            let h = res.headers_mut();
            if let Some(tag) = etag {
                h.insert(header::ETAG, HeaderValue::from_str(&tag).unwrap());
            }
            h.insert(header::VARY, HeaderValue::from_static("accept-encoding"));
            h.insert(header::CONTENT_ENCODING, HeaderValue::from_static(encoding));
            return res;
        }
    }
    let body = match tokio::fs::read(&full).await {
        Ok(body) => body,
        Err(_) if rel == "_404.html" => {
            let page = "<!doctype html><meta charset=\"utf-8\"><title>404</title><p>Page not found.</p>";
            let page = if site.mode == Mode::Dev { inject_dev(page.as_bytes()) } else { Bytes::from_static(page.as_bytes()) };
            return response(StatusCode::NOT_FOUND, "text/html; charset=utf-8", cache_control, page);
        }
        Err(_) => return Box::pin(file_response(site, "_404.html", StatusCode::NOT_FOUND, headers)).await,
    };
    let body = if site.mode == Mode::Dev && mime.starts_with("text/html") { inject_dev(&body) } else { Bytes::from(body) };
    let mut res = response(status, mime, cache_control, body);
    if let Some(tag) = etag {
        res.headers_mut().insert(header::ETAG, HeaderValue::from_str(&tag).unwrap());
    }
    res
}

fn response(status: StatusCode, mime: &str, cache_control: &str, body: Bytes) -> Response<Body> {
    let len = body.len();
    let mut res = Response::new(Full::new(body).map_err(|e| match e {}).boxed());
    *res.status_mut() = status;
    let h = res.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_str(mime).unwrap());
    h.insert(header::CACHE_CONTROL, HeaderValue::from_str(cache_control).unwrap());
    h.insert(header::CONTENT_LENGTH, HeaderValue::from(len));
    res
}

fn not_modified(cache_control: &str, etag: &str) -> Response<Body> {
    let mut res = Response::new(empty());
    *res.status_mut() = StatusCode::NOT_MODIFIED;
    res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_str(cache_control).unwrap());
    res.headers_mut().insert(header::ETAG, HeaderValue::from_str(etag).unwrap());
    res
}

fn empty() -> Body {
    Full::new(Bytes::new()).map_err(|e| match e {}).boxed()
}

fn etag(meta: &std::fs::Metadata) -> String {
    let mtime = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).unwrap_or_default();
    format!("W/\"{:x}-{:x}\"", meta.len(), mtime.as_nanos())
}

fn etag_matches(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|t| t.trim() == etag || t.trim() == "*"))
}

fn accepts(header: &str, encoding: &str) -> bool {
    header.split(',').any(|part| {
        let mut it = part.split(';');
        it.next().is_some_and(|n| n.trim().eq_ignore_ascii_case(encoding))
            && !it.any(|p| p.trim().strip_prefix("q=").and_then(|q| q.parse::<f32>().ok()) == Some(0.0))
    })
}

fn is_server_path(path: &str) -> bool {
    path == "/_otel" || path.starts_with("/_data/") || path == "/_data" || path.starts_with("/_action/")
}

fn inject_dev(html: &[u8]) -> Bytes {
    let text = String::from_utf8_lossy(html);
    if text.contains("/_pragma/dev.js") {
        return Bytes::copy_from_slice(html);
    }
    let out = match text.find("</head>") {
        Some(i) => format!("{}{DEV_SCRIPT}{}", &text[..i], &text[i..]),
        None => format!("{text}{DEV_SCRIPT}"),
    };
    Bytes::from(out)
}

fn decode_path(path: &str) -> Option<String> {
    let path = percent_decode(path)?;
    let bad = path.contains('\0') || path.contains('\\') || path.split('/').any(|s| s == ".." || s == ".");
    (!bad).then_some(path)
}

fn percent_decode(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn segments(pattern: &str) -> Vec<Option<String>> {
    pattern.split('/').filter(|s| !s.is_empty()).map(|s| if s.starts_with(':') { None } else { Some(s.to_owned()) }).collect()
}

fn read_json(path: &Path) -> Option<serde_json::Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Pages from the client manifest; endpoints and middleware from `../server/routes.json`.
fn load_routing(root: &Path) -> Routing {
    let mut routing = Routing::default();
    if let Some(serde_json::Value::Array(items)) = read_json(&root.join(MANIFEST)) {
        routing.routes.extend(items.iter().filter_map(|item| {
            Some(Route {
                segments: segments(item.get("pattern")?.as_str()?),
                html: item.get("html")?.as_str()?.to_owned(),
                ssr: item.get("ssr").and_then(|v| v.as_bool()).unwrap_or(false),
                endpoint: false,
            })
        }));
    }
    if let Some(server) = root.parent().and_then(|dir| read_json(&dir.join(SERVER_ROUTES))) {
        routing.middleware = server.get("middleware").and_then(|v| v.as_bool()).unwrap_or(false);
        let endpoints = server.get("endpoints").and_then(|v| v.as_array()).into_iter().flatten();
        routing.routes.extend(endpoints.filter_map(|p| p.as_str()).map(|p| Route { segments: segments(p), html: String::new(), ssr: false, endpoint: true }));
    }
    routing
}

fn mime(path: &str) -> &'static str {
    let ext = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "map" => "application/json",
        "webmanifest" => "application/manifest+json",
        "txt" => "text/plain; charset=utf-8",
        "md" => "text/markdown; charset=utf-8",
        "xml" => "application/xml",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "wasm" => "application/wasm",
        "pdf" => "application/pdf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    }
}

fn compressible(path: &str) -> bool {
    let mime = mime(path);
    mime.starts_with("text/") || mime.contains("json") || mime.contains("xml") || mime == "application/wasm"
}

fn is_sidecar(rel: &str) -> bool {
    rel.ends_with(".br") || rel.ends_with(".gz")
}

fn sidecar_path(path: &Path, ext: &str) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(".");
    os.push(ext);
    PathBuf::from(os)
}

fn has_sidecar(root: &Path, rel: &str) -> bool {
    let path = root.join(rel);
    sidecar_path(&path, "br").is_file() || sidecar_path(&path, "gz").is_file()
}

fn read_sidecar(path: &Path, ext: &str) -> Option<Bytes> {
    std::fs::read(sidecar_path(path, ext)).ok().map(Bytes::from)
}

// `serve` runs on an immutable dist: compressing once at startup, at max quality and in parallel,
// keeps CPU off the request path and avoids cache/locks; larger files are served from disk.
// Precompressed `.br` / `.gz` sidecars on disk are loaded directly without recompressing.
const MAX_CACHED: u64 = 8 * 1024 * 1024;

fn precompress(root: &Path) -> HashMap<String, Cached> {
    let mut files = Vec::new();
    walk(root, root, &mut files);
    files.retain(|(rel, meta)| !is_sidecar(rel) && meta.len() <= MAX_CACHED && (compressible(rel) || has_sidecar(root, rel)));
    let queue = Mutex::new(files);
    let out = Mutex::new(HashMap::new());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let Some((rel, meta)) = queue.lock().unwrap().pop() else { return };
                    let path = root.join(&rel);
                    let Ok(body) = std::fs::read(&path) else { continue };
                    let can_compress = compressible(&rel) && body.len() >= 256;
                    let gzip = read_sidecar(&path, "gz").or_else(|| can_compress.then(|| gzip(&body)).flatten());
                    let brotli = read_sidecar(&path, "br").or_else(|| can_compress.then(|| brotli(&body)).flatten());
                    let cached = Cached { body: Bytes::from(body), gzip, brotli, etag: etag(&meta) };
                    out.lock().unwrap().insert(rel, cached);
                }
            });
        }
    });
    out.into_inner().unwrap()
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, std::fs::Metadata)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::metadata(&path) else { continue };
        if meta.is_dir() {
            walk(root, &path, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            let rel = rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
            out.push((rel, meta));
        }
    }
}

fn gzip(data: &[u8]) -> Option<Bytes> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    enc.write_all(data).ok()?;
    let out = enc.finish().ok()?;
    (out.len() < data.len()).then(|| Bytes::from(out))
}

fn brotli(data: &[u8]) -> Option<Bytes> {
    let mut out = Vec::new();
    let params = brotli::enc::BrotliEncoderParams { quality: 11, lgwin: 22, ..Default::default() };
    brotli::BrotliCompress(&mut &data[..], &mut out, &params).ok()?;
    (out.len() < data.len()).then(|| Bytes::from(out))
}

/// Live reload message: `{"type":"hello"|"css"|"reload"|"error","data":"…"}` (`error` data is the diagnostics JSON).
fn reload_message(kind: &str, data: &str) -> String {
    serde_json::json!({ "type": kind, "data": data }).to_string()
}

fn event_message(event: &Event) -> String {
    match event {
        Event::Css => reload_message("css", ""),
        Event::Reload => reload_message("reload", ""),
        Event::Error(json) => reload_message("error", json),
    }
}

/// Live reload over a WebSocket: browsers count an `EventSource` against the 6 HTTP/1.1
/// connections per host, so a few open tabs stalled every other request (BUG-S-9).
fn reload_socket(site: &Site, mut req: Request<Incoming>) -> Response<Body> {
    let wants_ws = req.headers().get(header::UPGRADE).and_then(|v| v.to_str().ok()).is_some_and(|v| v.eq_ignore_ascii_case("websocket"));
    let key = req.headers().get(header::SEC_WEBSOCKET_KEY).and_then(|v| v.to_str().ok()).filter(|_| wants_ws);
    let Some(key) = key else {
        let mut res = response(StatusCode::UPGRADE_REQUIRED, "text/plain; charset=utf-8", "no-store", Bytes::from_static(b"WebSocket only"));
        res.headers_mut().insert(header::UPGRADE, HeaderValue::from_static("websocket"));
        return res;
    };
    let accept = ws::accept(key);
    let upgrade = hyper::upgrade::on(&mut req);
    // subscribed before the handshake, so no event between the response and the upgrade is lost
    let mut events = site.events.subscribe();
    let mut first = vec![reload_message("hello", &site.boot_id)];
    if let Some(json) = site.last_error.lock().unwrap().clone() {
        first.push(reload_message("error", &json));
    }
    tokio::spawn(async move {
        let Ok(upgraded) = upgrade.await else { return };
        let (mut rd, mut wr) = tokio::io::split(TokioIo::new(upgraded));
        for message in &first {
            if ws::send(&mut wr, message).await.is_err() {
                return;
            }
        }
        let mut buf = [0u8; 512];
        loop {
            tokio::select! {
                event = events.recv() => {
                    let message = match event {
                        Ok(event) => event_message(&event),
                        Err(broadcast::error::RecvError::Lagged(_)) => event_message(&Event::Reload),
                        Err(broadcast::error::RecvError::Closed) => return,
                    };
                    if ws::send(&mut wr, &message).await.is_err() {
                        return;
                    }
                }
                // the client sends nothing but a close frame (the tab is closing or navigating)
                read = rd.read(&mut buf) => match read {
                    Ok(n) if n > 0 && buf[0] & 0x0f != ws::CLOSE => {}
                    Ok(n) => {
                        if n > 0 {
                            let _ = wr.write_all(&[0x80 | ws::CLOSE, 0]).await;
                        }
                        return;
                    }
                    Err(_) => return,
                },
            }
        }
    });
    let mut res = Response::new(empty());
    *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    let h = res.headers_mut();
    h.insert(header::UPGRADE, HeaderValue::from_static("websocket"));
    h.insert(header::CONNECTION, HeaderValue::from_static("upgrade"));
    h.insert(header::SEC_WEBSOCKET_ACCEPT, HeaderValue::from_str(&accept).unwrap());
    res
}

/// The server half of RFC 6455 that live reload needs: handshake and unmasked text frames.
mod ws {
    use tokio::io::{AsyncWrite, AsyncWriteExt};

    pub const CLOSE: u8 = 0x8;
    const TEXT: u8 = 0x1;
    const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    pub fn accept(key: &str) -> String {
        base64(&sha1(format!("{}{GUID}", key.trim()).as_bytes()))
    }

    pub fn frame(text: &str) -> Vec<u8> {
        let len = text.len();
        let mut out = Vec::with_capacity(len + 10);
        out.push(0x80 | TEXT);
        match len {
            0..=125 => out.push(len as u8),
            126..=0xffff => {
                out.push(126);
                out.extend_from_slice(&(len as u16).to_be_bytes());
            }
            _ => {
                out.push(127);
                out.extend_from_slice(&(len as u64).to_be_bytes());
            }
        }
        out.extend_from_slice(text.as_bytes());
        out
    }

    pub async fn send(w: &mut (impl AsyncWrite + Unpin), text: &str) -> std::io::Result<()> {
        w.write_all(&frame(text)).await?;
        w.flush().await
    }

    pub fn sha1(data: &[u8]) -> [u8; 20] {
        let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
        let mut msg = data.to_vec();
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
        for chunk in msg.chunks(64) {
            let mut w = [0u32; 80];
            for (i, word) in chunk.chunks(4).enumerate() {
                w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
            }
            for i in 16..80 {
                w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
            }
            let [mut a, mut b, mut c, mut d, mut e] = h;
            for (i, wi) in w.iter().enumerate() {
                let (f, k) = match i {
                    0..=19 => ((b & c) | (!b & d), 0x5A827999),
                    20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                    40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                    _ => (b ^ c ^ d, 0xCA62C1D6),
                };
                let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*wi);
                e = d;
                d = c;
                c = b.rotate_left(30);
                b = a;
                a = t;
            }
            for (x, y) in h.iter_mut().zip([a, b, c, d, e]) {
                *x = x.wrapping_add(y);
            }
        }
        let mut out = [0u8; 20];
        for (i, x) in h.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&x.to_be_bytes());
        }
        out
    }

    pub fn base64(data: &[u8]) -> String {
        const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
        for chunk in data.chunks(3) {
            let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
            for i in 0..4 {
                out.push(if i <= chunk.len() { ABC[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
            }
        }
        out
    }
}

const HOP_BY_HOP: [&str; 7] = ["connection", "keep-alive", "proxy-connection", "transfer-encoding", "te", "trailer", "upgrade"];

async fn proxy(site: &Site, port: u16, req: Request<Incoming>) -> Response<Body> {
    let (mut parts, body) = req.into_parts();
    let target = parts.uri.path_and_query().map_or("/", |p| p.as_str());
    parts.uri = match format!("http://127.0.0.1:{port}{target}").parse() {
        Ok(uri) => uri,
        Err(_) => return bad_gateway("invalid URL"),
    };
    for name in HOP_BY_HOP {
        parts.headers.remove(name);
    }
    if site.mode == Mode::Dev {
        // HTML comes back uncompressed so the dev script can be injected
        parts.headers.remove(header::ACCEPT_ENCODING);
    }
    let req = Request::from_parts(parts, body.map_err(std::io::Error::other).boxed());
    let res = match site.client.request(req).await {
        Ok(res) => res,
        Err(e) => return bad_gateway(&e.to_string()),
    };
    let (mut parts, body) = res.into_parts();
    for name in HOP_BY_HOP {
        parts.headers.remove(name);
    }
    let html = parts.headers.get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).is_some_and(|v| v.starts_with("text/html"));
    if site.mode == Mode::Dev {
        parts.headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        if html && !parts.headers.contains_key(header::CONTENT_ENCODING) {
            let bytes = match body.collect().await {
                Ok(b) => b.to_bytes(),
                Err(e) => return bad_gateway(&e.to_string()),
            };
            let bytes = inject_dev(&bytes);
            parts.headers.insert(header::CONTENT_LENGTH, HeaderValue::from(bytes.len()));
            return Response::from_parts(parts, Full::new(bytes).map_err(|e| match e {}).boxed());
        }
    }
    Response::from_parts(parts, body.map_err(std::io::Error::other).boxed())
}

fn bad_gateway(detail: &str) -> Response<Body> {
    let text = format!("node server did not respond ({detail}); see the terminal");
    response(StatusCode::BAD_GATEWAY, "text/plain; charset=utf-8", "no-store", Bytes::from(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_and_rejects_traversal() {
        assert_eq!(decode_path("/shop/caf%C3%A9").as_deref(), Some("/shop/café"));
        assert_eq!(decode_path("/a/../b"), None);
        assert_eq!(decode_path("/a/%2e%2e/b"), None);
        assert_eq!(decode_path("/a%zz"), None);
    }

    #[test]
    fn parses_accept_encoding() {
        assert!(accepts("gzip, deflate, br", "br"));
        assert!(!accepts("gzip, br;q=0", "br"));
        assert!(accepts("gzip;q=0.5", "gzip"));
    }

    #[test]
    fn websocket_handshake_and_frames() {
        // RFC 6455 §1.3 example
        assert_eq!(ws::accept("dGhlIHNhbXBsZSBub25jZQ=="), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
        assert_eq!(ws::base64(&ws::sha1(b"abc")), "qZk+NkcGgWq6PiVxeFDCbJzQ2J0=");
        assert_eq!(ws::base64(b"ab"), "YWI=");
        assert_eq!(ws::frame("hi"), [0x81, 2, b'h', b'i']);
        let long = "x".repeat(300);
        assert_eq!(&ws::frame(&long)[..4], &[0x81, 126, 1, 44]);
        assert_eq!(ws::frame(&"x".repeat(70_000))[1], 127);
    }

    #[test]
    fn injects_dev_script_once() {
        let out = inject_dev(b"<html><head></head></html>");
        assert_eq!(&out[..], format!("<html><head>{DEV_SCRIPT}</head></html>").as_bytes());
        assert_eq!(inject_dev(&out), out);
    }
}
