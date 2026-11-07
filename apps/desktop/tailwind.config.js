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
          bg: '#050505',
          sidebar: '#000000',
          card: '#111111',
          cardHover: '#161616',
          border: '#222222',
          borderSubtle: '#1a1a1a',
          text: '#ffffff',
          textMuted: '#888888',
          textDim: '#555555',
          accent: '#ffffff',
          // Semantic statuses
          danger: '#ef4444',
          warning: '#f59e0b',
          success: '#10b981',
          info: '#3b82f6',
        }
      }
    },
  },
  plugins: [],
}
