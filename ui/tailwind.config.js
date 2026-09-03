/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        surface: {
          DEFAULT: "#0B0B0B",
          raised: "#121212",
          overlay: "#1A1A1A",
          hover: "#1E1E1E",
        },
        accent: {
          DEFAULT: "#007acc",
          muted: "#1a5276",
          gold: "#F0C24B",
        },
        border: {
          DEFAULT: "#171717",
          subtle: "#232323",
          active: "#333333",
        },
        status: {
          active: "#007acc",
          idle: "#6E6E6E",
          error: "#f44747",
          success: "#4ADE80",
          warning: "#F0C24B",
        },
        text: {
          primary: "#DCDCDC",
          secondary: "#C9C9C9",
          muted: "#A3A3A3",
          dim: "#7A7A7A",
        },
      },
      fontFamily: {
        mono: ["JetBrains Mono", "Fira Code", "Menlo", "monospace"],
        sans: ["Inter", "system-ui", "-apple-system", "sans-serif"],
      },
      borderRadius: {
        sm: "6px",
        md: "8px",
        lg: "9px",
        xl: "12px",
      },
    },
  },
  plugins: [],
};