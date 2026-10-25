// 2.116.0 - event pictures, connected to the backend and to the screens.
//
// lib/artworkStore.ts holds the one copy of what every event shows; this
// file gives it the real backend, keeps it current, and runs the quiet
// background search that finds pictures for events that have none.
import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { api, errMsg } from "./api";
import { createArtworkStore } from "./artworkStore";
import { onDataChanged } from "./dataChanged";
import { reportError } from "./diagnostics";
import type { ArtworkImage, ArtworkRef, ArtworkSuggestInput, ArtworkSuggestion } from "./types";

export const artworkStore = createArtworkStore({
  map: () => api.eventArtworkMap(),
  images: (keys) => api.eventArtworkImages(keys),
  report: (where, e) => {
    reportError(where, e, { severity: "warning" });
  },
});

/** Tables whose changes can change which picture an event shows - an event
 *  renamed or recategorised, a choice made on the other computer. */
const ARTWORK_TABLES = ["events", "event_artworks", "event_categories"];
let listening = false;

function listen() {
  if (listening) return;
  listening = true;
  onDataChanged((tables) => {
    if (tables.some((t) => ARTWORK_TABLES.includes(t))) artworkStore.invalidate();
  });
}

/** The picture an event shows on every screen. `undefined` while loading,
 *  `null` when there is none - both drawn as the same placeholder. */
export function useEventArtwork(eventId: number | null | undefined): {
  ref: ArtworkRef | null | undefined;
  image: ArtworkImage | null | undefined;
} {
  listen();
  useSyncExternalStore(artworkStore.subscribe, artworkStore.getVersion);
  if (eventId === null || eventId === undefined) return { ref: null, image: null };
  const ref = artworkStore.refFor(eventId);
  if (!ref) return { ref, image: ref === undefined ? undefined : null };
  return { ref, image: artworkStore.imageFor(ref) };
}

/** After a change made on this computer: an event saved, a picture chosen. */
export function refreshArtwork() {
  artworkStore.invalidate();
  nudgeArtworkSearch();
}

// ---------------------------------------------------------------------------
// The background search.
// ---------------------------------------------------------------------------

let started = false;
let running = false;
let again = false;
let timer: ReturnType<typeof setTimeout> | null = null;

const FIRST_MS = 8_000;
const BUSY_MS = 8_000;
const IDLE_MS = 15 * 60_000;
const PAUSED_MS = 5 * 60_000;
const FAILED_MS = 10 * 60_000;
const HIDDEN_MS = 60_000;

function schedule(ms: number) {
  if (!started) return;
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => void round(), ms);
}

async function round() {
  timer = null;
  if (!started) return;
  if (running) {
    again = true;
    return;
  }
  if (typeof document !== "undefined" && document.visibilityState !== "visible") {
    schedule(HIDDEN_MS);
    return;
  }
  running = true;
  let next = IDLE_MS;
  try {
    const r = await api.eventArtworkBackfill(3);
    if (r.found > 0) artworkStore.invalidate();
    next = r.paused ? PAUSED_MS : r.remaining > 0 ? BUSY_MS : IDLE_MS;
  } catch (e) {
    reportError("event pictures: background search", e, { severity: "warning" });
    next = FAILED_MS;
  } finally {
    running = false;
  }
  if (again) {
    again = false;
    next = Math.min(next, 1_500);
  }
  schedule(next);
}

/** Started once per signed-in session (Layout.tsx). A few searches at a
 *  time, only while the window is in front, and much less often once
 *  nothing is waiting - the backend paces each service on top of that. */
export function startArtworkSearch() {
  if (started) return;
  started = true;
  schedule(FIRST_MS);
}

export function stopArtworkSearch() {
  started = false;
  again = false;
  if (timer) clearTimeout(timer);
  timer = null;
}

/** Something new may be waiting (an event was just created): look soon. */
export function nudgeArtworkSearch() {
  if (!started) return;
  if (running) {
    again = true;
    return;
  }
  schedule(1_500);
}

// ---------------------------------------------------------------------------
// Suggestions for the picker.
// ---------------------------------------------------------------------------

/**
 * What the picker shows for an event being typed or edited: debounced, and
 * a reply to an older name is never shown over the reply to the newer one
 * (the request id), so typing "Cold" and then "Coldplay" can only ever end
 * on Coldplay.
 */
