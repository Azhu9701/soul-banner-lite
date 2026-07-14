import { createFileRoute, Outlet, useLocation } from '@tanstack/react-router'
import { Suspense } from "react"
import { PossessionEntry } from "@/components/possession-entry"

export const Route = createFileRoute('/possess')({
  component: PossessLayout,
})

function PossessLayout() {
  const location = useLocation()
  const isChildRoute = location.pathname !== '/possess'

  if (isChildRoute) {
    return <Outlet />
  }

  return <PossessPage />
}

function PossessPage() {
  return (
    <div className="space-y-6">
      <Suspense>
        <PossessionEntry />
      </Suspense>
    </div>
  );
}
