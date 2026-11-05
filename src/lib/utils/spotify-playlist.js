/** @param {string} input */
export function spotifyPlaylistId(input) {
  try {
    const url = new URL(input.trim())
    if (url.origin !== "https://open.spotify.com" || url.username || url.password) return null
    return /^\/(?:intl-[a-z]{2}(?:-[a-z]{2})?\/)?playlist\/([a-zA-Z0-9]{22})\/?$/i.exec(url.pathname)?.[1] ?? null
  } catch {
    return null
  }
}
