import { get, writable } from "svelte/store";
import type { Rule } from "../types";
import { deleteRule } from "./rules";

export type Dialog =
  | { kind: "rule"; rule: Rule | null; tab: "fill" | "paste" }
  | { kind: "hostKey"; ruleId: string; reconnect: boolean };

export const dialog = writable<Dialog | null>(null);

export function openNewRule(tab: "fill" | "paste" = "fill") {
  dialog.set({ kind: "rule", rule: null, tab });
}

export function openEditRule(rule: Rule) {
  dialog.set({ kind: "rule", rule, tab: "fill" });
}

export function openHostKey(ruleId: string, reconnect: boolean) {
  dialog.set({ kind: "hostKey", ruleId, reconnect });
}

export function closeDialog() {
  dialog.set(null);
}

/** Set when ssh can't be started at all; the app shows a setup banner. */
export const sshMissing = writable(false);

export type Toast =
  | { kind: "undo"; ruleId: string; name: string; expiresAt: number }
  | { kind: "message"; text: string; tone: "info" | "error" };

export const toast = writable<Toast | null>(null);

/** Rules deleted but still inside the undo window: hidden from the list. */
export const pendingDeletes = writable<Set<string>>(new Set());

export const UNDO_SECONDS = 8;

let toastTimer: ReturnType<typeof setTimeout> | undefined;
let pendingDelete: { ruleId: string; timer: ReturnType<typeof setTimeout> } | undefined;

function setPending(update: (s: Set<string>) => void) {
  pendingDeletes.update((current) => {
    const next = new Set(current);
    update(next);
    return next;
  });
}

export function showMessage(text: string, tone: "info" | "error" = "info") {
  clearTimeout(toastTimer);
  toast.set({ kind: "message", text, tone });
  toastTimer = setTimeout(() => toast.set(null), 5000);
}

async function commitDelete(ruleId: string) {
  pendingDelete = undefined;
  if (get(toast)?.kind === "undo") toast.set(null);
  try {
    await deleteRule(ruleId);
  } catch (e) {
    showMessage(String(e), "error");
  } finally {
    setPending((s) => s.delete(ruleId));
  }
}

/**
 * Hides the rule straight away and deletes it for real once the undo window
 * passes. Only one delete waits at a time: a second one commits the first.
 */
export function requestDelete(rule: Rule) {
  if (pendingDelete) {
    clearTimeout(pendingDelete.timer);
    void commitDelete(pendingDelete.ruleId);
  }
  clearTimeout(toastTimer);
  setPending((s) => s.add(rule.id));
  const timer = setTimeout(() => void commitDelete(rule.id), UNDO_SECONDS * 1000);
  pendingDelete = { ruleId: rule.id, timer };
  toast.set({ kind: "undo", ruleId: rule.id, name: rule.name, expiresAt: Date.now() + UNDO_SECONDS * 1000 });
}

export function undoDelete() {
  if (!pendingDelete) return;
  clearTimeout(pendingDelete.timer);
  const { ruleId } = pendingDelete;
  pendingDelete = undefined;
  setPending((s) => s.delete(ruleId));
  toast.set(null);
}
