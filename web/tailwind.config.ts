import type { Config } from 'tailwindcss'

export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      boxShadow: {
        panel: '0 16px 40px rgba(15, 23, 42, 0.14)'
      }
    }
  },
  plugins: []
} satisfies Config