export function useArtworkSuggestion(
  input: Omit<ArtworkSuggestInput, "mode"> | null,
  firstMode: "cached" | "search",
  delayMs = 600,
) {
  const [suggestion, setSuggestion] = useState<ArtworkSuggestion | null>(null);
  // What the suggestion shown was made for: until the reply for what is
  // typed now arrives, the one on screen is about the name typed before.
  const [madeFor, setMadeFor] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestId = useRef(0);
  const name = input?.name.trim() ?? "";
  const key = input ? `${input.eventId ?? ""}|${name}|${input.artistTeam ?? ""}|${input.categoryId ?? ""}` : "";
  const keyRef = useRef(key);
  keyRef.current = key;
  const latest = useRef(input);
  latest.current = input;

  const load = (mode: ArtworkSuggestInput["mode"]) => {
    const current = latest.current;
    const askedFor = keyRef.current;
    const id = ++requestId.current;
    if (!current || !current.name.trim()) {
      setSuggestion(null);
      setLoading(false);
      setError(null);
      return Promise.resolve(null);
    }
    setLoading(true);
    setError(null);
    return api
      .eventArtworkSuggest({ ...current, mode })
      .then((s) => {
        if (id !== requestId.current) return null;
        setSuggestion(s);
        setMadeFor(askedFor);
        // A first look that found nothing known yet asks once.
        if (mode === "cached" && s.outcome === "unknown") void load("search");
        return s;
      })
      .catch((e) => {
        if (id !== requestId.current) return null;
        setError(errMsg(e));
        reportError("event pictures: suggestion", e, { severity: "warning" });
        return null;
      })
      .finally(() => {
        if (id === requestId.current) setLoading(false);
      });
  };

  useEffect(() => {
    if (!key) {
      requestId.current += 1;
      setSuggestion(null);
      setLoading(false);
      return;
    }
    const t = setTimeout(() => void load(firstMode), delayMs);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, firstMode, delayMs]);

  /** True when the suggestion is about what is typed now. */
  const fresh = !!suggestion && madeFor === key;
  return { suggestion, loading, error, load, fresh };
}

// ---------------------------------------------------------------------------
// A picture added by hand.
// ---------------------------------------------------------------------------

const UPLOAD_EDGE = 320;
const UPLOAD_MAX_BYTES = 4 * 1024 * 1024 * 10;

/**
 * Shrinks a chosen file to a thumbnail on a canvas - nothing is installed
 * for it - keeping transparency for a PNG (a logo stays a logo on the dark
 * theme) and making everything else a JPEG.
 */
export function shrinkArtwork(file: File): Promise<{ src: string; fit: "cover" | "contain" }> {
  return new Promise((resolve, reject) => {
    if (!file.type.startsWith("image/")) {
      reject(new Error("That file is not a picture."));
      return;
    }
    if (file.size > UPLOAD_MAX_BYTES) {
      reject(new Error("That picture is too large - choose one under 40 MB."));
      return;
    }
    const reader = new FileReader();
    reader.onerror = () => reject(new Error("The file could not be read."));
    reader.onload = () => {
      const img = new Image();
      img.onerror = () => reject(new Error("That picture could not be opened."));
      img.onload = () => {
        const scale = Math.min(1, UPLOAD_EDGE / Math.max(img.width, img.height));
        const w = Math.max(1, Math.round(img.width * scale));
        const h = Math.max(1, Math.round(img.height * scale));
        const canvas = document.createElement("canvas");
        canvas.width = w;
        canvas.height = h;
        const ctx = canvas.getContext("2d");
        if (!ctx) {
          reject(new Error("The picture could not be prepared."));
          return;
        }
        const transparent = file.type === "image/png" || file.type === "image/webp" || file.type === "image/gif";
        if (!transparent) {
          ctx.fillStyle = "#ffffff";
          ctx.fillRect(0, 0, w, h);
        }
        ctx.drawImage(img, 0, 0, w, h);
        const src = transparent ? canvas.toDataURL("image/png") : canvas.toDataURL("image/jpeg", 0.86);
        // A square-ish photo fills the frame; a wide or tall one, or a logo,
        // is shown whole.
        const ratio = w / h;
        resolve({ src, fit: transparent || ratio > 1.4 || ratio < 0.7 ? "contain" : "cover" });
      };
      img.src = String(reader.result);
    };
    reader.readAsDataURL(file);
  });
}

export const SOURCE_NAMES: Record<string, string> = {
  deezer: "Deezer",
  wikipedia: "Wikipedia",
  upload: "Your picture",
  earlier: "Chosen before",
};

export function sourceName(source: string): string {
  return SOURCE_NAMES[source] ?? source;
}
