import { Link } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";

export function QuickActions() {
  return (
    <Link to="/possess" data-testid="new-possession-btn">
      <Button size="sm">
        <svg width="20" height="20" viewBox="0 0 28 28" fill="none" className="h-4 w-4">
          <ellipse cx="14" cy="17" rx="10" ry="3.5" stroke="currentColor" strokeWidth="1.2" fill="currentColor" fillOpacity="0.05" />
          <path d="M10 17 Q10 8 14 6 Q18 8 18 17" stroke="currentColor" strokeWidth="1.2" fill="currentColor" fillOpacity="0.3" />
          <path d="M10.5 20 Q10.5 22 14 23.5 Q17.5 22 17.5 20" stroke="currentColor" strokeWidth="1.2" fill="none" />
        </svg>
      </Button>
    </Link>
  );
}
