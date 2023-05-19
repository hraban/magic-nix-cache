//! Binary Cache API.

use std::io;

use axum::{
    extract::{BodyStream, Extension, Path},
    response::Redirect,
    routing::{get, put},
    Router,
};
use tokio_stream::StreamExt;
use tokio_util::io::StreamReader;

use super::State;
use crate::error::{Error, Result};

pub fn get_router() -> Router {
    Router::new()
        .route("/nix-cache-info", get(get_nix_cache_info))
        // .narinfo
        .route("/:path", get(get_narinfo))
        .route("/:path", put(put_narinfo))
        // .nar
        .route("/nar/:path", get(get_nar))
        .route("/nar/:path", put(put_nar))
}

async fn get_nix_cache_info() -> &'static str {
    // TODO: Make StoreDir configurable
    r#"WantMassQuery: 1
StoreDir: /nix/store
Priority: 41
"#
}

async fn get_narinfo(
    Extension(state): Extension<State>,
    Path(path): Path<String>,
) -> Result<Redirect> {
    let components: Vec<&str> = path.splitn(2, '.').collect();

    if components.len() != 2 {
        return Err(Error::NotFound);
    }

    if components[1] != "narinfo" {
        return Err(Error::NotFound);
    }

    let store_path_hash = components[0].to_string();
    let key = format!("{}.narinfo", store_path_hash);

    if let Some(url) = state.api.get_file_url(&[&key]).await? {
        return Ok(Redirect::temporary(&url));
    }

    if let Some(upstream) = &state.upstream {
        Ok(Redirect::temporary(&format!("{}/{}", upstream, path)))
    } else {
        Err(Error::NotFound)
    }
}
async fn put_narinfo(
    Extension(state): Extension<State>,
    Path(path): Path<String>,
    body: BodyStream,
) -> Result<()> {
    let components: Vec<&str> = path.splitn(2, '.').collect();

    if components.len() != 2 {
        return Err(Error::BadRequest);
    }

    if components[1] != "narinfo" {
        return Err(Error::BadRequest);
    }

    let store_path_hash = components[0].to_string();
    let key = format!("{}.narinfo", store_path_hash);
    let allocation = state.api.allocate_file_with_random_suffix(&key).await?;
    let stream = StreamReader::new(
        body.map(|r| r.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))),
    );
    state.api.upload_file(allocation, stream).await?;

    Ok(())
}

async fn get_nar(Extension(state): Extension<State>, Path(path): Path<String>) -> Result<Redirect> {
    if let Some(url) = state.api.get_file_url(&[&path]).await? {
        return Ok(Redirect::temporary(&url));
    }

    if let Some(upstream) = &state.upstream {
        Ok(Redirect::temporary(&format!("{}/nar/{}", upstream, path)))
    } else {
        Err(Error::NotFound)
    }
}
async fn put_nar(
    Extension(state): Extension<State>,
    Path(path): Path<String>,
    body: BodyStream,
) -> Result<()> {
    let allocation = state.api.allocate_file_with_random_suffix(&path).await?;
    let stream = StreamReader::new(
        body.map(|r| r.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))),
    );
    state.api.upload_file(allocation, stream).await?;

    Ok(())
}