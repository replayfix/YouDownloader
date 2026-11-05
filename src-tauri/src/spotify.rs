use std::path::PathBuf;
use std::process::Stdio;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::sync::Mutex;

static SPOTIFY_DOWNLOAD: Mutex<()> = Mutex::const_new(());

#[tauri::command]
#[specta::specta]
pub async fn download_spotify_playlist(
    app: AppHandle,
    playlist_id: String,
    track_id: Option<String>,
    progress: Channel<String>,
) -> Result<String, String> {
    if !valid_spotify_id(&playlist_id) {
        return Err("El identificador de la playlist no es válido.".into());
    }
    let _download = SPOTIFY_DOWNLOAD
        .try_lock()
        .map_err(|_| "Ya hay una descarga de Spotify en curso.".to_string())?;
    let root = crate::portable::root()
        .or_else(|| app.path().resource_dir().ok())
        .ok_or("No se encontró la carpeta de recursos.")?;
    let executable = root
        .join("binaries")
        .join("spotify-dl")
        .join("spotify-dl.exe");
    if track_id.is_none() && (!cfg!(target_os = "windows") || !executable.is_file()) {
        return Err(
            "Esta edición no incluye spotify-dl. Usa la edición portable de Windows.".into(),
        );
    }
    let ffmpeg = crate::ytdlp::binary::resolve_ffmpeg_path_with_app(&app)
        .await
        .ok_or("Instala FFmpeg desde Ajustes antes de descargar.")?;
    let path = std::env::var_os("PATH").unwrap_or_default();
    let search_path = std::env::join_paths(
        [PathBuf::from(ffmpeg), root.join("binaries")]
            .into_iter()
            .chain(std::env::split_paths(&path)),
    )
    .map_err(|error| error.to_string())?;
    let settings = crate::ytdlp::settings::get_settings(&app).map_err(|error| error.to_string())?;
    let folder = crate::ytdlp::security::sanitize_output_path(&settings.download_path)
        .map_err(|error| error.to_string())?;
    let _ = progress.send("Consultando el nombre de la playlist…".into());
    let playlist_url = format!("https://open.spotify.com/playlist/{playlist_id}");
    let title = match crate::ytdlp::dep_download::short_http_client()
        .get("https://open.spotify.com/oembed")
        .query(&[("url", playlist_url)])
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
    {
        Ok(response) => response
            .json::<SpotifyEmbed>()
            .await
            .ok()
            .map(|embed| embed.title),
        Err(_) => None,
    };
    let folder = PathBuf::from(folder).join(playlist_folder_name(title.as_deref(), &playlist_id));
    tokio::fs::create_dir_all(&folder)
        .await
        .map_err(|error| error.to_string())?;

    let mut command = if let Some(track_id) = track_id {
        let track = load_spotify_tracks(playlist_id)
            .await?
            .into_iter()
            .find(|track| track.id == track_id)
            .ok_or("La pista ya no aparece en la lista de Spotify.")?;
        let _ = progress.send(format!("Buscando: {} · {}", track.title, track.artist));
        let ytdlp = crate::ytdlp::binary::resolve_ytdlp_path_with_app(&app)
            .await
            .map_err(|error| error.to_string())?;
        audio_search_command(&ytdlp, &track.title, &track.artist, ".spotify-archive.txt")
    } else {
        let mut command = tokio::process::Command::new(executable);
        command.arg(playlist_id).args([
            "--output-dir",
            ".",
            "--audio-format",
            "mp3",
            "--download-archive",
            ".spotify-archive.txt",
        ]);
        command
    };
    run_music_download(&mut command, &folder, &search_path, &progress).await?;
    Ok(folder.to_string_lossy().into_owned())
}

pub(crate) fn audio_search_command(
    ytdlp: &str,
    title: &str,
    artist: &str,
    archive: &str,
) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(ytdlp);
    command.args([
        "--ignore-config",
        "--no-playlist",
        "--newline",
        "--extract-audio",
        "--audio-format",
        "mp3",
        "--download-archive",
        archive,
        "--output",
        "%(creator)s - %(title)s.%(ext)s",
        "--",
    ]);
    command.arg(format!("ytsearch1:{title} {artist} official audio"));
    command
}

