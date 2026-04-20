import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { useShallow } from "zustand/react/shallow";
import { useEditorStore } from "@/stores/editorStore";
import TranscriptContextMenu, { type ContextMenuState } from "./TranscriptContextMenu";
import FindReplaceBar from "./FindReplaceBar";

// Speaker colors for visual differentiation — distinct palette of 8
// hues used only when diarization assigns speaker IDs. Not a brand
// concern; the palette is intentional variety, not drift. Allowlisted
// in scripts/gate/check-brand-token-drift.ts.
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

/** Map confidence (0-1) to a visual style (currently disabled) */
function getConfidenceStyle(_confidence: number): React.CSSProperties {
  return {};
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
  // useShallow is mandatory here: `useEditorStore()` with no selector
  // returns a new object every store update and re-renders the whole
  // word list (10k+ spans) on every keystroke. With useShallow the
  // subscription only triggers when one of the picked fields actually
  // changes by reference / value. Key perf fix per audit finding F3.
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
    refreshFromBackend,
    clearHighlights,
  } = useEditorStore(
    useShallow((s) => ({
      words: s.words,
      selectedIndex: s.selectedIndex,
      selectionRange: s.selectionRange,
      highlightedIndices: s.highlightedIndices,
      highlightType: s.highlightType,
      deleteWord: s.deleteWord,
      restoreWord: s.restoreWord,
      silenceWord: s.silenceWord,
      splitWord: s.splitWord,
      deleteRange: s.deleteRange,
      restoreAll: s.restoreAll,
      undo: s.undo,
      redo: s.redo,
      selectWord: s.selectWord,
      setSelectionRange: s.setSelectionRange,
      refreshFromBackend: s.refreshFromBackend,
      clearHighlights: s.clearHighlights,
    })),
  );

  const containerRef = useRef<HTMLDivElement>(null);
  const findInputRef = useRef<HTMLInputElement>(null);
  const dragStartRef = useRef<number | null>(null);
  const isDraggingRef = useRef(false);
  const [showFind, setShowFind] = useState(false);
  const [findQuery, setFindQuery] = useState("");
  const [findMatchIndex, setFindMatchIndex] = useState(0);
  const [contextMenu, setContextMenu] = useState<ContextMenuState>({
    visible: false,
    x: 0,
    y: 0,
    wordIndex: -1,
  });

  const [cleanupSummary, setCleanupSummary] = useState<string | null>(null);

  const highlightedSet = useMemo(() => new Set(highlightedIndices), [highlightedIndices]);

  const closeContextMenu = useCallback(() => {
    setContextMenu((prev) => ({ ...prev, visible: false }));
  }, []);

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
        // Simple click (no drag) — clear any highlights
        if (highlightedIndices.length > 0) {
          clearHighlights();
          setCleanupSummary(null);
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
          await refreshFromBackend();
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
  }, [selectedIndex, selectionRange, highlightedIndices, highlightType, deleteWord, deleteRange, closeContextMenu, clearHighlights, refreshFromBackend]);

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
      {/* Detection toolbar — cleanup summary only */}
      {cleanupSummary && (
        <div className="flex items-center gap-2 mb-3">
          <span className="text-[11px] text-mid-gray/60">
            {cleanupSummary}
          </span>
        </div>
      )}

      {/* Find bar */}
      {showFind && (
        <FindReplaceBar
          findQuery={findQuery}
          findMatchIndex={findMatchIndex}
          findMatchCount={findMatches.length}
          findInputRef={findInputRef}
          onQueryChange={setFindQuery}
          onMatchIndexReset={() => setFindMatchIndex(0)}
          onNavigate={navigateFind}
          onDeleteAll={handleDeleteAllMatches}
          onClose={() => {
            setShowFind(false);
            setFindQuery("");
          }}
        />
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
            <React.Fragment key={`word-${word.start_us}-${index}`}>
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
                    : isHighlighted && highlightType === "duplicate"
                      ? t("editor.duplicateWord")
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
                  isHighlighted && highlightType === "filler" && "bg-red-400/50 text-black",
                  isHighlighted && highlightType === "duplicate" && "bg-orange-400/50 text-black",
                  isHighlighted && highlightType === "pause" && "bg-yellow-400/50 text-black",
                  isCurrentFindMatch && !isHighlighted && "ring-2 ring-logo-primary bg-logo-primary/30",
                  isFindMatch && !isCurrentFindMatch && !isHighlighted && "bg-logo-primary/15",
                  isSelected && !isFindMatch && !isHighlighted && "bg-logo-primary text-black",
                  isRangeSelected && !isSelected && !isFindMatch && !isHighlighted && "bg-logo-primary/40",
                  !isSelected && !isRangeSelected && !isFindMatch && !isHighlighted && !word.deleted && !word.silenced && "hover:bg-mid-gray/20",
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
      <TranscriptContextMenu
        contextMenu={contextMenu}
        contextWord={contextWord}
        selectionRange={selectionRange}
        onDelete={handleDeleteSelected}
        onRestore={handleRestoreSelected}
        onSilence={handleSilenceSelected}
        onSplit={handleSplitSelected}
        onUndo={undo}
        onRedo={redo}
        onRestoreAll={restoreAll}
        onClose={closeContextMenu}
      />
    </div>
  );
};

export default TranscriptEditor;
