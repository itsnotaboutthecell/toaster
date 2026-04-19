import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, FileVideo, Download, Terminal } from "lucide-react";
import { Button } from "@/components/ui/Button";
import type { ExportFormat, MediaType } from "@/bindings";

interface ExportMenuProps {
  mediaType: MediaType | null;
  disabled?: boolean;
  isExportingMedia?: boolean;
  onExportEditedMedia: () => void;
  onExportTranscript: (format: ExportFormat) => void;
  onExportFFmpegScript: () => void;
}

/**
 * Single export entry-point for the editor. Replaces the previous
 * four-location export UI (header [Export] button + EditorToolbar
 * SRT/VTT/Script buttons + FFmpeg script button).
 *
 * Popover lists every available export path; the trigger button is
 * always [Export ▼] so users have exactly one place to look.
 */
const ExportMenu: React.FC<ExportMenuProps> = ({
  mediaType,
  disabled,
  isExportingMedia,
  onExportEditedMedia,
  onExportTranscript,
  onExportFFmpegScript,
}) => {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDocClick = (e: MouseEvent) => {
      if (!containerRef.current) return;
      if (!containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDocClick);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDocClick);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const dispatch = (action: () => void) => {
    setOpen(false);
    action();
  };

  const editedLabel =
    mediaType === "Video"
      ? t("editor.exportMenu.editedVideo")
      : t("editor.exportMenu.editedAudio");

  return (
    <div ref={containerRef} className="relative inline-flex">
      <Button
        variant="brand"
        size="sm"
        onClick={() => setOpen((v) => !v)}
        disabled={disabled}
        className="inline-flex items-center gap-1.5"
        aria-haspopup="menu"
        aria-expanded={open}
      >
        <Download size={14} />
        {isExportingMedia
          ? t("editor.exporting")
          : t("editor.exportMenu.trigger")}
        <ChevronDown size={14} />
      </Button>
      {open && (
        <div
          role="menu"
          className="absolute right-0 top-full mt-1 z-20 w-56 rounded-lg border border-mid-gray/20 bg-background shadow-lg py-1"
        >
          <MenuItem
            icon={<FileVideo size={14} />}
            label={editedLabel}
            disabled={isExportingMedia || !mediaType}
            onClick={() => dispatch(onExportEditedMedia)}
          />
          <div className="my-1 border-t border-mid-gray/10" />
          <MenuItem
            icon={<Download size={14} />}
            label={t("editor.exportMenu.transcriptSrt")}
            onClick={() => dispatch(() => onExportTranscript("Srt"))}
          />
          <MenuItem
            icon={<Download size={14} />}
            label={t("editor.exportMenu.transcriptVtt")}
            onClick={() => dispatch(() => onExportTranscript("Vtt"))}
          />
          <MenuItem
            icon={<Download size={14} />}
            label={t("editor.exportMenu.transcriptScript")}
            onClick={() => dispatch(() => onExportTranscript("Script"))}
          />
          <div className="my-1 border-t border-mid-gray/10" />
          <MenuItem
            icon={<Terminal size={14} />}
            label={t("editor.exportMenu.ffmpegScript")}
            onClick={() => dispatch(onExportFFmpegScript)}
          />
        </div>
      )}
    </div>
  );
};

interface MenuItemProps {
  icon: React.ReactNode;
  label: string;
  disabled?: boolean;
  onClick: () => void;
}

const MenuItem: React.FC<MenuItemProps> = ({ icon, label, disabled, onClick }) => (
  <Button
    variant="ghost"
    size="sm"
    role="menuitem"
    onClick={onClick}
    disabled={disabled}
    className="w-full !justify-start gap-2 !rounded-none !border-0 text-sm"
  >
    <span className="text-mid-gray">{icon}</span>
    {label}
  </Button>
);

export default ExportMenu;
