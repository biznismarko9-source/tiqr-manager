/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        // 2.0.56: marko's own "Brown & beige" pick, replacing the generic
        // blue-accent/white-light/navy-dark look every dashboard starts
        // with. Both ramps below are full 11-stop Tailwind-shaped scales
        // (not just the 2 exact colors from the preview document) so every
        // existing bg-/text-/border-/ring-/divide-/placeholder:-slate-N and
        // -brand-N class across the whole app picks up the new colors
        // automatically - nothing else needed to change to re-theme every
        // page at once.
        //
        // brand: was a blue accent (50 #eef4ff ... 950 #181c4d) - now a
        // rich brown. 600 (~#7d5726) is the "confirmed" accent from the
        // preview document's light mode (was closer to #6b4a30 there - a
        // full graduated ramp needs a slightly different exact value at
        // each stop than one hand-picked swatch did, same reasoning as the
        // slate ramp below). 400 is the paler stop dark-mode text/links
        // already lean on for contrast against a dark page - same role
        // brand-400 always had, just brown instead of blue now.
        brand: {
          50: "#f9f4ec",
          100: "#f0e4d0",
          200: "#e2cba3",
          300: "#cea86d",
          400: "#b8863f",
          500: "#9c6d2e",
          600: "#7d5726",
          700: "#644422",
          800: "#4f371f",
          900: "#3f2c1c",
          950: "#251a10",
        },
        // slate: overrides Tailwind's own built-in blue-gray slate (50
        // #f8fafc ... 950 #020617) with a warm beige-to-brown neutral ramp
        // instead - this is what actually fixes "light mode white, dark
        // mode navy", since body/page backgrounds, borders, and most text
        // throughout the app are all slate-N, not brand-N. 50 (#f2e9d8) and
        // 950 (#241a10) are the exact page-background colors from the
        // confirmed preview; 900 (#2f2317) is deliberately LIGHTER than 950
        // so a dark-mode card (bg-slate-900) still reads as "raised" above
        // the dark-mode page (bg-slate-950) behind it, same relationship
        // light mode already has between white cards and the slate-50 page.
        slate: {
          50: "#f2e9d8",
          100: "#e9dcc2",
          200: "#dbc7a0",
          300: "#c4a877",
          400: "#a3835a",
          500: "#836548",
          600: "#684f39",
          700: "#503d2d",
          800: "#3d2f1e",
          900: "#2f2317",
          950: "#241a10",
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
