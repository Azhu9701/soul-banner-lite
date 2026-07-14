import { Link } from "@tanstack/react-router";
import { SidebarTitle } from "@/components/sidebar-title";

export function SidebarLogo() {
  return (
    <Link
      to="/"
      className="flex h-14 items-center gap-2 border-b px-4 shrink-0"
      data-testid="sidebar-logo"
    >
      <svg width="28" height="28" viewBox="0 0 28 28" fill="none" className="h-6 w-6 text-primary">
        <ellipse cx="14" cy="17" rx="10" ry="3.5" stroke="currentColor" strokeWidth="1.2" fill="currentColor" fillOpacity="0.05" />
        <path d="M10 17 Q10 8 14 6 Q18 8 18 17" stroke="currentColor" strokeWidth="1.2" fill="currentColor" fillOpacity="0.3" />
        <path d="M10.5 20 Q10.5 22 14 23.5 Q17.5 22 17.5 20" stroke="currentColor" strokeWidth="1.2" fill="none" />
      </svg>
      <SidebarTitle />
    </Link>
  );
}
