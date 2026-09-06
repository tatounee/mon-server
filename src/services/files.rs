use std::{
    env,
    path::{Path, PathBuf},
    str::FromStr,
    task::Poll,
};

use bytes::{Bytes, BytesMut};
use color_eyre::eyre::{Context, Report};
use http::{Method, Request, Response, StatusCode, Uri};
use tokio::{fs::File, io::AsyncReadExt};
use tower::Service;

/// How much room is reserved before each read when the file size is unknown
/// (or when the file grew past the size reported by its metadata).
const READ_CHUNK_SIZE: usize = 8 * 1024;

#[derive(Clone)]
pub struct StaticFile {
    root: PathBuf,
}

impl StaticFile {
    pub fn new<P: Into<PathBuf>>(static_dir: P) -> Result<Self, Report> {
        let mut root = env::current_dir().wrap_err("not found current dir")?;
        root.push(static_dir.into());
        let root = root
            .canonicalize()
            .wrap_err(format!("failed to canonicalize root path {root:?}"))?;

        Ok(Self { root: root })
    }

    fn uri_path(&self, uri: &Uri) -> Result<PathBuf, Report> {
        let mut path = self.root.clone();
        let mut uri = PathBuf::from_str(uri.path().trim_prefix("/"))
            .wrap_err("failed to convert uri to path")?;

        path.push(uri);
        let path = path
            .normalize_lexically()
            .wrap_err(format!("failed to get absolute requested path {path:?}"))?;

        Ok(path)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl<B> Service<Request<B>> for StaticFile {
    type Response = Response<Bytes>;

    type Error = Report;

    type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let root = self.root.clone();
        let path = self.uri_path(&req.uri());

        async move {
            if req.method() != Method::GET {
                return Ok(Response::builder()
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .body(Bytes::new())
                    .unwrap());
            }

            let mut path = path?;

            if !path.starts_with(root) {
                return Ok(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Bytes::new())
                    .unwrap());
            }

            if path.is_dir() {
                path.push("index.html")
            }

            let Ok(mut file) = File::open(&path).await else {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Bytes::new())
                    .unwrap());
            };

            // Reserve the whole file up front when the size is known, plus one
            // extra byte so the final (empty) read doesn't trigger a realloc.
            let mut buf = match file.metadata().await {
                Ok(metadata) => BytesMut::with_capacity(metadata.len() as usize + 1),
                Err(_) => BytesMut::new(),
            };

            loop {
                if buf.len() == buf.capacity() {
                    buf.reserve(READ_CHUNK_SIZE);
                }

                let read = file
                    .read_buf(&mut buf)
                    .await
                    .wrap_err(format!("failed to read file {path:?})"))?;

                if read == 0 {
                    break;
                }
            }

            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(buf.freeze())
                .unwrap())
        }
    }
}
