/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        // 2.0.58: reverted 2.0.56's "Brown & beige" back to the original
        // blue-accent/white-light/navy-dark look - marko tried the brown in
        // production and didn't like it. Both ramps below are full 11-stop
        // Tailwind-shaped scales (matching the same shape 2.0.56 used) so
        // every existing bg-/text-/border-/ring-/divide-/placeholder:-slate-N
        // and -brand-N class across the whole app picks up the reverted
        // colors automatically - same one-file mechanism that made 2.0.56
        // apply everywhere at once now makes reverting it a one-file change
        // too.
        //
        // brand: back to blue. This app's own history only ever recorded 3
        // exact stops of the original custom scale (50 #eef4ff, 600
        // #4a68f7 - the "confirmed" accent quoted in REDESIGN-2.0.56 - and
        // 950 #181c4d), not the other 8 - a full ramp was never dumped
        // anywhere before 2.0.56 replaced it. The 8 missing stops here are
        // reconstructed (HSL-interpolated between those 3 known points, the
        // same hue/lightness-curve approach 2.0.56 itself used to go from
        // one hand-picked swatch to a full ramp) rather than restored
        // byte-for-byte - so 50/600/950 are exactly what this app had
        // before, and 100-500/700-900 are a close, smooth match rather than
        // a guaranteed pixel-identical one. Flag it if any shade looks off
        // and I'll tune that one stop directly.
        brand: {
          50: "#eef4ff",
          100: "#d2e0ff",
          200: "#b6cbfe",
          300: "#9ab4fd",
          400: "#7f9cfb",
          500: "#6483f9",
          600: "#4a68f7",
          700: "#213fe9",
          800: "#1e30b5",
          900: "#1d277f",
          950: "#181c4d",
        },
        // slate: back to Tailwind's own built-in blue-gray slate scale -
        // this app's history confirms both endpoints (50 #f8fafc, 950
        // #020617) matched Tailwind's stock slate exactly, i.e. 2.0.56's
        // beige/brown values were the only override that ever existed here.
        // Restored byte-for-byte from Tailwind's published default palette,
        // not reconstructed - unlike brand above, there is no uncertainty
        // in this half of the revert.
        slate: {
          50: "#f8fafc",
          100: "#f1f5f9",
          200: "#e2e8f0",
          300: "#cbd5e1",
          400: "#94a3b8",
          500: "#64748b",
          600: "#475569",
          700: "#334155",
          800: "#1e293b",
          900: "#0f172a",
          950: "#020617",
        },
      },
      fontFamily: {
        sans: [
          "Inter",
          "ui-sans-serif",
          "system-ui",
          "-apple-system",
          "Segoe UI",
          "Roboto",
          "sans-serif",
        ],
      },
    },
  },
  plugins: [],
};