pub(crate) async fn run_music_download(
    command: &mut tokio::process::Command,
    folder: &std::path::Path,
    search_path: &std::ffi::OsStr,
    progress: &Channel<String>,
) -> Result<(), String> {
    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let mut child = command
        .current_dir(folder)
        .env("PATH", search_path)
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUNBUFFERED", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("No se pudo ejecutar el descargador: {error}"))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or("Falta la salida de spotify-dl.")?;
    let stderr = child.stderr.take().ok_or("Falta la salida de errores.")?;
    let mut stderr = BufReader::new(stderr).lines();
    let read_progress = async {
        let mut chunk = [0; 4096];
        let mut line = Vec::new();
        let mut previous = String::new();
        loop {
            let read = stdout.read(&mut chunk).await?;
            if read == 0 {
                break;
            }
            for &byte in &chunk[..read] {
                if matches!(byte, b'\r' | b'\n') {
                    if let Some(status) = progress_message(&String::from_utf8_lossy(&line)) {
                        if status != previous {
                            let _ = progress.send(status.clone());
                            previous = status;
                        }
                    }
                    line.clear();
                } else {
                    line.push(byte);
                }
            }
        }
        Ok::<_, std::io::Error>(())
    };
    let read_errors = async {
        let mut failed = false;
        while let Some(line) = stderr.next_line().await? {
            failed |= line.contains("ERROR:");
        }
        Ok::<_, std::io::Error>(failed)
    };
    let (_, failed, status) = tokio::try_join!(read_progress, read_errors, child.wait())
        .map_err(|error| format!("No se pudo leer el progreso de la descarga: {error}"))?;
    if !status.success() || failed {
        return Err("El descargador informó errores. Revisa las pistas guardadas.".into());
    }
    Ok(())
}

fn valid_spotify_id(id: &str) -> bool {
    id.len() == 22 && id.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

#[derive(serde::Serialize, specta::Type)]
pub struct SpotifyTrack {
    id: String,
    title: String,
    artist: String,
}

#[tauri::command]
#[specta::specta]
pub async fn load_spotify_tracks(playlist_id: String) -> Result<Vec<SpotifyTrack>, String> {
    if !valid_spotify_id(&playlist_id) {
        return Err("El identificador de la playlist no es válido.".into());
    }
    let error = "No se pudo leer la lista de canciones de Spotify.";
    let html = crate::ytdlp::dep_download::short_http_client()
        .get(format!(
            "https://open.spotify.com/embed/playlist/{playlist_id}"
        ))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| error)?
        .text()
        .await
        .map_err(|_| error)?;
    let (_, payload) = html
        .split_once(r#"<script id="__NEXT_DATA__" type="application/json">"#)
        .ok_or(error)?;
    let (payload, _) = payload.split_once("</script>").ok_or(error)?;
    let data: serde_json::Value = serde_json::from_str(payload).map_err(|_| error)?;
    let tracks = data
        .pointer("/props/pageProps/state/data/entity/trackList")
        .and_then(serde_json::Value::as_array)
        .ok_or(error)?;
    Ok(tracks
        .iter()
        .filter_map(|track| {
            let id = track["uri"].as_str()?.strip_prefix("spotify:track:")?;
            if !valid_spotify_id(id) {
                return None;
            }
            Some(SpotifyTrack {
                id: id.into(),
                title: track["title"].as_str()?.into(),
                artist: track["subtitle"].as_str().unwrap_or_default().into(),
            })
        })
        .collect())
}

#[derive(serde::Deserialize)]
struct SpotifyEmbed {
    title: String,
}

pub(crate) fn playlist_folder_name(title: Option<&str>, playlist_id: &str) -> String {
    let safe_title: String = title
        .unwrap_or_default()
        .trim()
        .chars()
        .take(80)
        .map(|ch| {
            if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let safe_title = safe_title.trim_matches([' ', '.']);
    let name = if safe_title.is_empty() {
        "Playlist de Spotify"
    } else {
        safe_title
    };
    let folder = format!("{name} [{playlist_id}]");
    if crate::ytdlp::security::is_windows_reserved_name(&folder) {
        format!("_{folder}")
    } else {
        folder
    }
}

fn progress_message(line: &str) -> Option<String> {
    let line = line.trim();
    if let Some(title) = line.strip_prefix("[MATCHING] ") {
        return Some(format!("Buscando: {title}"));
    }
    if line.starts_with("[ExtractAudio]") {
        return Some("Convirtiendo pista a MP3…".into());
    }
    let download = line.strip_prefix("[download] ")?.trim_start();
    let token = download.split_whitespace().next()?;
    if token.ends_with('%') {
        Some(format!("Descargando pista · {token}"))
    } else if download.starts_with("Destination:") {
        Some("Descargando pista…".into())
    } else {
        None
    }
}
