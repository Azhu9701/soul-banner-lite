"use client";

import { Link, useLocation } from "@tanstack/react-router";
import { ChevronRight } from "lucide-react";
import { useBreadcrumb } from "@/contexts/breadcrumb-context";

const labels: Record<string, string> = {
  souls: "角色",
  possess: "开庭",
  sessions: "庭审记录",
  analytics: "庭审统计",
};

export function Breadcrumb() {
  const location = useLocation();
  const pathname = location.pathname;
  const { lastLabel } = useBreadcrumb();
  const segments = pathname.split("/").filter(Boolean);

  if (segments.length === 0) return null;

  const isLast = (i: number) => i === segments.length - 1;

  return (
    <nav data-testid="breadcrumb" aria-label="面包屑">
      <ol className="flex items-center gap-1 text-sm text-muted-foreground">
        <li>
          <Link to="/" className="hover:text-foreground transition-colors">
            首页
          </Link>
        </li>
        {segments.map((seg, i) => {
          const rawLabel = isLast(i) && lastLabel
            ? lastLabel
            : labels[seg] || decodeURIComponent(seg);
          const displayLabel = rawLabel.length > 30
            ? rawLabel.slice(0, 30) + "…"
            : rawLabel;

          return (
            <li key={i} className="flex items-center gap-1">
              <ChevronRight className="h-3 w-3" />
              <Link
                to={`/${segments.slice(0, i + 1).join("/")}` as any}
                className="hover:text-foreground transition-colors capitalize"
              >
                {displayLabel}
              </Link>
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
