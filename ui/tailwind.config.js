/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        sentinel: {
          bg: "#0f1419",
          panel: "#1a2332",
          accent: "#3b82f6",
          danger: "#ef4444",
          success: "#22c55e",
          muted: "#94a3b8",
        },
      },
    },
  },
  plugins: [],
};
