import { Link } from "@tanstack/react-router";
import { Play } from "lucide-react";
import { Button } from "@/components/ui/button";

interface SummonButtonProps {
  soulName: string;
}

export function SummonButton({ soulName }: SummonButtonProps) {
  return (
    <Link
      to="/possess"
      search={{ preset: 'single', souls: encodeURIComponent(soulName) }}
      data-testid={`summon-btn-${soulName}`}
    >
      <Button size="sm">
        <Play className="mr-1 h-4 w-4" />
        召唤此角色
      </Button>
    </Link>
  );
}
