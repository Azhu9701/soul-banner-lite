import { Link } from "@tanstack/react-router";
import { cn } from "@/lib/utils";
import type { NavItem as NavItemType } from "@/config/nav";

interface NavItemProps {
  item: NavItemType;
  active: boolean;
}

export function NavItem({ item, active }: NavItemProps) {
  const Icon = item.icon;
  return (
    <Link
      to={item.href}
      data-testid={`nav-${item.key}`}
      className={cn(
        "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
        active
          ? "bg-primary/10 text-primary"
          : "text-muted-foreground hover:bg-muted hover:text-foreground"
      )}
    >
      <Icon className="h-4 w-4 shrink-0" />
      <span className="truncate">{item.label}</span>
    </Link>
  );
}
