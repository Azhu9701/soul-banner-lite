import { Link } from "@tanstack/react-router";
import type { SoulListEntry } from "@/lib/api";
import { DeleteSoulButton } from "@/components/delete-soul-button";
import { Users } from "lucide-react";

export function SoulCard({ soul }: { soul: SoulListEntry }) {
  return (
    <Link
      to="/souls/$name"
      params={{ name: encodeURIComponent(soul.name) }}
      data-testid={`soul-card-${soul.name}`}
      className="group relative flex flex-col gap-3 rounded-lg border bg-background p-4 transition-all hover:-translate-y-0.5 hover:shadow-md hover:border-primary/30"
    >
      <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
        <DeleteSoulButton soulName={soul.name} />
      </div>
      <div className="flex items-start justify-between">
        <span className="text-xs text-muted-foreground font-mono">
          {soul.ismism_code}
        </span>
      </div>
      <h3 className="text-lg font-semibold truncate pr-8">{soul.name}</h3>
      <p className="text-sm text-muted-foreground">{soul.field}</p>
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <span className="flex items-center gap-1">
          <Users className="h-3 w-3" aria-hidden="true" />
          {soul.summon_count}
        </span>
        {soul.tags.length > 0 && (
          <span className="truncate max-w-[60%] text-right">
            {soul.tags.slice(0, 3).join(", ")}
          </span>
        )}
      </div>
    </Link>
  );
}
