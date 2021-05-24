import { defineConfig } from 'vite'
import preact from '@preact/preset-vite'
import windiCSS from 'vite-plugin-windicss'


// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    preact(),
    windiCSS()
  ]
})
