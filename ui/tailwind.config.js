/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        surface: {
          DEFAULT: "#0d0d0d",
          raised: "#1a1a0e",
          overlay: "#2a2a12",
        },
        accent: {
          DEFAULT: "#c8b400",
          muted: "#6b6200",
        },
        lemon: {
          50: "#fefce8",
          100: "#fef9c3",
          200: "#fef08a",
          300: "#fde047",
          400: "#d4c800",
          500: "#c8b400",
          600: "#a39200",
          700: "#6b6200",
          800: "#3d3800",
          900: "#1a1a0e",
        },
        status: {
          active: "#d4c800",
          idle: "#6b6b5a",
          error: "#c83232",
          success: "#7ab800",
        },
      },
      fontFamily: {
        mono: ["JetBrains Mono", "Fira Code", "Menlo", "monospace"],
      },
    },
  },
  plugins: [],
};