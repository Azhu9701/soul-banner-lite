"use client";

import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { DeleteSoulConfirmDialog } from "@/components/delete-soul-confirm-dialog";

interface DeleteSoulButtonProps {
  soulName: string;
  variant?: "icon" | "text";
  className?: string;
}

export function DeleteSoulButton({ soulName, variant = "icon", className }: DeleteSoulButtonProps) {
  const [open, setOpen] = useState(false);
  const navigate = useNavigate();

  const handleDeleted = () => {
    navigate({ to: "/souls" });
  };

  if (variant === "text") {
    return (
      <>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setOpen(true)}
          className={className}
          title="删除角色"
        >
          <Trash2 className="h-4 w-4 text-red-500" />
        </Button>
        <DeleteSoulConfirmDialog
          open={open}
          onOpenChange={setOpen}
          soulName={soulName}
          onDeleted={handleDeleted}
        />
      </>
    );
  }

  return (
    <>
      <Button
        variant="ghost"
        size="icon"
        className={className}
        onClick={() => setOpen(true)}
        title="删除角色"
      >
        <Trash2 className="h-4 w-4 text-red-500" />
      </Button>
      <DeleteSoulConfirmDialog
        open={open}
        onOpenChange={setOpen}
        soulName={soulName}
        onDeleted={handleDeleted}
      />
    </>
  );
}
