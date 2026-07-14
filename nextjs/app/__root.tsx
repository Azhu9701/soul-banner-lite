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
    title: '模拟仲裁庭',
  }),
  component: RootComponent,
})

function RootComponent() {
  return (
    <html lang="zh-CN" suppressHydrationWarning>
      <head>
        <meta name="mobile-web-app-capable" content="yes" />
      </head>
      <body className="antialiased">
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
