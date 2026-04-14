import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Search, X, AudioLines, Timer } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useEditorStore } from "@/stores/editorStore";

interface ContextMenuState {
  visible: boolean;
  x: number;
  y: number;
  wordIndex: number;
}

// Speaker colors for visual differentiation
const SPEAKER_COLORS = [
  "#8B5CF6", // violet
  "#06B6D4", // cyan
  "#F97316", // orange
  "#10B981", // emerald
  "#EC4899", // pink
  "#6366F1", // indigo
  "#14B8A6", // teal
  "#F59E0B", // amber
];

function getSpeakerColor(speakerId: number): string {
  if (speakerId < 0) return "transparent";
  return SPEAKER_COLORS[speakerId % SPEAKER_COLORS.length];
}

/** Map confidence (0-1) to an underline style */
function getConfidenceStyle(confidence: number): React.CSSProperties {
  if (confidence >= 0.9) return {};
  if (confidence >= 0.7)
    return {
      textDecorationLine: "underline",
      textDecorationStyle: "dotted",
      textDecorationColor: "rgba(234, 179, 8, 0.5)",
      textUnderlineOffset: "3px",
    };
  return {
    textDecorationLine: "underline",
    textDecorationStyle: "wavy",
    textDecorationColor: "rgba(239, 68, 68, 0.6)",
    textUnderlineOffset: "3px",
  };
}

interface PauseInfo {
  after_word_index: number;
  gap_duration_us: number;
}

interface FillerAnalysis {
  filler_indices: number[];
  pauses: PauseInfo[];
  filler_count: number;
  pause_count: number;
}

interface TranscriptEditorProps {
  showConfidence?: boolean;
  showSpeakers?: boolean;
  onWordClick?: (index: number) => void;
}

