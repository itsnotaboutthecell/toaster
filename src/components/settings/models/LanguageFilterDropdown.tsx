import React, { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, Globe } from "lucide-react";
import { LANGUAGES } from "@/lib/constants/languages.ts";

interface LanguageFilterDropdownProps {
  /** Selected language code, or "all" for no filter. */
  value: string;
  onChange: (next: string) => void;
}

/**
 * Shared language filter dropdown used by ModelsSettings in both the
 * default and `lockedCategory` layouts. Owns its own open/search/focus
 * and outside-click behavior; parents pass value/onChange only.
 */
export const LanguageFilterDropdown: React.FC<LanguageFilterDropdownProps> = ({
  value,
  onChange,
}) => {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const rootRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        rootRef.current &&
        !rootRef.current.contains(event.target as Node)
      ) {
        setOpen(false);
        setSearch("");
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  useEffect(() => {
    if (open && searchInputRef.current) {
      searchInputRef.current.focus();
    }
  }, [open]);

  const filteredLanguages = useMemo(() => {
    return LANGUAGES.filter(
      (lang) =>
        lang.value !== "auto" &&
        lang.label.toLowerCase().includes(search.toLowerCase()),
    );
  }, [search]);

  const selectedLabel = useMemo(() => {
    if (value === "all") {
      return t("settings.models.filters.allLanguages");
    }
    return LANGUAGES.find((lang) => lang.value === value)?.label || "";
  }, [value, t]);

  const pick = (next: string) => {
    onChange(next);
    setOpen(false);
    setSearch("");
  };

  return (
    <div className="relative" ref={rootRef}>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-lg transition-colors ${
          value !== "all"
            ? "bg-logo-primary/20 text-logo-primary"
            : "bg-mid-gray/10 text-text/60 hover:bg-mid-gray/20"
        }`}
      >
        <Globe className="w-3.5 h-3.5" />
        <span className="max-w-[120px] truncate">{selectedLabel}</span>
        <ChevronDown
          className={`w-3.5 h-3.5 transition-transform ${
            open ? "rotate-180" : ""
          }`}
        />
      </button>

      {open && (
        <div className="absolute top-full right-0 mt-1 w-56 bg-background border border-mid-gray/80 rounded-lg shadow-lg z-50 overflow-hidden">
          <div className="p-2 border-b border-mid-gray/40">
            <input
              ref={searchInputRef}
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && filteredLanguages.length > 0) {
                  pick(filteredLanguages[0].value);
                } else if (e.key === "Escape") {
                  setOpen(false);
                  setSearch("");
                }
              }}
              placeholder={t(
                "settings.general.language.searchPlaceholder",
              )}
              className="w-full px-2 py-1 text-sm bg-mid-gray/10 border border-mid-gray/40 rounded-md focus:outline-none focus:ring-1 focus:ring-logo-primary"
            />
          </div>
          <div className="max-h-48 overflow-y-auto">
            <button
              type="button"
              onClick={() => pick("all")}
              className={`w-full px-3 py-1.5 text-sm text-left transition-colors ${
                value === "all"
                  ? "bg-logo-primary/20 text-logo-primary font-semibold"
                  : "hover:bg-mid-gray/10"
              }`}
            >
              {t("settings.models.filters.allLanguages")}
            </button>
            {filteredLanguages.map((lang) => (
              <button
                key={lang.value}
                type="button"
                onClick={() => pick(lang.value)}
                className={`w-full px-3 py-1.5 text-sm text-left transition-colors ${
                  value === lang.value
                    ? "bg-logo-primary/20 text-logo-primary font-semibold"
                    : "hover:bg-mid-gray/10"
                }`}
              >
                {lang.label}
              </button>
            ))}
            {filteredLanguages.length === 0 && (
              <div className="px-3 py-2 text-sm text-text/50 text-center">
                {t("settings.general.language.noResults")}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
