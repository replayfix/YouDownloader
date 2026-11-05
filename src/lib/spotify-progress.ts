import { writable } from "svelte/store"

export const spotifyProgress = writable<string | null>(null)
export const deezerProgress = writable<string | null>(null)
