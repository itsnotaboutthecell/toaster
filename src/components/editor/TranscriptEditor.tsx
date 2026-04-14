import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useEditorStore } from "@/stores/editorStore";

interface ContextMenuState {
  visible: boolean;
  x: number;
  y: number;
  wordIndex: number;
}

const TranscriptEditor: React.FC = () => {
  const { t } = useTranslation();
  const {
    words,
    selectedIndex,
    selectionRange,
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
  } = useEditorStore();

  const containerRef = useRef<HTMLDivElement>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState>({
    visible: false,
    x: 0,
    y: 0,
    wordIndex: -1,
  });

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

  const handleWordClick = useCallback(
    (index: number, e: React.MouseEvent) => {
      if (e.shiftKey && selectedIndex !== null) {
        const start = Math.min(selectedIndex, index);
        const end = Math.max(selectedIndex, index);
        setSelectionRange([start, end]);
      } else {
        selectWord(index);
      }
      closeContextMenu();
    },
    [selectedIndex, selectWord, setSelectionRange, closeContextMenu],
  );

  const handleContextMenu = useCallback(
    (index: number, e: React.MouseEvent) => {
      e.preventDefault();
      selectWord(index);
      setContextMenu({ visible: true, x: e.clientX, y: e.clientY, wordIndex: index });
    },
    [selectWord],
  );

  const handleDeleteSelected = useCallback(async () => {
    if (selectionRange) {
      await deleteRange(selectionRange[0], selectionRange[1]);
    } else if (selectedIndex !== null) {
      await deleteWord(selectedIndex);
    }
    closeContextMenu();
  }, [selectedIndex, selectionRange, deleteWord, deleteRange, closeContextMenu]);

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

  return (
    <div
      ref={containerRef}
      className="relative p-4 outline-none select-none"
      tabIndex={0}
      onKeyDown={handleKeyDown}
    >
      {/* Word spans */}
      <div className="flex flex-wrap gap-1 leading-relaxed">
        {words.map((word, index) => {
          const isSelected = selectedIndex === index;
          const isRangeSelected = isInSelectionRange(index);

          return (
            <span
              key={`${index}-${word.start_us}`}
              role="button"
              tabIndex={-1}
              onClick={(e) => handleWordClick(index, e)}
              onContextMenu={(e) => handleContextMenu(index, e)}
              className={[
                "cursor-pointer rounded px-1 py-0.5 transition-colors",
                word.deleted && "line-through opacity-40",
                word.silenced && !word.deleted && "opacity-60 italic",
                isSelected && "bg-[#E8A838] text-[#1E1E1E]",
                isRangeSelected && !isSelected && "bg-[#E8A838]/40",
                !isSelected && !isRangeSelected && !word.deleted && !word.silenced && "hover:bg-[rgba(128,128,128,0.2)]",
              ]
                .filter(Boolean)
                .join(" ")}
            >
              {word.text}
            </span>
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
