import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";

interface DiscardWordsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const DiscardWords: React.FC<DiscardWordsProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const [newWord, setNewWord] = useState("");
    const discardWords: string[] = getSetting("custom_filler_words") ?? [];
    const allowWords: string[] = getSetting("custom_words") || [];

    const handleAddWord = () => {
      const trimmedWord = newWord.trim();
      const sanitizedWord = trimmedWord.replace(/[<>"'&]/g, "");
      if (sanitizedWord && sanitizedWord.length <= 50) {
        if (discardWords.includes(sanitizedWord)) {
          toast.error(
            t("settings.controls.discardWords.duplicate", {
              word: sanitizedWord,
            }),
          );
          return;
        }
        if (allowWords.includes(sanitizedWord)) {
          toast.error(
            t("settings.controls.discardWords.conflictWithAllow", {
              word: sanitizedWord,
            }),
          );
          return;
        }
        updateSetting("custom_filler_words", [...discardWords, sanitizedWord]);
        setNewWord("");
      }
    };

    const handleRemoveWord = (wordToRemove: string) => {
      updateSetting(
        "custom_filler_words",
        discardWords.filter((word) => word !== wordToRemove),
      );
    };

    const handleKeyPress = (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAddWord();
      }
    };

    return (
      <>
        <SettingContainer
          title={t("settings.controls.discardWords.title")}
          description={t("settings.controls.discardWords.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        >
          <div className="flex items-center gap-2">
            <Input
              type="text"
              className="max-w-40"
              value={newWord}
              onChange={(e) => setNewWord(e.target.value)}
              onKeyDown={handleKeyPress}
              placeholder={t("settings.controls.discardWords.placeholder")}
              variant="compact"
              disabled={isUpdating("custom_filler_words")}
            />
            <Button
              onClick={handleAddWord}
              disabled={
                !newWord.trim() ||
                newWord.trim().length > 50 ||
                isUpdating("custom_filler_words")
              }
              variant="primary"
              size="md"
            >
              {t("settings.controls.discardWords.add")}
            </Button>
          </div>
        </SettingContainer>
        {discardWords.length > 0 && (
          <div
            className={`px-4 p-2 ${grouped ? "" : "rounded-lg border border-mid-gray/20"} flex flex-wrap gap-1`}
          >
            {discardWords.map((word) => (
              <Button
                key={word}
                onClick={() => handleRemoveWord(word)}
                disabled={isUpdating("custom_filler_words")}
                variant="secondary"
                size="sm"
                className="inline-flex items-center gap-1 cursor-pointer"
                aria-label={t("settings.controls.discardWords.remove", { word })}
              >
                <span>{word}</span>
                <svg
                  className="w-3 h-3"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </Button>
            ))}
          </div>
        )}
      </>
    );
  },
);
