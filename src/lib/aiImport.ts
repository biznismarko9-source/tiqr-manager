// TIQR Manager - AI Import Assistant, frontend half (2.7.0)
//
// Everything here is deliberately UI-free so it can be reasoned about (and,
// where it matters, tested) on its own: getting an image out of a paste /
// drop / file pick, making it safe and cheap to send, fingerprinting it so
// the same picture is never analyzed twice, and describing how each
// extracted field maps into an existing form input.
//
// The React half is `components/AiImportPanel.tsx`; the Rust half is
// `src-tauri/src/commands/ai_import.rs`, whose module doc comment is the
// place to start.
//
// The rule that shapes this whole file: **nothing here ever writes to the
// database, and nothing here calls the API on its own.** `AiImportSession`
// is the only thing that can reach `api.analyzeImportImage`, and it only
// does so from a method a user action calls directly.

import { api } from "./api";
import type { AiImportKind, AiImportResult, AiImportTicketGroup } from "./types";

// ---------------------------------------------------------------------------
// Image handling (marko's section 11)
// ---------------------------------------------------------------------------

/** Mirrors `ALLOWED_MEDIA_TYPES` in ai_import.rs. Both sides check; this one
 * is for a good error message, that one is the actual gate. */
export const AI_IMPORT_MEDIA_TYPES = ["image/png", "image/jpeg", "image/webp"] as const;

/** Mirrors `MAX_IMAGE_BYTES` in ai_import.rs (Anthropic's own per-image
 * ceiling). Anything bigger is downscaled below rather than rejected - a
 * 4000px-wide retina screenshot is completely normal and shouldn't be a
 * dead end. */
export const AI_IMPORT_MAX_BYTES = 5 * 1024 * 1024;

/** Longest edge we ever send. Anthropic downsamples anything larger than
 * this server-side anyway, so sending more costs tokens without adding
 * legibility - and doing the resize ourselves means we control the
 * quality setting, which is what keeps small ticket text readable. */
export const AI_IMPORT_MAX_EDGE = 1568;

export function isSupportedImageType(type: string): boolean {
  return (AI_IMPORT_MEDIA_TYPES as readonly string[]).includes(type);
}

/** One image, ready to send. `previewUrl` is a data URL for the panel's own
 * thumbnail, so the picture stays on screen through an analysis, an error
 * and a retry (marko's section 13: "image zostáva dostupný"). */
export interface AiImportImage {
  mediaType: string;
  base64: string;
  previewUrl: string;
  fingerprint: string;
}

/** FNV-1a over the base64 payload. Not a cryptographic hash and doesn't need
 * to be - its only job is "have I already paid to analyze exactly these
 * bytes in this session," where a collision would at worst reuse a result
 * for a genuinely different image the user could re-analyze with Retry.
 * Chosen over `crypto.subtle.digest` on purpose: that's async, and its
 * availability depends on the webview treating the app's origin as secure,
 * which is one more thing that can silently differ between a dev build and
 * an installed one. */
export function fingerprintBase64(base64: string): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < base64.length; i++) {
    h ^= base64.charCodeAt(i);
    // 32-bit FNV prime multiply, written as shifts so it stays in int range.
    h = (h + (h << 1) + (h << 4) + (h << 7) + (h << 8) + (h << 24)) >>> 0;
  }
  return `${base64.length.toString(36)}-${h.toString(36)}`;
}

function readAsDataUrl(file: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.onerror = () => reject(new Error("That image could not be read."));
    reader.readAsDataURL(file);
  });
}

function loadImage(dataUrl: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("That image could not be read."));
    img.src = dataUrl;
  });
}

function splitDataUrl(dataUrl: string): { mediaType: string; base64: string } {
  const match = /^data:([^;,]+)[^,]*,(.*)$/s.exec(dataUrl);
  if (!match) throw new Error("That image could not be read.");
  return { mediaType: match[1]!, base64: match[2]! };
}

/**
 * Turns a picked/pasted/dropped file into something safe and cheap to send.
 *
 * The size rule is deliberately "leave it alone unless we have to touch it":
 * a screenshot that is already within the ceiling and within
 * `AI_IMPORT_MAX_EDGE` is sent byte-for-byte, because every re-encode costs
 * some of the small-text legibility this whole feature depends on. Only an
 * oversized image is redrawn, and then at high JPEG quality rather than a
 * default one - the goal is "fits", not "small".
 */
