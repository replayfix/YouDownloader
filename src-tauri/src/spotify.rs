use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

static SPOTIFY_DOWNLOAD: Mutex<()> = Mutex::const_new(());

#[tauri::command]
#[specta::specta]
pub async fn download_spotify_playlist(app: AppHandle, playlist_id: String) -> Result<(), String> {
    if playlist_id.len() != 22 || !playlist_id.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
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
    if !cfg!(target_os = "windows") || !executable.is_file() {
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
    let folder = PathBuf::from(folder);
    tokio::fs::create_dir_all(&folder)
        .await
        .map_err(|error| error.to_string())?;

    let mut command = tokio::process::Command::new(executable);
    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let output = command
        .arg(playlist_id)
        .arg("--output-dir")
        .arg(&folder)
        .args([
            "--audio-format",
            "mp3",
            "--download-archive",
            ".spotify-archive.txt",
        ])
        .env("PATH", search_path)
        .env("PYTHONIOENCODING", "utf-8")
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|error| format!("No se pudo ejecutar spotify-dl: {error}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() || stderr.lines().any(|line| line.contains("ERROR:")) {
        return Err("Spotify-dl informó errores. Revisa las pistas guardadas.".into());
    }
    Ok(())
}
