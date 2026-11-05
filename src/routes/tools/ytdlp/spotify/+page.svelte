<script lang="ts">
  import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager"
  import { openUrl } from "@tauri-apps/plugin-opener"
  import { invoke } from "@tauri-apps/api/core"
  import { spotifyPlaylistId } from "$lib/utils/spotify-playlist.js"

  let input = $state("")
  let playlistId = $state<string | null>(null)
  let message = $state("")
  let downloading = $state(false)
  let playlistUrl = $derived(playlistId ? `https://open.spotify.com/playlist/${playlistId}` : "")

  function loadPlaylist() {
    playlistId = spotifyPlaylistId(input)
    message = playlistId ? "" : "Pega un enlace de playlist de open.spotify.com."
  }

  async function pastePlaylist() {
    try {
      input = (await readText()).trim()
      loadPlaylist()
    } catch {
      message = "No se pudo leer el portapapeles. Pega la URL en el campo."
    }
  }

  async function downloadPlaylist() {
    if (!playlistId || downloading) return
    downloading = true
    message = "Buscando las pistas en YouTube Music y descargando en MP3…"
    try {
      await invoke("download_spotify_playlist", { playlistId })
      message = "Proceso finalizado. Revisa tu carpeta de descargas."
    } catch (error) {
      message = String(error)
    } finally {
      downloading = false
    }
  }

  async function openDownloader() {
    try {
      await writeText(playlistUrl)
      message = "URL copiada. Pégala en SpotSaver para continuar."
    } catch {
      message = "Copia manualmente el enlace de la playlist para pegarlo en SpotSaver."
    }

    try {
      await openUrl("https://spotsaver.net/es/")
    } catch {
      message = "No se pudo abrir SpotSaver en el navegador."
    }
  }
</script>

<section class="mx-auto w-full max-w-4xl space-y-4 p-6">
  <h2 class="text-xl font-semibold text-yt-text">Spotify</h2>
  <form class="space-y-2" onsubmit={(event) => { event.preventDefault(); loadPlaylist() }}>
    <fieldset disabled={downloading} class="space-y-2">
    <label class="block text-sm text-yt-text" for="spotify-playlist">Pega la URL de la playlist</label>
    <div class="flex flex-wrap gap-2">
      <input
        id="spotify-playlist"
        type="text"
        inputmode="url"
        spellcheck={false}
        bind:value={input}
        placeholder="https://open.spotify.com/playlist/…"
        class="min-w-52 flex-1 rounded-lg border border-yt-border bg-yt-surface px-3 py-2 text-yt-text"
      />
      <button type="button" onclick={pastePlaylist} class="rounded-lg border border-yt-border px-3 py-2">Pegar</button>
      <button type="submit" class="rounded-lg bg-yt-primary px-4 py-2 font-medium text-white">Cargar playlist</button>
    </div>
    </fieldset>
  </form>

  <p role="status" class="text-sm text-yt-text-secondary">{message}</p>

  {#if playlistId}
    <iframe
      title="Playlist de Spotify"
      src={`https://open.spotify.com/embed/playlist/${playlistId}`}
      height="480"
      class="w-full rounded-xl border-0"
      allow="autoplay; encrypted-media; fullscreen; picture-in-picture"
    ></iframe>
    <button type="button" disabled={downloading} onclick={downloadPlaylist} class="rounded-lg bg-yt-primary px-4 py-2 text-white disabled:opacity-50">
      {downloading ? "Descargando…" : "Descargar playlist en MP3"}
    </button>
    <button type="button" disabled={downloading} onclick={openDownloader} class="rounded-lg border border-yt-border px-4 py-2">
      Copiar URL y abrir SpotSaver
    </button>
    <p class="text-sm text-yt-text-secondary">El audio se busca en YouTube Music. Algunas coincidencias pueden corresponder a otra versión de la canción.</p>
  {/if}
</section>
