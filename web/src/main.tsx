import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'
import appIconUrl from '../../logo.png'

const faviconElement =
  document.querySelector<HTMLLinkElement>("link[rel~='icon']") ?? document.createElement('link')

faviconElement.rel = 'icon'
faviconElement.type = 'image/png'
faviconElement.href = appIconUrl

if (!faviconElement.parentElement) {
  document.head.appendChild(faviconElement)
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)
