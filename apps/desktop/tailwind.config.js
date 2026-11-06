/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        bhoot: {
          dark: '#0a0d14',
          card: '#111726',
          border: '#1e293b',
          accent: '#10b981', // emerald green for offline finance
          gold: '#f59e0b',
        }
      }
    },
  },
  plugins: [],
}
