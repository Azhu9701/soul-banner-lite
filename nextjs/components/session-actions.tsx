"use client";

import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Pencil, Trash2, Check, X, Download } from "lucide-react";
import { ConfirmButton } from "@/components/ui/confirm-button";
import { renameSession, deleteSession, exportSessionMarkdown } from "@/lib/api";
import { triggerSessionsUpdate } from "@/components/sidebar-sessions";

export default function SessionActions({ sessionId, title }: { sessionId: string; title: string }) {
  const navigate = useNavigate();
  const [editing, setEditing] = useState(false);
  const [newTitle, setNewTitle] = useState(title);
  const [error, setError] = useState("");
  const [exporting, setExporting] = useState(false);

  const onRename = async () => {
    if (!newTitle.trim()) return;
    try { await renameSession(sessionId, newTitle.trim()); triggerSessionsUpdate(); setEditing(false); }
    catch (e: unknown) { setError(e instanceof Error ? e.message : String(e)); }
  };

  const onExport = async () => {
    setExporting(true);
    setError("");
    try { await exportSessionMarkdown(sessionId, title); }
    catch (e: unknown) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setExporting(false); }
  };

  if (editing) {
    return (
      <div className="flex items-center gap-2">
        <Input value={newTitle} onChange={(e) => setNewTitle(e.target.value)} className="w-48" autoFocus />
        <Button size="sm" onClick={onRename}><Check className="h-4 w-4" /></Button>
        <Button size="sm" variant="ghost" onClick={() => setEditing(false)}><X className="h-4 w-4" /></Button>
        {error && <span className="text-xs text-red-500">{error}</span>}
      </div>
    );
  }

  return (
    <div className="flex items-center gap-1">
      <Button variant="ghost" size="icon" onClick={onExport} disabled={exporting} title="导出 Markdown">
        <Download className="h-4 w-4" />
      </Button>
      <Button variant="ghost" size="icon" onClick={() => setEditing(true)} title="重命名">
        <Pencil className="h-4 w-4" />
      </Button>
      <ConfirmButton
        icon={<Trash2 className="h-4 w-4 text-red-500" />}
        confirmText="确认删除"
        title="删除会话"
        onConfirm={async () => { await deleteSession(sessionId); triggerSessionsUpdate(); navigate({ to: "/sessions" }); }}
      />
      {error && <span className="text-xs text-red-500 ml-2">{error}</span>}
    </div>
  );
}
