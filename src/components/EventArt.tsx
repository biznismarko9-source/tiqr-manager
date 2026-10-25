// 2.116.0 - an event's picture, the same on every screen.
//
// One component for the Events list, an event's page, Inventory, an order,
// Sales and a sale. It never looks for a picture itself - it shows what the
// shared store (lib/eventArtwork.ts) says the event shows, so every screen
// agrees. The frame has its size from the first frame on, picture or not,
// so nothing moves when a picture arrives; without one it is a quiet
// placeholder, never someone else's picture.
import { useEventArtwork, sourceName } from "../lib/eventArtwork";
import type { ArtworkImage } from "../lib/types";
import { IconImage } from "./icons";

export type ArtSize = "row" | "xs" | "sm" | "md" | "lg";

/** Square, 6-12px radius: 28 / 40 px in tables, 56 / 72 px on a record's page. */
const FRAME: Record<ArtSize, string> = {
  // A one-line table row (Inventory): 28px reaching into the cell's own
  // padding, so a dense list keeps its height.
  row: "h-7 w-7 -my-1 rounded-md",
  xs: "h-8 w-8 rounded-lg",
  sm: "h-10 w-10 rounded-lg",
  md: "h-14 w-14 rounded-xl",
  lg: "h-[72px] w-[72px] rounded-xl",
};

const GLYPH: Record<ArtSize, string> = {
  row: "h-3.5 w-3.5",
  xs: "h-3.5 w-3.5",
  sm: "h-4 w-4",
  md: "h-5 w-5",
  lg: "h-6 w-6",
};

/** A picture in its frame - or the placeholder. Also what the picker shows
 *  a candidate in, so a choice looks there exactly as it will on the list. */
export function ArtFrame({
  image,
  name,
  size = "sm",
  className = "",
}: {
  image: (Omit<Pick<ArtworkImage, "src" | "fit" | "title" | "source">, "src"> & { src: string | null }) | null | undefined;
  name: string;
  size?: ArtSize;
  className?: string;
}) {
  // `inline-block`: a span sizes only as a block, and the frame must keep its
  // size wherever it is put - in a flex row or on its own.
  const frame = `relative inline-block shrink-0 overflow-hidden align-middle ${FRAME[size]} ${className}`;
  if (!image?.src) {
    return (
      <span
        aria-hidden="true"
        className={`${frame} inline-grid place-items-center border border-slate-200 bg-slate-100 text-slate-400 dark:border-white/10 dark:bg-white/[0.04] dark:text-slate-500`}
      >
        <IconImage className={GLYPH[size]} />
      </span>
    );
  }
  const contain = image.fit === "contain";
  const tip = image.title ? `${image.title}${image.source ? ` · ${sourceName(image.source)}` : ""}` : undefined;
  return (
    <span
      className={`${frame} border border-slate-200 dark:border-white/10 ${contain ? "bg-white p-[3px]" : "bg-slate-100 dark:bg-white/[0.04]"}`}
      title={tip}
    >
      <img
        src={image.src}
        alt={`Picture for ${name}`}
        className={`h-full w-full ${contain ? "object-contain" : "object-cover"}`}
        draggable={false}
        decoding="async"
      />
    </span>
  );
}

/** The picture an event shows, by its id. */
export function EventArt({
  eventId,
  name,
  size = "sm",
  className = "",
}: {
  eventId: number | null | undefined;
  name: string;
  size?: ArtSize;
  className?: string;
}) {
  const { image } = useEventArtwork(eventId);
  return <ArtFrame image={image} name={name} size={size} className={className} />;
}
