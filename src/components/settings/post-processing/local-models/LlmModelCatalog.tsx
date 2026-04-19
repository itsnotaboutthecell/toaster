import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Button } from "../../../ui/Button";
import { Alert } from "../../../ui/Alert";
import { useSettings } from "../../../../hooks/useSettings";

// Local hand-written types — these mirror `commands/llm_models.rs` and will
// be regenerated into `src/bindings.ts` on the next `cargo tauri dev`.
type LlmModelInfo = {
  id: string;
  display_name: string;
  size_label: string;
  ram_required_mb: number;
  disk_required_mb: number;
  is_default: boolean;
  is_downloaded: boolean;
  is_selected: boolean;
};

type DownloadProgressPayload = {
  asset_kind: "llm";
  model_id: string;
  bytes_downloaded: number;
  bytes_total: number;
  bytes_per_second: number;
};

type DownloadCompletedPayload = {
  asset_kind: "llm";
  model_id: string;
};

type DownloadFailedPayload = {
  asset_kind: "llm";
  model_id: string;
  error: string;
};

const PROGRESS_EVENT = "llm-model-download-progress";
const COMPLETED_EVENT = "llm-model-download-completed";
const FAILED_EVENT = "llm-model-download-failed";
const CANCELLED_EVENT = "llm-model-download-cancelled";
const DELETED_EVENT = "llm-model-deleted";

type Progress = {
  bytesDownloaded: number;
  bytesTotal: number;
  bytesPerSecond: number;
};

export const LlmModelCatalog: React.FC = () => {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettings();
  const [entries, setEntries] = useState<LlmModelInfo[]>([]);
  const [progress, setProgress] = useState<Record<string, Progress>>({});
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const list = await invoke<LlmModelInfo[]>("list_llm_models");
      setEntries(list);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    const unlisten: Array<Promise<() => void>> = [
      listen<DownloadProgressPayload>(PROGRESS_EVENT, (event) => {
        if (event.payload.asset_kind !== "llm") return;
        setProgress((prev) => ({
          ...prev,
          [event.payload.model_id]: {
            bytesDownloaded: event.payload.bytes_downloaded,
            bytesTotal: event.payload.bytes_total,
            bytesPerSecond: event.payload.bytes_per_second,
          },
        }));
      }),
      listen<DownloadCompletedPayload>(COMPLETED_EVENT, (event) => {
        if (event.payload.asset_kind !== "llm") return;
        setProgress((prev) => {
          const next = { ...prev };
          delete next[event.payload.model_id];
          return next;
        });
        void refresh();
      }),
      listen<DownloadFailedPayload>(FAILED_EVENT, (event) => {
        if (event.payload.asset_kind !== "llm") return;
        setError(event.payload.error);
        setProgress((prev) => {
          const next = { ...prev };
          delete next[event.payload.model_id];
          return next;
        });
      }),
      listen(CANCELLED_EVENT, () => void refresh()),
      listen(DELETED_EVENT, () => void refresh()),
    ];
    return () => {
      unlisten.forEach((p) => void p.then((fn) => fn()));
    };
  }, [refresh]);

  const selectedId = settings?.local_llm_model_id ?? null;

  const handleDownload = useCallback(async (entry: LlmModelInfo) => {
    setBusyId(entry.id);
    setError(null);
    try {
      await invoke("download_llm_model", { modelId: entry.id });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusyId(null);
    }
  }, []);

  const handleCancel = useCallback(async (entry: LlmModelInfo) => {
    try {
      await invoke("cancel_llm_download", { modelId: entry.id });
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const handleDelete = useCallback(async (entry: LlmModelInfo) => {
    setBusyId(entry.id);
    try {
      await invoke("delete_llm_model", { modelId: entry.id });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusyId(null);
    }
  }, []);

  const handleSelect = useCallback(
    async (entry: LlmModelInfo) => {
      try {
        await invoke("set_selected_llm_model", { modelId: entry.id });
        updateSetting("local_llm_model_id", entry.id);
        await refresh();
      } catch (e) {
        setError(String(e));
      }
    },
    [refresh, updateSetting],
  );

  const sorted = useMemo(
    () => [...entries].sort((a, b) => a.ram_required_mb - b.ram_required_mb),
    [entries],
  );

  return (
    <div className="space-y-3">
      <p className="text-sm text-text-secondary px-4">
        {t("settings.postProcessing.localModels.description")}
      </p>
      {error && (
        <div className="px-4">
          <Alert variant="error">{error}</Alert>
        </div>
      )}
      <div className="space-y-2 px-4">
        {sorted.map((entry) => {
          const p = progress[entry.id];
          const isDownloading = !!p;
          const isSelected = selectedId === entry.id;
          return (
            <div
              key={entry.id}
              className="flex items-center justify-between gap-4 rounded border border-border-subtle bg-surface-elevated px-3 py-2"
            >
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-text-primary">
                    {entry.display_name}
                  </span>
                  {entry.is_default && (
                    <span className="text-xs text-text-secondary">
                      {t("settings.postProcessing.localModels.defaultBadge")}
                    </span>
                  )}
                  {isSelected && (
                    <span className="text-xs text-accent">
                      {t("settings.postProcessing.localModels.selectedBadge")}
                    </span>
                  )}
                </div>
                <div className="text-xs text-text-secondary">
                  {t("settings.postProcessing.localModels.requirementsFormat", {
                    size: entry.size_label,
                    ram: entry.ram_required_mb,
                    disk: entry.disk_required_mb,
                  })}
                </div>
                {isDownloading && (
                  <div className="mt-1 text-xs text-text-secondary">
                    {t(
                      "settings.postProcessing.localModels.progressFormat",
                      {
                        percent: Math.round(
                          (p.bytesDownloaded / Math.max(1, p.bytesTotal)) * 100,
                        ),
                        speed: (p.bytesPerSecond / 1_000_000).toFixed(2),
                      },
                    )}
                  </div>
                )}
              </div>
              <div className="flex items-center gap-2">
                {entry.is_downloaded ? (
                  <>
                    {!isSelected && (
                      <Button
                        variant="primary"
                        size="sm"
                        disabled={busyId === entry.id}
                        onClick={() => void handleSelect(entry)}
                      >
                        {t("settings.postProcessing.localModels.select")}
                      </Button>
                    )}
                    <Button
                      variant="secondary"
                      size="sm"
                      disabled={busyId === entry.id || isSelected}
                      onClick={() => void handleDelete(entry)}
                    >
                      {t("settings.postProcessing.localModels.delete")}
                    </Button>
                  </>
                ) : isDownloading ? (
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => void handleCancel(entry)}
                  >
                    {t("settings.postProcessing.localModels.cancel")}
                  </Button>
                ) : (
                  <Button
                    variant="primary"
                    size="sm"
                    disabled={busyId === entry.id}
                    onClick={() => void handleDownload(entry)}
                  >
                    {t("settings.postProcessing.localModels.download")}
                  </Button>
                )}
              </div>
            </div>
          );
        })}
        {sorted.length === 0 && (
          <p className="text-sm text-text-secondary">
            {t("settings.postProcessing.localModels.empty")}
          </p>
        )}
      </div>
    </div>
  );
};
