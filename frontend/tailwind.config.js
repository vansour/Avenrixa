/** @type {import('tailwindcss').Config} */
import defaultTheme from 'tailwindcss/preset-default'
import colors from 'tailwindcss/preset-default/colors'
import { default as flattenColor } from 'tailwindcss/preset-default/colors'

const { slate } = colors
const backgroundColors = {
  ...flattenColor(slate),
}

const button = {
  ...flattenColor(slate),
  base: 'h-9.5 sm:h-10',
  lg: 'h-11 sm:h-12',
  icon: 'h-10 w-10',
}

const input = {
  ...flattenColor(slate),
  l: 'h-10 w-full text-sm rounded-md border border-input bg-background px-3 py-2 text-sm file:mr-4 file:py-2 file:rounded-md file:border-0 file:bg-background file:text-sm file:font-medium',
  m: 'h-10 px-3 py-2',
}

const card = {
  ...flattenColor(slate),
  DEFAULT: 'rounded-xl border border bg-card text-card-foreground shadow',
}

const radius = {
  lg: '0.5rem',
  md: 'calc(0.5rem - 2px)',
  sm: 'calc(0.5rem - 4px)',
}

module.exports = {
  darkMode: 'class',
  content: [
    './index.html',
    './src/**/*.{vue,js,ts,jsx,tsx}',
  ],
  safelist: {
    pattern: /border-/i,
  },
  theme: {
    extend: {
      colors: {
        border: 'hsl(var(--border))',
        input: 'hsl(var(--input))',
        ring: 'hsl(var(--ring))',
        background: 'hsl(var(--background))',
        foreground: 'hsl(var(--foreground))',
        primary: {
          DEFAULT: 'hsl(var(--primary))',
          foreground: 'hsl(var(--primary-foreground))',
        },
        secondary: {
          DEFAULT: 'hsl(var(--secondary))',
          foreground: 'hsl(var(--secondary-foreground))',
        },
        destructive: {
          DEFAULT: 'hsl(var(--destructive))',
          foreground: 'hsl(var(--destructive-foreground))',
        },
        muted: {
          DEFAULT: 'hsl(var(--muted))',
          foreground: 'hsl(var(--muted-foreground))',
        },
        accent: {
          DEFAULT: 'hsl(var(--accent))',
          foreground: 'hsl(var(--accent-foreground))',
        },
        popover: {
          DEFAULT: 'hsl(var(--popover))',
          foreground: 'hsl(var(--popover-foreground))',
        },
        card: {
          DEFAULT: 'hsl(var(--card))',
          foreground: 'hsl(var(--card-foreground))',
        },
      },
      borderRadius: {
        lg: 'var(--radius)',
        md: 'calc(var(--radius) - 2px)',
        sm: 'calc(var(--radius) - 4px)',
      },
      keyframes: {
        'accordion-down': {
          from: { height: '0' },
          to: { height: 'var(--radix-accordion-content-height)' },
        },
        'accordion-up': {
          from: { height: 'var(--radix-accordion-content-height)' },
          to: { height: '0' },
        },
        'in': {
          from: { scale: 0.98, opacity: 0 },
          to: { scale: 1, opacity: 1 },
        },
        'out': {
          from: { scale: 1, opacity: 1 },
          to: { scale: 0.98, opacity: 0 },
        },
      },
      animation: {
        'accordion-down': 'accordion-down 0.2s ease-out',
        'accordion-up': 'accordion-up 0.2s ease-out',
        'in': 'in 0.2s ease-out',
        'out': 'out 0.2s ease-out',
      },
    },
  },
}