export async function prepareImage(file: File): Promise<AiImportImage> {
  if (!isSupportedImageType(file.type)) {
    throw new Error("Unsupported image type. Use PNG, JPG or WebP.");
  }
  const originalUrl = await readAsDataUrl(file);
  const img = await loadImage(originalUrl);
  const longestEdge = Math.max(img.naturalWidth, img.naturalHeight);

  if (file.size <= AI_IMPORT_MAX_BYTES && longestEdge <= AI_IMPORT_MAX_EDGE) {
    const { mediaType, base64 } = splitDataUrl(originalUrl);
    return { mediaType, base64, previewUrl: originalUrl, fingerprint: fingerprintBase64(base64) };
  }

  const scale = Math.min(1, AI_IMPORT_MAX_EDGE / longestEdge);
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.round(img.naturalWidth * scale));
  canvas.height = Math.max(1, Math.round(img.naturalHeight * scale));
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("That image could not be read.");
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = "high";
  ctx.drawImage(img, 0, 0, canvas.width, canvas.height);

  const resizedUrl = canvas.toDataURL("image/jpeg", 0.92);
  const { mediaType, base64 } = splitDataUrl(resizedUrl);
  // A resize that somehow still isn't enough is a real failure, not
  // something to keep halving in a loop - marko gets the short message and
  // can crop it himself.
  if (base64.length * 0.75 > AI_IMPORT_MAX_BYTES) {
    throw new Error("Image is too large.");
  }
  return { mediaType, base64, previewUrl: resizedUrl, fingerprint: fingerprintBase64(base64) };
}

// ---------------------------------------------------------------------------
// Getting an image out of a paste or a drop
// ---------------------------------------------------------------------------

/**
 * The Ctrl+V half (marko's section 2), and the reason it is a pure function:
 * the single most important behaviour here is what it does when the
 * clipboard holds NO image, which is **nothing at all**. It returns null,
 * the caller doesn't call `preventDefault()`, and pasting text into whatever
 * input has focus works exactly as it always did. A paste handler that
 * swallowed every paste would break every form in the app for the sake of
 * one panel.
 */
export function imageFromClipboard(e: ClipboardEvent): File | null {
  const data = e.clipboardData;
  if (!data) return null;
  // `files` covers a screenshot pasted from the OS clipboard; `items` covers
  // an image copied out of a browser/another app. Both exist in the Tauri
  // webview and neither is reliably a superset of the other. Indexed rather
  // than `Array.from`: `FileList`/`DataTransferItemList` are array-LIKE, not
  // reliably iterable across the DOM lib versions this project may build
  // against, and this is not the place to find that out.
  const files = data.files;
  if (files) {
    for (let i = 0; i < files.length; i++) {
      const file = files.item(i);
      if (file && file.type.startsWith("image/")) return file;
    }
  }
  const items = data.items;
  if (items) {
    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      if (item && item.kind === "file" && item.type.startsWith("image/")) {
        const file = item.getAsFile();
        if (file) return file;
      }
    }
  }
  return null;
}

/** The drag & drop half. NOTE: this only ever fires because `dragDropEnabled`
 * is false for the main window (`src-tauri/tauri.conf.json`) - with Tauri's
 * default of true, the OS-level handler swallows the drop and no HTML drop
 * event reaches the webview at all. If drag & drop ever silently stops
 * working, that flag is the first thing to check. */
export function imageFromDrop(files: FileList | null): File | null {
  if (!files) return null;
  for (let i = 0; i < files.length; i++) {
    const file = files.item(i);
    if (file && file.type.startsWith("image/")) return file;
  }
  return null;
}

// ---------------------------------------------------------------------------
// Field labels + where each one goes
// ---------------------------------------------------------------------------

/** Human labels for the review list. Keys match `field_names_for_kind` in
 * ai_import.rs; a key missing here falls back to the raw name rather than
 * hiding the field. */
export const AI_IMPORT_FIELD_LABELS: Record<string, string> = {
  // event
  name: "Event name",
  eventDate: "Event date",
  venue: "Venue",
  city: "City",
  country: "Country",
  category: "Category",
  status: "Status",
  // order
  eventName: "Event",
  orderReference: "Order reference",
  platform: "Platform",
  purchaseDate: "Purchase date",
  totalPrice: "Total price",
  currency: "Currency",
  // sale
  saleDate: "Sale date",
  quantity: "Quantity",
  salePrice: "Sale price",
  sellingFees: "Selling fees",
  marketplace: "Marketplace",
  buyerReference: "Buyer reference",
  paymentStatus: "Payment status",
  deliveryStatus: "Delivery status",
};

export function fieldLabel(field: string): string {
  return AI_IMPORT_FIELD_LABELS[field] ?? field;
}

