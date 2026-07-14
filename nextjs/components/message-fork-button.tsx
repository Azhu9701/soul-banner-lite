"use client";

import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { GitFork, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { forkSession } from "@/lib/api";

export function MessageForkButton({
  sessionId,
  messageSeq,
}: {
  sessionId: string;
  messageSeq: number;
}) {
  const navigate = useNavigate();
  const [forking, setForking] = useState(false);

  const onFork = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setForking(true);
    try {
      const result = await forkSession(sessionId, messageSeq);
      navigate({ to: `/sessions/${result.session_id}?fork=true` });
    } catch (e: unknown) {
      console.error("Fork failed:", e);
      setForking(false);
    }
  };

  return (
    <Button
      variant="ghost"
      size="icon"
      className="h-7 w-7"
      onClick={onFork}
      disabled={forking}
      title="从这条消息重新出发"
    >
      {forking ? (
        <Loader2 className="h-3.5 w-3.5 animate-spin" />
      ) : (
        <GitFork className="h-3.5 w-3.5" />
      )}
    </Button>
  );
}
