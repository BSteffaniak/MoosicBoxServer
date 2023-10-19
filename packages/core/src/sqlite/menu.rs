use crate::{
    app::AppState,
    cache::{get_or_set_to_cache, CacheItemType, CacheRequest},
};
use serde::{Deserialize, Serialize};
use std::{sync::PoisonError, time::Duration};
use thiserror::Error;

use super::{
    db::{self, DbError},
    models::{Album, AlbumSource, Artist, Track},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FullAlbum {
    pub id: i32,
    pub title: String,
    pub artist: String,
    pub year: Option<i32>,
    pub icon: Option<String>,
    pub source: AlbumSource,
}

impl<T> From<PoisonError<T>> for GetAlbumError {
    fn from(_err: PoisonError<T>) -> Self {
        Self::PoisonError
    }
}

#[derive(Debug, Error)]
pub enum GetArtistError {
    #[error("Album not found with ID {artist_id:?}")]
    ArtistNotFound { artist_id: i32 },
    #[error("Unknown source: {artist_source:?}")]
    UnknownSource { artist_source: String },
    #[error("Poison error")]
    PoisonError,
    #[error(transparent)]
    SqliteError(#[from] sqlite::Error),
    #[error(transparent)]
    DbError(#[from] db::DbError),
}

pub async fn get_artist(artist_id: i32, data: &AppState) -> Result<Artist, GetArtistError> {
    let request = CacheRequest {
        key: format!("artist|{artist_id}"),
        expiration: Duration::from_secs(5 * 60),
    };

    Ok(get_or_set_to_cache(request, || async {
        let library = data.db.as_ref().unwrap().library.lock().unwrap();
        match db::get_artist(&library, artist_id) {
            Ok(artist) => {
                if artist.is_none() {
                    return Err(GetArtistError::ArtistNotFound { artist_id });
                }

                let artist = artist.unwrap();

                Ok(CacheItemType::Artist(artist))
            }
            Err(err) => Err(GetArtistError::DbError(err)),
        }
    })
    .await?
    .into_artist()
    .unwrap())
}

#[derive(Debug, Error)]
pub enum GetAlbumError {
    #[error("Album not found with ID {album_id:?}")]
    AlbumNotFound { album_id: i32 },
    #[error("Too many albums found with ID {album_id:?}")]
    TooManyAlbumsFound { album_id: i32 },
    #[error("Unknown source: {album_source:?}")]
    UnknownSource { album_source: String },
    #[error("Poison error")]
    PoisonError,
    #[error(transparent)]
    SqliteError(#[from] sqlite::Error),
    #[error(transparent)]
    DbError(#[from] db::DbError),
}

pub async fn get_album(album_id: i32, data: &AppState) -> Result<Album, GetAlbumError> {
    let request = CacheRequest {
        key: format!("album|{album_id}"),
        expiration: Duration::from_secs(5 * 60),
    };

    Ok(get_or_set_to_cache(request, || async {
        let library = data.db.as_ref().unwrap().library.lock().unwrap();
        match db::get_album(&library, album_id) {
            Ok(album) => {
                if album.is_none() {
                    return Err(GetAlbumError::AlbumNotFound { album_id });
                }

                let album = album.unwrap();

                Ok(CacheItemType::Album(album))
            }
            Err(err) => Err(GetAlbumError::DbError(err)),
        }
    })
    .await?
    .into_album()
    .unwrap())
}

impl<T> From<PoisonError<T>> for GetAlbumsError {
    fn from(_err: PoisonError<T>) -> Self {
        Self::PoisonError
    }
}

#[derive(Debug, Error)]
pub enum GetAlbumsError {
    #[error("Poison error")]
    PoisonError,
    #[error(transparent)]
    JsonError(#[from] awc::error::JsonPayloadError),
    #[error(transparent)]
    SqliteError(#[from] sqlite::Error),
}

pub async fn get_albums(data: &AppState) -> Result<Vec<Album>, GetAlbumsError> {
    let request = CacheRequest {
        key: "sqlite|local_albums".to_string(),
        expiration: Duration::from_secs(5 * 60),
    };

    Ok(get_or_set_to_cache(request, || async {
        Ok::<CacheItemType, GetAlbumsError>(CacheItemType::Albums(
            data.db
                .as_ref()
                .unwrap()
                .library
                .lock()?
                .prepare("SELECT * from albums")?
                .into_iter()
                .filter_map(|row| row.ok())
                .map(|row| {
                    let id = row.read::<i64, _>("id") as i32;
                    let artist_id = row.read::<i64, _>("artist") as i32;
                    let title = String::from(row.read::<&str, _>("title"));
                    let date_released = row
                        .read::<Option<&str>, _>("date_released")
                        .map(|date| date.to_string());
                    let artwork = row
                        .read::<Option<&str>, _>("artwork")
                        .map(|_a| format!("/albums/{id}/300x300"));
                    let directory = row
                        .read::<Option<&str>, _>("directory")
                        .map(|dir| dir.to_string());
                    Album {
                        id,
                        title,
                        artist_id,
                        date_released,
                        artwork,
                        directory,
                        ..Default::default()
                    }
                })
                .collect(),
        ))
    })
    .await?
    .into_albums()
    .unwrap())
}

impl<T> From<PoisonError<T>> for GetAlbumTracksError {
    fn from(_err: PoisonError<T>) -> Self {
        Self::Poison
    }
}

#[derive(Debug, Error)]
pub enum GetAlbumTracksError {
    #[error("Poison error")]
    Poison,
    #[error(transparent)]
    Json(#[from] awc::error::JsonPayloadError),
    #[error(transparent)]
    Sqlite(#[from] sqlite::Error),
    #[error(transparent)]
    Db(#[from] DbError),
}

pub async fn get_album_tracks(
    album_id: i32,
    data: &AppState,
) -> Result<Vec<Track>, GetAlbumTracksError> {
    let request = CacheRequest {
        key: format!("sqlite|local_album_tracks|{album_id}"),
        expiration: Duration::from_secs(5 * 60),
    };

    Ok(get_or_set_to_cache(request, || async {
        let library = data.db.as_ref().unwrap().library.lock().unwrap();
        Ok::<CacheItemType, GetAlbumTracksError>(CacheItemType::AlbumTracks(db::get_album_tracks(
            &library, album_id,
        )?))
    })
    .await?
    .into_album_tracks()
    .unwrap())
}

#[derive(Debug, Error)]
pub enum GetArtistAlbumsError {
    #[error("Poison error")]
    Poison,
    #[error(transparent)]
    Json(#[from] awc::error::JsonPayloadError),
    #[error(transparent)]
    Sqlite(#[from] sqlite::Error),
    #[error(transparent)]
    Db(#[from] DbError),
}

pub async fn get_artist_albums(
    artist_id: i32,
    data: &AppState,
) -> Result<Vec<Album>, GetArtistAlbumsError> {
    let request = CacheRequest {
        key: format!("sqlite|local_artist_albums|{artist_id}"),
        expiration: Duration::from_secs(5 * 60),
    };

    Ok(get_or_set_to_cache(request, || async {
        let library = data.db.as_ref().unwrap().library.lock().unwrap();
        Ok::<CacheItemType, GetArtistAlbumsError>(CacheItemType::ArtistAlbums(
            db::get_artist_albums(&library, artist_id)?,
        ))
    })
    .await?
    .into_artist_albums()
    .unwrap())
}
