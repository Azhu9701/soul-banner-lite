"use client";

import { useState, useCallback } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Input } from "@/components/ui/input";
import { Search } from "lucide-react";
import { useDebouncedCallback } from "use-debounce";

interface SoulFilterBarProps {
  totalCount: number;
}

export function SoulFilterBar({ totalCount }: SoulFilterBarProps) {
  const navigate = useNavigate();
  const search = useSearch({ strict: false }) as Record<string, string>;
  const [query, setQuery] = useState(search?.q || "");

  const updateFilter = useCallback(
    (key: string, value: string | null) => {
      const params = new URLSearchParams();
      for (const [k, v] of Object.entries(search || {})) {
        if (v) params.set(k, v);
      }
      if (value) {
        params.set(key, value);
      } else {
        params.delete(key);
      }
      navigate({ to: `/souls?${params.toString()}`, replace: true });
    },
    [navigate, search]
  );

  const debouncedSearch = useDebouncedCallback((value: string) => {
    if (value) {
      updateFilter("q", value);
    } else {
      updateFilter("q", null);
    }
  }, 300);

  return (
    <div
      className="flex flex-col gap-3 sm:flex-row sm:items-center"
      data-testid="soul-filter-bar"
    >
      <div className="relative flex-1">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          placeholder="搜索角色名、领域、标签..."
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            debouncedSearch(e.target.value);
          }}
          className="pl-9"
          data-testid="soul-search-input"
          aria-label="搜索角色"
        />
      </div>
      <span className="text-sm text-muted-foreground whitespace-nowrap">
        共 {totalCount} 角色
      </span>
    </div>
  );
}
