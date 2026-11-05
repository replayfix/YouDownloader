<script module lang="ts">
  type DeezerTrack = { id: string; title: string; artist: string }
  let input = $state("")
  let playlistId = $state<string | null>(null)
  let message = $state("")
  let title = $state("")
  let tracks = $state<DeezerTrack[]>([])
  let tracksMessage = $state("")
  let selectedIds = $state<string[]>([])
  let selectWholePlaylist = $state(false)
  let loadGeneration = 0
</script>

<script lang="ts">
  import { readText } from "@tauri-apps/plugin-clipboard-manager"
  import TrackToggle from "$lib/components/TrackToggle.svelte"
  import { Channel, invoke } from "@tauri-apps/api/core"
  import { deezerProgress } from "$lib/spotify-progress"


  let downloading = $derived($deezerProgress !== null)

  async function loadPlaylist() {
    const generation = ++loadGeneration
    const link = input.trim()
    playlistId = null
    title = ""
    tracks = []
    selectedIds = []
    selectWholePlaylist = false
    tracksMessage = ""
    message = link ? "" : "Pega el enlace de una playlist pública de Deezer."
    if (!link) return

    tracksMessage = "Cargando canciones…"
    try {
      const result = await invoke<{ id: string; title: string; tracks: DeezerTrack[] }>("load_deezer_playlist", { playlistId: link })
      if (generation !== loadGeneration) return
      title = result.title
      playlistId = result.id
      tracks = result.tracks
      tracksMessage = tracks.length ? "" : "Deezer no mostró canciones disponibles."
    } catch (error) {
      if (generation === loadGeneration) {
        tracksMessage = typeof error === "string" ? error : "No se pudo cargar la lista de Deezer."
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
    deezerProgress.set(message)
    const progress = new Channel<string>()
    progress.onmessage = (status) => deezerProgress.set(status)
    try {
      const folder = await invoke<string>("download_deezer_playlist", {
        playlistId,
        trackIds: selectWholePlaylist ? null : [...selectedIds],
        progress,
      })
      message = `Proceso finalizado. Carpeta: ${folder}`
    } catch (error) {
      message = String(error)
    } finally {
      deezerProgress.set(null)
    }
  }

</script>

<section class="mx-auto h-full w-full max-w-4xl space-y-4 overflow-y-auto p-6">
  <h2 class="text-xl font-semibold text-yt-text">Deezer</h2>
  <form class="space-y-2" onsubmit={(event) => { event.preventDefault(); loadPlaylist() }}>
    <fieldset disabled={downloading} class="space-y-2">
    <label class="block text-sm text-yt-text" for="deezer-playlist">Pega la URL de la playlist</label>
    <div class="flex flex-wrap gap-2">
      <input
        id="deezer-playlist"
        type="text"
        inputmode="url"
        spellcheck={false}
        bind:value={input}
        placeholder="https://www.deezer.com/playlist/…"
        class="min-w-52 flex-1 rounded-lg border border-yt-border bg-yt-surface px-3 py-2 text-yt-text"
      />
      <button type="button" onclick={pastePlaylist} class="rounded-lg border border-yt-border px-3 py-2">Pegar</button>
      <button type="submit" class="rounded-lg bg-yt-primary px-4 py-2 font-medium text-white">Cargar playlist</button>
    </div>
    </fieldset>
  </form>

  <p role="status" class="break-words text-sm text-yt-text-secondary">{$deezerProgress ?? message}</p>
  {#if tracksMessage}
    <p role="status" class="text-sm text-yt-text-secondary">{tracksMessage}</p>
  {/if}

  {#if playlistId}
    {#if title}<h3 class="font-semibold text-yt-text">{title}</h3>{/if}
    <div class="flex flex-wrap gap-2">
      <label class="flex items-center gap-2 text-sm text-yt-text">
        <TrackToggle checked={selectWholePlaylist} label="Seleccionar toda la playlist" disabled={downloading} onchange={toggleAll} />
        Seleccionar todo
      </label>
      <button type="button" disabled={downloading || (!selectWholePlaylist && !selectedIds.length)} onclick={downloadPlaylist} class="rounded-lg bg-yt-primary px-4 py-2 font-medium text-white disabled:opacity-50">
        Descargar en MP3
      </button>
    </div>
    {#if tracks.length}
      <h3 class="text-sm font-semibold text-yt-text">Canciones disponibles para descarga individual</h3>
      <ol class="divide-y divide-yt-border rounded-lg border border-yt-border">
        {#each tracks as track}
          <li class="flex items-center justify-between gap-3 px-3 py-2">
            <div class="min-w-0 break-words">
              <p class="font-medium text-yt-text">{track.title}</p>
              <p class="text-sm text-yt-text-secondary">{track.artist}</p>
            </div>
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
    {/if}
    <p class="text-sm text-yt-text-secondary">El audio se busca en YouTube Music. Algunas coincidencias pueden corresponder a otra versión de la canción.</p>
  {/if}
</section>