const TranscriptEditor: React.FC<TranscriptEditorProps> = ({
  showConfidence = true,
  showSpeakers = true,
  onWordClick,
}) => {
  const { t } = useTranslation();
  const {
    words,
    selectedIndex,
    selectionRange,
    highlightedIndices,
    highlightType,
    deleteWord,
    restoreWord,
    silenceWord,
    splitWord,
    deleteRange,
    restoreAll,
    undo,
    redo,
    selectWord,
    setSelectionRange,
    setWords,
    setHighlightedIndices,
    clearHighlights,
  } = useEditorStore();

  const containerRef = useRef<HTMLDivElement>(null);
  const findInputRef = useRef<HTMLInputElement>(null);
  const dragStartRef = useRef<number | null>(null);
  const isDraggingRef = useRef(false);
  const [showFind, setShowFind] = useState(false);
  const [findQuery, setFindQuery] = useState("");
  const [findMatchIndex, setFindMatchIndex] = useState(0);
  const [isDetecting, setIsDetecting] = useState(false);
  const [contextMenu, setContextMenu] = useState<ContextMenuState>({
    visible: false,
    x: 0,
    y: 0,
    wordIndex: -1,
  });

  const highlightedSet = useMemo(() => new Set(highlightedIndices), [highlightedIndices]);

  const closeContextMenu = useCallback(() => {
    setContextMenu((prev) => ({ ...prev, visible: false }));
  }, []);

  // Detect fillers — highlights filler words in the transcript
  const handleDetectFillers = useCallback(async () => {
    setIsDetecting(true);
    try {
      const result = await invoke<FillerAnalysis>("analyze_fillers", {});
      if (result.filler_indices.length > 0) {
        setHighlightedIndices(result.filler_indices, "filler");
      } else {
        clearHighlights();
      }
    } catch (err) {
      console.error("Filler detection failed:", err);
    } finally {
      setIsDetecting(false);
    }
  }, [setHighlightedIndices, clearHighlights]);

  // Detect pauses — highlights words adjacent to long pauses
  const handleDetectPauses = useCallback(async () => {
    setIsDetecting(true);
    try {
      const result = await invoke<FillerAnalysis>("analyze_fillers", {});
      if (result.pauses.length > 0) {
        const pauseWordIndices = result.pauses.map((p: PauseInfo) => p.after_word_index);
        setHighlightedIndices(pauseWordIndices, "pause");
      } else {
        clearHighlights();
      }
    } catch (err) {
      console.error("Pause detection failed:", err);
    } finally {
      setIsDetecting(false);
    }
  }, [setHighlightedIndices, clearHighlights]);

  const isInSelectionRange = useCallback(
    (index: number): boolean => {
      if (!selectionRange) return false;
      const [start, end] = selectionRange;
      return index >= start && index <= end;
    },
    [selectionRange],
  );

  // Drag-select handlers
  const handleWordMouseDown = useCallback(
    (index: number, e: React.MouseEvent) => {
      if (e.button !== 0) return; // left click only
      dragStartRef.current = index;
      isDraggingRef.current = false;
      closeContextMenu();
    },
    [closeContextMenu],
  );

  const handleWordMouseEnter = useCallback(
    (index: number) => {
      if (dragStartRef.current === null) return;
      isDraggingRef.current = true;
      const start = Math.min(dragStartRef.current, index);
      const end = Math.max(dragStartRef.current, index);
      selectWord(dragStartRef.current);
      setSelectionRange([start, end]);
    },
    [selectWord, setSelectionRange],
  );

  const handleWordMouseUp = useCallback(
    (index: number, e: React.MouseEvent) => {
      if (dragStartRef.current === null) return;
      if (!isDraggingRef.current) {
        // Simple click (no drag) — clear any filler/pause highlights
        if (highlightedIndices.length > 0) {
          clearHighlights();
        }
        if (e.shiftKey && selectedIndex !== null) {
          const start = Math.min(selectedIndex, index);
          const end = Math.max(selectedIndex, index);
          setSelectionRange([start, end]);
        } else {
          selectWord(index);
          onWordClick?.(index);
        }
      }
      dragStartRef.current = null;
      isDraggingRef.current = false;
    },
    [selectedIndex, selectWord, setSelectionRange, onWordClick, highlightedIndices, clearHighlights],
  );

  // Clear drag on global mouseup (in case mouse leaves the container)
  useEffect(() => {
    const handleGlobalMouseUp = () => {
      dragStartRef.current = null;
      isDraggingRef.current = false;
    };
    window.addEventListener("mouseup", handleGlobalMouseUp);
    return () => window.removeEventListener("mouseup", handleGlobalMouseUp);
  }, []);

  // Find matches
  const findMatches = useMemo(() => {
    if (!findQuery.trim()) return [];
    const q = findQuery.toLowerCase();
    return words
      .map((w, i) => ({ index: i, match: w.text.toLowerCase().includes(q) }))
      .filter((m) => m.match)
      .map((m) => m.index);
  }, [words, findQuery]);

  // Ctrl+F to toggle find bar
  useEffect(() => {
    const handleFind = (e: KeyboardEvent) => {
      if (e.key === "f" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        setShowFind((prev) => {
          if (!prev) setTimeout(() => findInputRef.current?.focus(), 50);
          return !prev;
        });
      }
    };
    window.addEventListener("keydown", handleFind);
    return () => window.removeEventListener("keydown", handleFind);
  }, []);

  const navigateFind = useCallback(
    (direction: 1 | -1) => {
      if (findMatches.length === 0) return;
      const next = (findMatchIndex + direction + findMatches.length) % findMatches.length;
      setFindMatchIndex(next);
      selectWord(findMatches[next]);
    },
    [findMatches, findMatchIndex, selectWord],
  );

  const handleDeleteAllMatches = useCallback(async () => {
    if (findMatches.length === 0) return;
    for (const idx of findMatches) {
      await deleteWord(idx);
    }
    setFindQuery("");
    setShowFind(false);
  }, [findMatches, deleteWord]);

  const handleContextMenu = useCallback(
    (index: number, e: React.MouseEvent) => {
      e.preventDefault();
      selectWord(index);
      setContextMenu({ visible: true, x: e.clientX, y: e.clientY, wordIndex: index });
    },
    [selectWord],
  );

  const handleDeleteSelected = useCallback(async () => {
    if (highlightedIndices.length > 0) {
      // Bulk-delete highlighted words (fillers or pause-adjacent)
      if (highlightType === "filler") {
        const count = await invoke<number>("delete_fillers", {});
        if (count > 0) {
          const updated = await invoke<typeof words>("editor_get_words", {});
          await setWords(updated);
        }
      } else {
        // Delete each highlighted word individually
        for (const idx of highlightedIndices) {
          await deleteWord(idx);
        }
      }
      clearHighlights();
    } else if (selectionRange) {
      await deleteRange(selectionRange[0], selectionRange[1]);
    } else if (selectedIndex !== null) {
      await deleteWord(selectedIndex);
    }
    closeContextMenu();
  }, [selectedIndex, selectionRange, highlightedIndices, highlightType, deleteWord, deleteRange, closeContextMenu, clearHighlights, setWords]);

  const handleRestoreSelected = useCallback(async () => {
    if (selectedIndex !== null) {
      await restoreWord(selectedIndex);
    }
    closeContextMenu();
  }, [selectedIndex, restoreWord, closeContextMenu]);

  const handleSilenceSelected = useCallback(async () => {
    if (selectedIndex !== null) {
      await silenceWord(selectedIndex);
    }
    closeContextMenu();
  }, [selectedIndex, silenceWord, closeContextMenu]);

  const handleSplitSelected = useCallback(async () => {
    if (selectedIndex !== null) {
      const word = words[selectedIndex];
      if (word) {
        const midpoint = Math.floor(word.text.length / 2);
        if (midpoint > 0) {
          await splitWord(selectedIndex, midpoint);
        }
      }
    }
    closeContextMenu();
  }, [selectedIndex, words, splitWord, closeContextMenu]);

  const handleKeyDown = useCallback(
    async (e: React.KeyboardEvent) => {
      if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        await handleDeleteSelected();
      } else if (e.key === "z" && (e.ctrlKey || e.metaKey) && e.shiftKey) {
        e.preventDefault();
        await redo();
      } else if (e.key === "z" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        await undo();
      }
    },
    [handleDeleteSelected, undo, redo],
  );

  // Close context menu on outside click
  useEffect(() => {
    const handleClick = () => closeContextMenu();
    if (contextMenu.visible) {
      document.addEventListener("click", handleClick);
      return () => document.removeEventListener("click", handleClick);
    }
  }, [contextMenu.visible, closeContextMenu]);

  if (words.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-[rgba(240,240,240,0.6)]">
        <p>{t("editor.noTranscript")}</p>
      </div>
    );
  }

  const contextWord = contextMenu.wordIndex >= 0 ? words[contextMenu.wordIndex] : null;
  const findMatchSet = new Set(findMatches);

  return (
    <div
      ref={containerRef}
      className="relative p-4 outline-none select-none"
      tabIndex={0}
      onKeyDown={handleKeyDown}
    >
      {/* Detection toolbar — above transcript */}
      <div className="flex items-center gap-2 mb-3">
        <button
          onClick={handleDetectFillers}
          disabled={isDetecting}
          className={`flex items-center gap-1 px-2 py-1 rounded text-[11px] border transition-colors disabled:opacity-50 ${
            highlightType === "filler"
              ? "border-[#EEEEEE]/50 text-[#EEEEEE] bg-[#EEEEEE]/10"
              : "bg-background border-mid-gray/20 text-mid-gray hover:bg-mid-gray/10"
          }`}
        >
          <AudioLines size={12} />
          {t("editor.detectFillers")}
        </button>
        <button
          onClick={handleDetectPauses}
          disabled={isDetecting}
          className={`flex items-center gap-1 px-2 py-1 rounded text-[11px] border transition-colors disabled:opacity-50 ${
            highlightType === "pause"
              ? "border-[#EEEEEE]/50 text-[#EEEEEE] bg-[#EEEEEE]/10"
              : "bg-background border-mid-gray/20 text-mid-gray hover:bg-mid-gray/10"
          }`}
        >
          <Timer size={12} />
          {t("editor.detectPauses")}
        </button>
        {highlightedIndices.length > 0 && (
          <>
            <span className="text-[11px] text-mid-gray/60">
              {highlightedIndices.length} {highlightType === "filler" ? t("editor.fillersFound") : t("editor.pausesFound")}
            </span>
            <button
              onClick={() => clearHighlights()}
              className="text-[11px] text-mid-gray/60 hover:text-mid-gray transition-colors"
            >
              <X size={12} />
            </button>
          </>
        )}
      </div>

      {/* Find bar */}
      {showFind && (
        <div className="flex items-center gap-2 mb-3 p-2 rounded-lg bg-[#1E1E1E] border border-mid-gray/20">
          <Search size={14} className="text-mid-gray/60 shrink-0" />
          <input
            ref={findInputRef}
            type="text"
            value={findQuery}
            onChange={(e) => {
              setFindQuery(e.target.value);
              setFindMatchIndex(0);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") navigateFind(e.shiftKey ? -1 : 1);
              if (e.key === "Escape") {
                setShowFind(false);
                setFindQuery("");
              }
            }}
            placeholder={t("editor.findPlaceholder")}
            className="flex-1 bg-transparent text-sm text-[#F0F0F0] outline-none placeholder:text-mid-gray/40"
          />
          {findMatches.length > 0 && (
            <span className="text-[11px] text-mid-gray/60 shrink-0">
              {findMatchIndex + 1}/{findMatches.length}
            </span>
          )}
          {findMatches.length > 0 && (
            <button
              onClick={handleDeleteAllMatches}
              className="px-2 py-0.5 text-[11px] text-red-400 bg-red-900/20 rounded hover:bg-red-900/40 transition-colors"
            >
              {t("editor.deleteAll")}
            </button>
          )}
          <button
            onClick={() => {
              setShowFind(false);
              setFindQuery("");
            }}
            className="text-mid-gray/60 hover:text-mid-gray transition-colors"
          >
            <X size={14} />
          </button>
        </div>
      )}

      {/* Word spans */}
      <div className="flex flex-wrap gap-1 leading-relaxed">
        {words.map((word, index) => {
          const isSelected = selectedIndex === index;
          const isRangeSelected = isInSelectionRange(index);
          const isFindMatch = findMatchSet.has(index);
          const isCurrentFindMatch = findMatches.length > 0 && findMatches[findMatchIndex] === index;
          const isHighlighted = highlightedSet.has(index);
          const prevWord = index > 0 ? words[index - 1] : null;
          const showSpeakerLabel =
            showSpeakers &&
            word.speaker_id >= 0 &&
            (!prevWord || prevWord.speaker_id !== word.speaker_id);

          const confidenceStyle =
            showConfidence && !word.deleted ? getConfidenceStyle(word.confidence) : {};
          const speakerBorderStyle =
            showSpeakers && word.speaker_id >= 0
              ? { borderLeft: `2px solid ${getSpeakerColor(word.speaker_id)}`, paddingLeft: "3px" }
              : {};

          return (
            <React.Fragment key={`${index}-${word.start_us}`}>
              {showSpeakerLabel && (
                <div className="w-full mt-2 mb-0.5 flex items-center gap-1.5">
                  <span
                    className="inline-block w-2 h-2 rounded-full"
                    style={{ backgroundColor: getSpeakerColor(word.speaker_id) }}
                  />
                  <span className="text-[10px] uppercase tracking-wider text-mid-gray/60">
                    {t("editor.speaker", { id: word.speaker_id + 1 })}
                  </span>
                </div>
              )}
              <span
                role="button"
                tabIndex={-1}
                onMouseDown={(e) => handleWordMouseDown(index, e)}
                onMouseEnter={() => handleWordMouseEnter(index)}
                onMouseUp={(e) => handleWordMouseUp(index, e)}
                onContextMenu={(e) => handleContextMenu(index, e)}
                style={{ ...confidenceStyle, ...speakerBorderStyle }}
                title={
                  isHighlighted && highlightType === "filler"
                    ? t("editor.fillerWord")
                    : isHighlighted && highlightType === "pause"
                      ? t("editor.pauseDetected")
                      : showConfidence && word.confidence < 0.9
                        ? `${t("editor.confidence")}: ${Math.round(word.confidence * 100)}%`
                        : undefined
                }
                className={[
                  "cursor-pointer rounded px-1 py-0.5 transition-colors",
                  word.deleted && "line-through opacity-40",
                  word.silenced && !word.deleted && "opacity-60 italic",
                  isHighlighted && highlightType === "filler" && "bg-red-300/30 text-black",
                  isHighlighted && highlightType === "pause" && "bg-red-300/30 text-black",
                  isCurrentFindMatch && !isHighlighted && "ring-2 ring-[#E8A838] bg-[#E8A838]/30",
                  isFindMatch && !isCurrentFindMatch && !isHighlighted && "bg-[#E8A838]/15",
                  isSelected && !isFindMatch && !isHighlighted && "bg-[#E8A838] text-[#1E1E1E]",
                  isRangeSelected && !isSelected && !isFindMatch && !isHighlighted && "bg-[#E8A838]/40",
                  !isSelected && !isRangeSelected && !isFindMatch && !isHighlighted && !word.deleted && !word.silenced && "hover:bg-[rgba(128,128,128,0.2)]",
                ]
                  .filter(Boolean)
                  .join(" ")}
              >
                {word.text}
              </span>
            </React.Fragment>
          );
        })}
      </div>

      {/* Context menu */}
      {contextMenu.visible && (
        <div
          className="fixed z-50 min-w-[160px] rounded-md border border-[rgba(128,128,128,0.2)] bg-[#252525] py-1 shadow-lg"
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          {contextWord && !contextWord.deleted && (
            <button
              className="w-full px-3 py-1.5 text-left text-sm text-[#F0F0F0] hover:bg-[rgba(128,128,128,0.2)]"
              onClick={handleDeleteSelected}
            >
              {selectionRange ? t("editor.deleteRange") : t("editor.deleteWord")}
            </button>
          )}
          {contextWord && contextWord.deleted && (
            <button
              className="w-full px-3 py-1.5 text-left text-sm text-[#F0F0F0] hover:bg-[rgba(128,128,128,0.2)]"
              onClick={handleRestoreSelected}
            >
              {t("editor.restoreWord")}
            </button>
          )}
          {contextWord && !contextWord.deleted && (
            <button
              className="w-full px-3 py-1.5 text-left text-sm text-[#F0F0F0] hover:bg-[rgba(128,128,128,0.2)]"
              onClick={handleSilenceSelected}
            >
              {t("editor.silenceWord")}
            </button>
          )}
          {contextWord && !contextWord.deleted && contextWord.text.length > 1 && (
            <button
              className="w-full px-3 py-1.5 text-left text-sm text-[#F0F0F0] hover:bg-[rgba(128,128,128,0.2)]"
              onClick={handleSplitSelected}
            >
              {t("editor.splitWord")}
            </button>
          )}
          <div className="my-1 border-t border-[rgba(128,128,128,0.2)]" />
          <button
            className="w-full px-3 py-1.5 text-left text-sm text-[#F0F0F0] hover:bg-[rgba(128,128,128,0.2)]"
            onClick={async () => {
              await undo();
              closeContextMenu();
            }}
          >
            {t("editor.undo")}
          </button>
          <button
            className="w-full px-3 py-1.5 text-left text-sm text-[#F0F0F0] hover:bg-[rgba(128,128,128,0.2)]"
            onClick={async () => {
              await redo();
              closeContextMenu();
            }}
          >
            {t("editor.redo")}
          </button>
          <div className="my-1 border-t border-[rgba(128,128,128,0.2)]" />
          <button
            className="w-full px-3 py-1.5 text-left text-sm text-[#F0F0F0] hover:bg-[rgba(128,128,128,0.2)]"
            onClick={async () => {
              await restoreAll();
              closeContextMenu();
            }}
          >
            {t("editor.restoreAll")}
          </button>
        </div>
      )}
    </div>
  );
};

export default TranscriptEditor;
