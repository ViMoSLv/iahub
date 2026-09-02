/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        surface: {
          DEFAULT: "#1a1a2e",
          raised: "#16213e",
          overlay: "#0f3460",
        },
        accent: {
          DEFAULT: "#e94560",
          muted: "#533483",
        },
        status: {
          active: "#00d2ff",
          idle: "#7b8794",
          error: "#e94560",
          success: "#0ead69",
        },
      },
      fontFamily: {
        mono: ["JetBrains Mono", "Fira Code", "Menlo", "monospace"],
      },
    },
  },
  plugins: [],
};