// Small dependency-free inline icon set (stroke-based, 24x24 viewbox) so we
// don't need an extra icon library for a handful of glyphs.
import type { SVGProps } from "react";

function Svg(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      {...props}
    />
  );
}

export const IconPlus = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M12 5v14M5 12h14" />
  </Svg>
);
export const IconSearch = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <circle cx="11" cy="11" r="7" />
    <path d="m21 21-4.3-4.3" />
  </Svg>
);
export const IconX = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M18 6 6 18M6 6l12 12" />
  </Svg>
);
export const IconChevronDown = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="m6 9 6 6 6-6" />
  </Svg>
);
export const IconChevronUp = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="m18 15-6-6-6 6" />
  </Svg>
);
export const IconTrash = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2m3 0-1 14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2L4 6h16Z" />
  </Svg>
);
export const IconPencil = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M12 20h9" />
    <path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z" />
  </Svg>
);
export const IconDownload = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M12 3v12m0 0-4-4m4 4 4-4" />
    <path d="M4 19h16" />
  </Svg>
);
export const IconUpload = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M12 21V9m0 0-4 4m4-4 4 4" />
    <path d="M4 3h16" />
  </Svg>
);
export const IconArrowLeft = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M19 12H5m0 0 6 6m-6-6 6-6" />
  </Svg>
);
export const IconTicket = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M3 8a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v2a2 2 0 0 0 0 4v2a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-2a2 2 0 0 0 0-4Z" />
    <path d="M13 5v2M13 17v2M13 11v2" />
  </Svg>
);
export const IconCalendarDays = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <rect x="3" y="4" width="18" height="18" rx="2" />
    <path d="M16 2v4M8 2v4M3 10h18" />
    <path d="M8 14h.01M12 14h.01M16 14h.01M8 18h.01M12 18h.01M16 18h.01" />
  </Svg>
);
export const IconPackage = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="m21 8-9-5-9 5 9 5 9-5Z" />
    <path d="M3 8v8l9 5 9-5V8M12 13v8" />
  </Svg>
);
export const IconWallet = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M21 12V7a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-5Zm0 0h-5a2 2 0 0 0 0 4h5" />
  </Svg>
);
export const IconGauge = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M12 14 15 9M4.6 15a9 9 0 1 1 14.8 0" />
  </Svg>
);
export const IconSettings = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1-1.5 1.7 1.7 0 0 0-1.9.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.6-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1 1.7 1.7 0 0 0-.3-1.9l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.9.3H9a1.7 1.7 0 0 0 1-1.6V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.9-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.9V9a1.7 1.7 0 0 0 1.6 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1Z" />
  </Svg>
);
export const IconReceipt = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M4 3h16v18l-3-2-2 2-2-2-2 2-2-2-2 2-3-2Z" />
    <path d="M8 8h8M8 12h8M8 16h5" />
  </Svg>
);
export const IconBoxes = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M2.5 9.5 12 4l9.5 5.5L12 15z" />
    <path d="M2.5 9.5v6L12 21l9.5-5.5v-6M12 15v6" />
  </Svg>
);
export const IconAlertTriangle = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" />
    <path d="M12 9v4M12 17h.01" />
  </Svg>
);
export const IconCheck = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M20 6 9 17l-5-5" />
  </Svg>
);
export const IconDatabase = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <ellipse cx="12" cy="5" rx="9" ry="3" />
    <path d="M3 5v14a9 3 0 0 0 18 0V5" />
    <path d="M3 12a9 3 0 0 0 18 0" />
  </Svg>
);
// 1.8.2: Settings Home category icons.
export const IconTag = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <path d="M20.59 13.41 11 3.83A2 2 0 0 0 9.59 3.24L3 3v6.59a2 2 0 0 0 .59 1.41l9.59 9.59a2 2 0 0 0 2.82 0l4.59-4.59a2 2 0 0 0 0-2.82Z" />
    <path d="M7 7h.01" />
  </Svg>
);
export const IconSun = (p: SVGProps<SVGSVGElement>) => (
  <Svg {...p}>
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" />
  </Svg>
);