export const TICKET_GROUP_LABELS: Record<keyof Omit<AiImportTicketGroup, "seats">, string> = {
  quantity: "Quantity",
  tier: "Tier / Level",
  section: "Section",
  row: "Row",
  ticketType: "Ticket type",
  unitPrice: "Price per ticket",
  fees: "Fees per ticket",
};

/**
 * True only for a real `YYYY-MM-DD` string, which is the only thing an
 * `<input type="date">` will actually display - hand it "14 Sep 2026" and it
 * silently shows an empty field, which looks exactly like the extraction
 * having failed.
 *
 * The prompt already asks for ISO dates (see `build_prompt` in
 * ai_import.rs), so this is the belt-and-braces half: a date that somehow
 * arrives in another format is simply not written into the form, while
 * staying visible (and editable) in the review list, so nothing is lost and
 * nothing is silently wrong. It also range-checks the parts, so
 * "2026-13-45" doesn't pass.
 */
export function isIsoDate(value: string | undefined | null): value is string {
  if (!value || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const [y, m, d] = value.split("-").map(Number);
  if (m! < 1 || m! > 12 || d! < 1 || d! > 31) return false;
  const parsed = new Date(`${value}T00:00:00Z`);
  return !Number.isNaN(parsed.getTime()) && parsed.getUTCDate() === d && parsed.getUTCMonth() + 1 === m;
}

/** Case/whitespace-insensitive lookup of an extracted name against a list the
 * app already has (event categories, platforms, ...). Returns null on no
 * match, and the caller then leaves that field alone - the AI is never
 * allowed to create a new lookup row, only to point at one that exists. */
export function matchByName<T extends { id: number; name: string }>(
  options: T[],
  raw: string | undefined,
): T | null {
  if (!raw) return null;
  const needle = raw.trim().toLowerCase();
  if (!needle) return null;
  return options.find((o) => o.name.trim().toLowerCase() === needle) ?? null;
}

/** What the panel hands back once marko presses "Fill form". Nothing has
 * been saved at this point and nothing will be until he submits the form
 * himself - this is the boundary between "AI suggested" and "the app's own
 * create flow", and it is intentionally a plain bag of strings.
 *
 * `fields` only ever contains values that are actually present: a field the
 * image didn't show is absent here, so filling can never blank something the
 * user already typed. */
export interface AiImportApplied {
  fields: Record<string, string>;
  /** The ticket group marko chose, for the "order" kind. Null everywhere
   * else, and null for an order whose screenshot showed no ticket detail. */
  group: AiImportTicketGroup | null;
}

// ---------------------------------------------------------------------------
// The one thing allowed to call the API (marko's section 14)
// ---------------------------------------------------------------------------

/**
 * One panel's worth of analysis state, holding the two guarantees marko
 * asked for that a component alone can't give:
 *
 *   1. **One image = one request.** `analyze` refuses to spend anything on a
 *      fingerprint it has already analyzed in this session and hands back
 *      the cached result instead. Re-dropping the same screenshot, closing
 *      and reopening the panel, or pressing "Fill form" and then coming
 *      back, all cost nothing.
 *   2. **No request without an explicit action.** Every call site is a
 *      click, a drop, a paste or Retry. There is no timer, no effect, no
 *      background pass, and nothing here re-runs when an extracted field is
 *      edited - editing is pure local state in the panel.
 *
 * `retry` is the one deliberate escape hatch: it bypasses the cache, because
 * "try that again" after a failure is exactly the case where re-sending the
 * same bytes is the point. That is also why it is only ever wired to a
 * button.
 */
export class AiImportSession {
  private cache = new Map<string, AiImportResult>();

  constructor(private readonly kind: AiImportKind) {}

  /** Returns a cached result when this exact image was already analyzed. */
  cached(image: AiImportImage): AiImportResult | null {
    return this.cache.get(image.fingerprint) ?? null;
  }

  async analyze(image: AiImportImage): Promise<AiImportResult> {
    const hit = this.cache.get(image.fingerprint);
    if (hit) return hit;
    return this.request(image);
  }

  /** Explicit user retry - skips the cache on purpose. */
  async retry(image: AiImportImage): Promise<AiImportResult> {
    this.cache.delete(image.fingerprint);
    return this.request(image);
  }

  private async request(image: AiImportImage): Promise<AiImportResult> {
    const result = await api.analyzeImportImage(this.kind, image.mediaType, image.base64);
    // An unreadable verdict is cached too: re-sending the identical blurry
    // screenshot would cost the same money for the same answer. Retry still
    // clears it, for the case where the failure was transient.
    this.cache.set(image.fingerprint, result);
    return result;
  }
}
