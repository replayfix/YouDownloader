<script module lang="ts">
  type SpotifyTrack = { id: string; title: string; artist: string; duration_seconds?: number | null }
  let input = $state("")
  let playlistId = $state<string | null>(null)
  let message = $state("")
  let tracks = $state<SpotifyTrack[]>([])
  let tracksMessage = $state("")
  let selectedIds = $state<string[]>([])
  let selectWholePlaylist = $state(false)
  let loadGeneration = 0
</script>

<script lang="ts">
  import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager"
  import { openUrl } from "@tauri-apps/plugin-opener"
  import TrackToggle from "$lib/components/TrackToggle.svelte"
  import { Channel, invoke } from "@tauri-apps/api/core"
  import { spotifyProgress } from "$lib/spotify-progress"
  import { spotifyPlaylistId } from "$lib/utils/spotify-playlist.js"


  function formatDuration(seconds?: number | null) {
    if (seconds == null) return "—"
    return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`
  }

  let downloading = $derived($spotifyProgress !== null)
  let playlistUrl = $derived(playlistId ? `https://open.spotify.com/playlist/${playlistId}` : "")

  async function loadPlaylist() {
    const generation = ++loadGeneration
    const id = spotifyPlaylistId(input)
    playlistId = id
    tracks = []
    selectedIds = []
    selectWholePlaylist = false
    tracksMessage = ""
    message = id ? "" : "Pega un enlace de playlist de open.spotify.com."
    if (!id) return

    tracksMessage = "Cargando canciones…"
    try {
      const result = await invoke<SpotifyTrack[]>("load_spotify_tracks", { playlistId: id })
      if (generation !== loadGeneration) return
      tracks = result
      tracksMessage = tracks.length ? "" : "Spotify no mostró canciones disponibles."
    } catch {
      if (generation === loadGeneration) {
        tracksMessage = "No se pudo cargar la lista de canciones. Puedes usar Descargar todo."
      }
    }
  }

  async function pastePlaylist() {
    try {
      input = (await readText()).trim()
      loadPlaylist()
    } catch {
      message = "No se pudo leer el portapapeles. Pega la URL en el campo."
    }
  }

  function toggleAll() {
    selectWholePlaylist = !selectWholePlaylist
    selectedIds = selectWholePlaylist ? [...new Set(tracks.map((track) => track.id))] : []
  }

  function toggleTrack(id: string) {
    selectWholePlaylist = false
    selectedIds = selectedIds.includes(id)
      ? selectedIds.filter((selected) => selected !== id)
      : [...selectedIds, id]
  }

  async function downloadPlaylist() {
    if (!playlistId || downloading || (!selectWholePlaylist && !selectedIds.length)) return
    message = "Buscando las pistas en YouTube Music y descargando en MP3…"
    spotifyProgress.set(message)
    const progress = new Channel<string>()
    progress.onmessage = (status) => spotifyProgress.set(status)
    try {
      const folder = await invoke<string>("download_spotify_playlist", {
        playlistId,
        trackIds: selectWholePlaylist ? null : [...selectedIds],
        progress,
      })
      message = `Proceso finalizado. Carpeta: ${folder}`
    } catch (error) {
      message = String(error)
    } finally {
      spotifyProgress.set(null)
    }
  }

  async function openDownloader() {
    try {
      await writeText(playlistUrl)
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

<section class="mx-auto h-full w-full max-w-4xl space-y-4 overflow-y-auto p-6">
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

  <p role="status" class="break-words text-sm text-yt-text-secondary">{$spotifyProgress ?? message}</p>

  {#if playlistId}
    <div class="flex flex-wrap gap-2">
      <label class="flex items-center gap-2 text-sm text-yt-text">
        <TrackToggle checked={selectWholePlaylist} label="Seleccionar toda la playlist" disabled={downloading} onchange={toggleAll} />
        Seleccionar todo
      </label>
      <button type="button" disabled={downloading || (!selectWholePlaylist && !selectedIds.length)} onclick={downloadPlaylist} class="rounded-lg bg-yt-primary px-4 py-2 font-medium text-white disabled:opacity-50">
        Descargar en MP3
      </button>
      <button type="button" disabled={downloading} onclick={openDownloader} class="rounded-lg border border-yt-border px-4 py-2">
        Copiar URL y abrir SpotSaver
      </button>
    </div>
    <p class="text-sm text-yt-text-secondary" role="status">{tracksMessage}</p>
    {#if tracks.length}
      <h3 class="text-sm font-semibold text-yt-text">Canciones disponibles · {tracks.length}</h3>
      <div class="overflow-hidden rounded-lg border border-yt-border">
        <div aria-hidden="true" class="grid grid-cols-[1.5rem_minmax(0,1fr)_3rem_6.5rem] gap-2 border-b border-yt-border bg-yt-surface px-3 py-2 text-xs font-medium text-yt-text-secondary">
          <span class="text-center">#</span>
          <span>Canción</span>
          <span class="text-right">Tiempo</span>
          <span class="text-center">Elegir</span>
        </div>
      <ol class="divide-y divide-yt-border">
        {#each tracks as track, index}
          <li class="grid grid-cols-[1.5rem_minmax(0,1fr)_3rem_6.5rem] items-center gap-2 px-3 py-2 hover:bg-yt-highlight">
            <span aria-hidden="true" class="text-center text-xs tabular-nums text-yt-text-muted">{index + 1}</span>
            <div class="min-w-0 break-words">
              <p class="font-medium text-yt-text">{track.title}</p>
              <p class="text-sm text-yt-text-secondary">{track.artist}</p>
            </div>
            <span class="text-right text-xs tabular-nums text-yt-text-secondary">{formatDuration(track.duration_seconds)}</span>
            <div class="flex justify-center">
              <TrackToggle
                checked={selectedIds.includes(track.id)}
                label={`Seleccionar ${track.title}`}
                disabled={downloading}
                onchange={() => toggleTrack(track.id)}
              />
            </div>
          </li>
        {/each}
      </ol>
      </div>
    {/if}
    <p class="text-sm text-yt-text-secondary">El audio se busca en YouTube Music. Algunas coincidencias pueden corresponder a otra versión de la canción.</p>
  {/if}
</section>
