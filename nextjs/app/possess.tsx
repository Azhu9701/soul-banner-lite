import { createFileRoute, Outlet, useLocation } from '@tanstack/react-router'
import { Suspense } from "react"
import { PossessionEntry } from "@/components/possession-entry"

export const Route = createFileRoute('/possess')({
  component: PossessPage,
})

function PossessPage() {
  const location = useLocation()
  if (location.pathname !== '/possess') {
    return <Outlet />
  }

  return (
    <div className="space-y-6">
      <Suspense>
        <PossessionEntry />
      </Suspense>
    </div>
  );
}
