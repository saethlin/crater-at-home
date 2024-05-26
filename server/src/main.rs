use ansi_to_html::Renderer;
use anyhow::Result;
use bytes::Bytes;
use hyper::body::Frame;
use hyper::body::Incoming;
use hyper::body::SizeHint;
use hyper::header::HeaderValue;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use semver::Version;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use tokio::net::TcpListener;
use xz2::bufread::XzDecoder;

mod tokiort;
use tokiort::TokioIo;

const NOT_FOUND: &str = include_str!("../404.html");
const LANDING_PAGE: &str = include_str!("../index.html");

async fn handle(req: Request<Incoming>) -> Result<Response<Body>> {
    let mut response = inner(req).await;
    response.headers_mut().insert(
        "Content-Type",
        HeaderValue::from_static("text/html;charset=utf-8"),
    );
    Ok(response)
}

async fn inner(req: Request<Incoming>) -> Response<Body> {
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

    let from_file = |path: &str, title: String| {
        if let Ok(file) = File::open(path) {
            log::info!("Rendering from file {}", path);
            let reader = BufReader::new(file);
            let reader = XzDecoder::new(reader);
            let body = Body {
                is_eof: false,
                kind: BodyKind::Rendered(Renderer::new(reader, title)),
            };
            Response::new(body)
        } else {
            log::info!("Unable to open file {}", path);
            error_response()
        }
    };

    match components[..] {
        ["logs", krate, version] => {
            let path = format!("{root}/{tool}/raw/{krate}/{version}");
            from_file(&path, format!("{} {}", krate, version))
        }
        ["ub"] => {
            if let Ok(file) = File::open(format!("{root}/{tool}/ub")) {
                let mut reader = BufReader::new(file);
                reader.fill_buf().unwrap();
                Response::new(Body {
                    is_eof: false,
                    kind: BodyKind::Streamed(reader),
                })
            } else {
                error_response()
            }
        }
        [] => {
            if let Some(krate) = req.uri().query() {
                let path = format!("{root}/{tool}/raw/{krate}");
                log::info!("{path}");

                let mut max_version = None;
                let Ok(iter) = std::fs::read_dir(&path) else {
                    return error_response();
                };
                for entry in iter {
                    let entry = entry.unwrap();
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
                    from_file(&path, format!("{} {}", path, version))
                } else {
                    error_response()
                }
            } else {
                Response::new(Body {
                    is_eof: false,
                    kind: BodyKind::Static(LANDING_PAGE),
                })
            }
        }
        _ => error_response(),
    }
}

fn error_response() -> Response<Body> {
    let mut response = Response::new(Body {
        is_eof: false,
        kind: BodyKind::Static(NOT_FOUND),
    });
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
                .serve_connection(io, service_fn(handle))
                .await
            {
                log::error!("Error serving connection: {:?}", err);
            }
        });
    }
}

struct Body {
    is_eof: bool,
    kind: BodyKind,
}

enum BodyKind {
    Static(&'static str),
    Streamed(BufReader<File>),
    Rendered(Renderer<XzDecoder<BufReader<File>>>),
}

impl hyper::body::Body for Body {
    type Data = Bytes;
    type Error = anyhow::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match &mut self.kind {
            BodyKind::Static(s) => {
                if s.is_empty() {
                    Poll::Ready(None)
                } else {
                    let chunk = Bytes::from_static(s.as_bytes());
                    *s = "";
                    Poll::Ready(Some(Ok(Frame::data(chunk))))
                }
            }
            BodyKind::Rendered(renderer) => {
                if let Ok(Some(line)) = renderer.next_line() {
                    let mut line = String::from_utf8_lossy(&line);
                    for pat in [
                        "Undefined Behavior:",
                        "ERROR: AddressSanitizer:",
                        "attempted to leave type",
                        "misaligned pointer dereference",
                    ] {
                        if line.contains(pat) {
                            let replacement = format!("<span id=\"ub\"></span>{pat}");
                            line = line.replacen(pat, &replacement, 1).into();
                            break;
                        }
                    }
                    let chunk = Bytes::copy_from_slice(line.as_bytes());
                    Poll::Ready(Some(Ok(Frame::data(chunk))))
                } else {
                    Poll::Ready(None)
                }
            }
            BodyKind::Streamed(reader) => {
                let buf = reader.fill_buf()?;
                let len = buf.len();
                let chunk = Bytes::copy_from_slice(buf);
                reader.consume(len);
                reader.fill_buf()?;
                if chunk.is_empty() {
                    Poll::Ready(None)
                } else {
                    Poll::Ready(Some(Ok(Frame::data(chunk))))
                }
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.is_eof
    }

    fn size_hint(&self) -> SizeHint {
        match &self.kind {
            BodyKind::Static(s) => SizeHint::with_exact(s.len() as u64),
            BodyKind::Streamed(_) | BodyKind::Rendered { .. } => SizeHint::default(),
        }
    }
}
