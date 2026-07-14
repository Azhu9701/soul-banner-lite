import { Link } from "@tanstack/react-router";
import { AlertTriangle, CheckCircle, TrendingDown } from "lucide-react";
import type { SoulAlert, BoundaryReview } from "@/lib/api";

interface AlertPanelProps {
  unsummoned: SoulAlert[];
  lowEffectiveness: BoundaryReview[];
}

export function AlertPanel({ unsummoned, lowEffectiveness }: AlertPanelProps) {
  const total = unsummoned.length + lowEffectiveness.length;

  return (
    <div data-testid="alert-panel">
      <h3 className="text-sm font-semibold mb-3">告警 ({total})</h3>
      {total === 0 && (
        <p className="text-sm text-green-600 flex items-center gap-1">
          <CheckCircle className="h-4 w-4" />✓ 一切正常
        </p>
      )}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {unsummoned.length > 0 && (
          <div>
            <h4 className="text-xs font-semibold text-red-500 flex items-center gap-1 mb-2">
              <AlertTriangle className="h-3 w-3" />
              未召唤 ({unsummoned.length})
            </h4>
            <div className="space-y-1">
              {unsummoned.map((a) => (
                <Link
                  key={a.soul_name}
                  to={`/souls/${encodeURIComponent(a.soul_name)}` as any}
                  className="block rounded-md border border-red-200 bg-red-50 dark:bg-red-950 px-3 py-1.5 text-sm hover:border-red-300 transition-colors"
                >
                  <span className="font-medium">{a.soul_name}</span>
                  <span className="text-xs text-muted-foreground ml-2">
                    {a.detail}
                  </span>
                </Link>
              ))}
            </div>
          </div>
        )}
        {lowEffectiveness.length > 0 && (
          <div>
            <h4 className="text-xs font-semibold text-yellow-500 flex items-center gap-1 mb-2">
              <TrendingDown className="h-3 w-3" />
              低效检测 ({lowEffectiveness.length})
            </h4>
            <div className="space-y-1">
              {lowEffectiveness.map((b) => (
                <Link
                  key={b.soul_name}
                  to={`/souls/${encodeURIComponent(b.soul_name)}` as any}
                  className="block rounded-md border border-yellow-200 bg-yellow-50 dark:bg-yellow-950 px-3 py-1.5 text-sm hover:border-yellow-300 transition-colors"
                >
                  <span className="font-medium">{b.soul_name}</span>
                  <span className="text-xs text-muted-foreground ml-2">
                    有效率 {(b.effective_rate * 100).toFixed(0)}% · {b.recommendation}
                  </span>
                </Link>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
