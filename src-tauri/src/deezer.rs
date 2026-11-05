use std::path::PathBuf;
use tauri::ipc::Channel;
use tauri::AppHandle;
use tokio::sync::Mutex;

static DEEZER_DOWNLOAD: Mutex<()> = Mutex::const_new(());

#[derive(serde::Serialize, specta::Type)]
pub struct DeezerPlaylist {
    title: String,
    tracks: Vec<DeezerTrack>,
}

#[derive(serde::Serialize, specta::Type)]
struct DeezerTrack {
    id: String,
    title: String,
    artist: String,
}

async fn deezer_request(path: &str) -> Result<serde_json::Value, String> {
    let error = "No se pudo consultar Deezer. Comprueba que la playlist sea pública.";
    let data: serde_json::Value = crate::ytdlp::dep_download::short_http_client()
        .get(format!("https://api.deezer.com/{path}"))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| error)?
        .json()
        .await
        .map_err(|_| error)?;
    if data.get("error").is_some() {
        return Err(error.into());
    }
    Ok(data)
}

#[tauri::command]
#[specta::specta]
pub async fn load_deezer_playlist(playlist_id: String) -> Result<DeezerPlaylist, String> {
    if playlist_id.is_empty()
        || playlist_id.len() > 20
        || !playlist_id.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err("El identificador de la playlist de Deezer no es válido.".into());
    }
    let path = format!("playlist/{playlist_id}");
    let metadata = deezer_request(&path).await?;
    let title = metadata["title"]
        .as_str()
        .ok_or("Deezer no devolvió el nombre de la playlist.")?
        .to_owned();
    let mut tracks = Vec::new();
    let mut index = 0;
    loop {
        let page = deezer_request(&format!("{path}/tracks?limit=100&index={index}")).await?;
        let items = page["data"]
            .as_array()
            .ok_or("Deezer no devolvió las canciones de la playlist.")?;
        tracks.extend(items.iter().filter_map(|track| {
            Some(DeezerTrack {
                id: track["id"].as_u64()?.to_string(),
                title: track["title"].as_str()?.into(),
                artist: track["artist"]["name"].as_str()?.into(),
            })
        }));
        if items.is_empty() || !page["next"].is_string() {
            break;
        }
        index += items.len();
    }
    Ok(DeezerPlaylist { title, tracks })
}

#[tauri::command]
#[specta::specta]
pub async fn download_deezer_playlist(
    app: AppHandle,
    playlist_id: String,
    track_id: Option<String>,
    progress: Channel<String>,
) -> Result<String, String> {
    let _download = DEEZER_DOWNLOAD
        .try_lock()
        .map_err(|_| "Ya hay una descarga de Deezer en curso.".to_string())?;
    let _ = progress.send("Consultando la playlist de Deezer…".into());
    let playlist = load_deezer_playlist(playlist_id.clone()).await?;
    let tracks: Vec<_> = playlist
        .tracks
        .into_iter()
        .filter(|track| track_id.as_ref().is_none_or(|id| id == &track.id))
        .take(if track_id.is_some() { 1 } else { usize::MAX })
        .collect();
    if tracks.is_empty() {
        return Err("No hay canciones disponibles para descargar.".into());
    }

    let ffmpeg = crate::ytdlp::binary::resolve_ffmpeg_path_with_app(&app)
        .await
        .ok_or("Instala FFmpeg desde Ajustes antes de descargar.")?;
    let ytdlp = crate::ytdlp::binary::resolve_ytdlp_path_with_app(&app)
        .await
        .map_err(|error| error.to_string())?;
    let path = std::env::var_os("PATH").unwrap_or_default();
    let portable_bin = crate::portable::root().map(|root| root.join("binaries"));
    let search_path = std::env::join_paths(
        std::iter::once(PathBuf::from(ffmpeg))
            .chain(portable_bin)
            .chain(std::env::split_paths(&path)),
    )
    .map_err(|error| error.to_string())?;
    let settings = crate::ytdlp::settings::get_settings(&app).map_err(|error| error.to_string())?;
    let root = crate::ytdlp::security::sanitize_output_path(&settings.download_path)
        .map_err(|error| error.to_string())?;
    let folder = PathBuf::from(root).join(crate::spotify::playlist_folder_name(
        Some(&playlist.title),
        &playlist_id,
    ));
    tokio::fs::create_dir_all(&folder)
        .await
        .map_err(|error| error.to_string())?;

    let total = tracks.len();
    let mut failed = 0;
    for (index, track) in tracks.into_iter().enumerate() {
        let _ = progress.send(format!(
            "{}/{} · Buscando: {} · {}",
            index + 1,
            total,
            track.title,
            track.artist
        ));
        let mut command = crate::spotify::audio_search_command(
            &ytdlp,
            &track.title,
            &track.artist,
            ".deezer-archive.txt",
        );
        if let Err(error) =
            crate::spotify::run_music_download(&mut command, &folder, &search_path, &progress).await
        {
            if total == 1 {
                return Err(error);
            }
            failed += 1;
        }
    }
    if failed > 0 {
        return Err(format!(
            "No se pudieron descargar {failed} de {total} canciones. Carpeta: {}",
            folder.display()
        ));
    }
    Ok(folder.to_string_lossy().into_owned())
}
