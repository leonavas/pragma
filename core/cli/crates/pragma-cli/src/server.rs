use bytes::Bytes;
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::body::{Frame, Incoming};
use hyper::header::{self, HeaderMap, HeaderValue};
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::client::legacy::{Client, connect::HttpConnector};
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::collections::HashMap;
use std::convert::Infallible;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc};

pub type Body = BoxBody<Bytes, std::io::Error>;

pub const DEV_SCRIPT: &str = "<script type=\"module\" src=\"/_pragma/dev.js\"></script>";
const DEV_JS: &str = include_str!("dev.js");
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Dev,
    Serve,
}

#[derive(Clone, Debug)]
pub enum Event {
    Css(String),
    Reload,
    Error(String),
}

#[derive(Clone, Debug)]
struct Route {
    segments: Vec<Option<String>>,
    html: String,
    ssr: bool,
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
    routes: RwLock<Arc<Vec<Route>>>,
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
            routes: RwLock::new(Arc::new(load_manifest(&root))),
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
        *self.routes.write().unwrap() = Arc::new(load_manifest(&self.root));
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

    fn match_route(&self, path: &str) -> Option<Route> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let routes = self.routes.read().unwrap().clone();
        routes
            .iter()
            .filter(|r| {
                r.segments.len() == parts.len()
                    && r.segments.iter().zip(&parts).all(|(s, p)| s.as_deref().is_none_or(|s| s == *p))
            })
            // static beats param, segment by segment
            .min_by_key(|r| r.segments.iter().map(Option::is_none).collect::<Vec<_>>())
            .cloned()
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
                .await;
        });
    }
}

async fn handle(site: Arc<Site>, req: Request<Incoming>) -> Result<Response<Body>, Infallible> {
    let raw = req.uri().path().to_owned();
    if site.mode == Mode::Dev {
        match raw.as_str() {
            "/_pragma/reload" => return Ok(sse(&site)),
            "/_pragma/dev.js" => {
                return Ok(response(StatusCode::OK, "text/javascript; charset=utf-8", "no-store", Bytes::from_static(DEV_JS.as_bytes())));
            }
            _ => {}
        }
    }
    let Some(path) = decode_path(&raw) else {
        return Ok(response(StatusCode::BAD_REQUEST, "text/plain; charset=utf-8", "no-store", Bytes::from_static("invalid path".as_bytes())));
    };
    let route = site.match_route(&path);
    if let Some(port) = site.backend()
        && (is_server_path(&path) || route.as_ref().is_some_and(|r| r.ssr) && !path.ends_with(".html"))
    {
        return Ok(proxy(&site, port, req).await);
    }
    if site.mode == Mode::Dev && path == "/_otel" && req.method() == Method::POST {
        if let Ok(body) = req.into_body().collect().await {
            crate::node::print_client_logs(&body.to_bytes());
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
    Ok(res)
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

pub fn inject_dev(html: &[u8]) -> Bytes {
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
    let path = String::from_utf8(out).ok()?;
    let bad = path.contains('\0') || path.contains('\\') || path.split('/').any(|s| s == ".." || s == ".");
    (!bad).then_some(path)
}

fn load_manifest(root: &Path) -> Vec<Route> {
    let Ok(text) = std::fs::read_to_string(root.join("_pragma/manifest.json")) else { return Vec::new() };
    let Ok(serde_json::Value::Array(items)) = serde_json::from_str(&text) else { return Vec::new() };
    items
        .iter()
        .filter_map(|item| {
            let pattern = item.get("pattern")?.as_str()?;
            Some(Route {
                segments: pattern
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .map(|s| if s.starts_with(':') { None } else { Some(s.to_owned()) })
                    .collect(),
                html: item.get("html")?.as_str()?.to_owned(),
                ssr: item.get("ssr").and_then(|v| v.as_bool()).unwrap_or(false),
            })
        })
        .collect()
}

pub fn mime(path: &str) -> &'static str {
    let ext = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "map" => "application/json",
        "webmanifest" => "application/manifest+json",
        "txt" => "text/plain; charset=utf-8",
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

// `serve` runs on an immutable dist: compressing once at startup, at max quality and in parallel,
// keeps CPU off the request path and avoids cache/locks; larger files are served from disk.
const MAX_CACHED: u64 = 8 * 1024 * 1024;

fn precompress(root: &Path) -> HashMap<String, Cached> {
    let mut files = Vec::new();
    walk(root, root, &mut files);
    files.retain(|(rel, meta)| compressible(rel) && meta.len() <= MAX_CACHED);
    let queue = Mutex::new(files);
    let out = Mutex::new(HashMap::new());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let Some((rel, meta)) = queue.lock().unwrap().pop() else { return };
                    let Ok(body) = std::fs::read(root.join(&rel)) else { continue };
                    let (gzip, brotli) = if body.len() >= 256 { (gzip(&body), brotli(&body)) } else { (None, None) };
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

struct ChannelBody(mpsc::Receiver<Bytes>);

impl hyper::body::Body for ChannelBody {
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Bytes>, std::io::Error>>> {
        self.0.poll_recv(cx).map(|chunk| chunk.map(|b| Ok(Frame::data(b))))
    }
}

fn sse_frame(event: &Event) -> Bytes {
    // the browser does not fire an SSE event without `data`
    let (name, data) = match event {
        Event::Css(href) => ("css", href.as_str()),
        Event::Reload => ("reload", "1"),
        Event::Error(json) => ("error", json.as_str()),
    };
    Bytes::from(format!("event: {name}\ndata: {data}\n\n"))
}

fn sse(site: &Site) -> Response<Body> {
    let (tx, rx) = mpsc::channel(16);
    let mut events = site.events.subscribe();
    let mut first = format!("retry: 1000\nevent: hello\ndata: {}\n\n", site.boot_id);
    if let Some(json) = site.last_error.lock().unwrap().clone() {
        first.push_str(&String::from_utf8_lossy(&sse_frame(&Event::Error(json))));
    }
    tokio::spawn(async move {
        if tx.send(Bytes::from(first)).await.is_err() {
            return;
        }
        let mut ping = tokio::time::interval(Duration::from_secs(15));
        ping.tick().await;
        loop {
            let chunk = tokio::select! {
                event = events.recv() => match event {
                    Ok(event) => sse_frame(&event),
                    Err(broadcast::error::RecvError::Lagged(_)) => sse_frame(&Event::Reload),
                    Err(broadcast::error::RecvError::Closed) => return,
                },
                _ = ping.tick() => Bytes::from_static(b": ping\n\n"),
            };
            if tx.send(chunk).await.is_err() {
                return;
            }
        }
    });
    let mut res = Response::new(ChannelBody(rx).boxed());
    let h = res.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    h.insert("x-accel-buffering", HeaderValue::from_static("no"));
    res
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
    fn injects_dev_script_once() {
        let out = inject_dev(b"<html><head></head></html>");
        assert_eq!(&out[..], format!("<html><head>{DEV_SCRIPT}</head></html>").as_bytes());
        assert_eq!(inject_dev(&out), out);
    }
}
