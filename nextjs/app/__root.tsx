import { createRootRoute, Outlet } from '@tanstack/react-router'
import { Providers } from '@/components/providers'
import { ShellLayout } from '@/components/shell-layout'
import { BreadcrumbProvider } from '@/contexts/breadcrumb-context'
import '@/app/globals.css'

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: 'utf-8' },
      { name: 'viewport', content: 'width=device-width, initial-scale=1' },
      { name: 'theme-color', content: '#0a0a0a' },
      { name: 'mobile-web-app-capable', content: 'yes' },
      { name: 'apple-mobile-web-app-capable', content: 'yes' },
      { name: 'apple-mobile-web-app-status-bar-style', content: 'black-translucent' },
      { name: 'apple-mobile-web-app-title', content: '模拟仲裁庭' },
    ],
    links: [
      { rel: 'icon', href: '/icon.svg' },
      { rel: 'apple-touch-icon', href: '/apple-icon.svg' },
      { rel: 'manifest', href: '/manifest.json' },
    ],
    scripts: [
      { src: '/sw-register.js', defer: true },
    ],
    title: '模拟仲裁庭',
  }),
  component: RootComponent,
})

function RootComponent() {
  const isDev = typeof process !== 'undefined'
    ? process.env.NODE_ENV !== 'production'
    : !!(typeof window !== 'undefined' && (window as any).__vite_dev__)

  return (
    <html lang="zh-CN" suppressHydrationWarning>
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta name="theme-color" content="#0a0a0a" />
        <meta name="mobile-web-app-capable" content="yes" />
        <meta name="apple-mobile-web-app-capable" content="yes" />
        <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />
        <meta name="apple-mobile-web-app-title" content="模拟仲裁庭" />
        <link rel="icon" href="/icon.svg" />
        <link rel="apple-touch-icon" href="/apple-icon.svg" />
        <link rel="manifest" href="/manifest.json" />
        <title>模拟仲裁庭</title>
      </head>
      <body className="antialiased">
        {import.meta.env.DEV && (
          <script type="module" src="/@id/virtual:tanstack-start-dev-client-entry"></script>
        )}
        <script src="/sw-register.js" defer />
        <Providers>
          <BreadcrumbProvider>
            <ShellLayout>
              <Outlet />
            </ShellLayout>
          </BreadcrumbProvider>
        </Providers>
      </body>
    </html>
  )
}
