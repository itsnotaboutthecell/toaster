import type { Result } from "@/bindings";

/**
 * Unwrap a specta `Result<T, string>` tagged union, throwing on the
 * error branch. Shared by EditorView + its extracted hooks.
 */
export const unwrapResult = <T,>(result: Result<T, string>): T => {
  if (result.status === "ok") {
    return result.data;
  }
  throw new Error(result.error);
};
