/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./src/**/*.{js,ts,jsx,tsx,mdx}'],
  theme: {
    extend: {
      colors: {
        hydra: {
          idle: '#6366f1',
          listening: '#8b5cf6',
          working: '#3b82f6',
          attention: '#f59e0b',
          approval: '#f97316',
          success: '#22c55e',
          error: '#ef4444',
          offline: '#6b7280',
        },
        risk: {
          none: '#22c55e',
          low: '#3b82f6',
          medium: '#f59e0b',
          high: '#ef4444',
          critical: '#dc2626',
        },
      },
      animation: {
        'breathe': 'breathe 3s ease-in-out infinite',
        'pulse-slow': 'pulse 2s ease-in-out infinite',
        'spin-slow': 'spin 3s linear infinite',
        'bounce-gentle': 'bounce-gentle 1.5s ease-in-out infinite',
      },
      keyframes: {
        breathe: {
          '0%, 100%': { opacity: '0.7', transform: 'scale(1)' },
          '50%': { opacity: '1', transform: 'scale(1.05)' },
        },
        'bounce-gentle': {
          '0%, 100%': { transform: 'translateY(0)' },
          '50%': { transform: 'translateY(-4px)' },
        },
      },
    },
  },
  plugins: [],
};
