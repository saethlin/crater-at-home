use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;

use anyhow::Result;
use hyper::body::Incoming;
use hyper::header::HeaderValue;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use semver::Version;
use tokio::net::TcpListener;

mod tokiort;
use tokiort::TokioIo;

use ansi_to_html::Handle;

const NOT_FOUND: &str = include_str!("../404.html");
const LANDING_PAGE: &str = include_str!("../index.html");

async fn echo(req: Request<Incoming>) -> Result<Response<BodyKind>> {
    log::info!("{req:?}");

    let path = req.uri().path();
    let mut components = path
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    if components.get(0) == Some(&"staging") {
        components.remove(0);
    }

    let root = "/crater-at-home";
    let tool = "miri";

    let mut response = match components[..] {
        ["logs", krate, version] => {
            let path = format!("{root}/{tool}/raw/{krate}/{version}");
            log::info!("{}", path);
            if let Ok(file) = File::open(path) {
                let reader = BufReader::new(file);
                let body = BodyKind::Rendered(Handle::new(reader));
                Response::new(body)
            } else {
                error_response()
            }
        }
        ["ub"] => {
        }
        Response::new(BodyKind::Static("under construction")),
        [] => {
            if let Some(krate) = req.uri().query() {
                let path = format!("{root}/{tool}/raw/{krate}");
                log::info!("{path}");

                let mut max_version = None;
                for entry in std::fs::read_dir(&path)? {
                    let entry = entry?;
                    let path = entry.path();
                    let Some(file_name) = path.file_name() else {
                        continue;
                    };
                    let Some(file_name) = file_name.to_str() else {
                        continue;
                    };
                    let version = Version::parse(file_name).ok();
                    max_version = max_version.max(version);
                }

                if let Some(version) = max_version {
                    let path = format!("{path}/{version}");
                    log::info!("{}", path);
                    if let Ok(file) = File::open(path) {
                        let reader = BufReader::new(file);
                        let body = BodyKind::Rendered(Handle::new(reader));
                        Response::new(body)
                    } else {
                        error_response()
                    }
                } else {
                    error_response()
                }
            } else {
                Response::new(BodyKind::Static(LANDING_PAGE))
            }
        }
        _ => error_response(),
    };
    response
        .headers_mut()
        .insert("Content-Type", HeaderValue::from_static("text/html;charset=utf-8"));
    Ok(response)
}

fn error_response() -> Response<BodyKind> {
    let mut response = Response::new(BodyKind::Static(NOT_FOUND));
    *response.status_mut() = StatusCode::NOT_FOUND;
    response
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));

    let listener = TcpListener::bind(addr).await?;
    log::info!("Listening on http://{}", addr);
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(echo))
                .await
            {
                log::error!("Error serving connection: {:?}", err);
            }
        });
    }
}

enum BodyKind {
    Static(&'static str),
    Streamed(BufReader<File>),
    Rendered(Handle<BufReader<File>>),
}

use bytes::Bytes;
use hyper::body::Frame;
use hyper::body::SizeHint;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use std::io::Read;

impl hyper::body::Body for BodyKind {
    type Data = Bytes;
    type Error = anyhow::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match &mut *self {
            BodyKind::Static(s) => {
                if s.is_empty() {
                    Poll::Ready(None)
                } else {
                    let chunk = Bytes::from_static(s.as_bytes());
                    *s = "";
                    Poll::Ready(Some(Ok(Frame::data(chunk))))
                }
            }
            BodyKind::Rendered(h) => {
                let mut chunk: [u8; 4096] = [0u8; 4096];
                if let Ok(n) = h.read(&mut chunk) {
                    let chunk = Bytes::copy_from_slice(&chunk[..n]);
                    Poll::Ready(Some(Ok(Frame::data(chunk))))
                } else {
                    Poll::Ready(None)
                }
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            BodyKind::Static(s) => s.is_empty(),
            BodyKind::Rendered(h) => h.is_empty(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            BodyKind::Static(s) => SizeHint::with_exact(s.len() as u64),
            BodyKind::Streamed(_) | BodyKind::Rendered(_) => SizeHint::default(),
        }
    }
}
