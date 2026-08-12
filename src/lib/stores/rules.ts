import { writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Rule, RuleWithStatus, StatusEvent } from "../types";
import { toCleanRule } from "../types";

export const rules = writable<RuleWithStatus[]>([]);
export const loading = writable(true);

export async function fetchRules() {
  loading.set(true);
  const result = await invoke<RuleWithStatus[]>("get_rules");
  rules.set(result);
  loading.set(false);
}

export async function createRule(rule: Rule): Promise<Rule> {
  const created = await invoke<Rule>("create_rule", { rule: toCleanRule(rule) });
  await fetchRules();
  return created;
}

export async function updateRule(rule: Rule): Promise<Rule> {
  const updated = await invoke<Rule>("update_rule", { rule: toCleanRule(rule) });
  await fetchRules();
  return updated;
}

export async function deleteRule(id: string): Promise<void> {
  await invoke("delete_rule", { id });
  await fetchRules();
}

export async function toggleRule(id: string): Promise<boolean> {
  const connected = await invoke<boolean>("toggle_rule", { id });
  rules.update((current) =>
    current.map((r) =>
      r.id === id
        ? { ...r, tunnelStatus: connected ? { status: "connected" as const } : { status: "disconnected" as const } }
        : r,
    ),
  );
  return connected;
}

export function listenForStatusChanges() {
  listen<StatusEvent>("tunnel-status-changed", (event) => {
    const { ruleId, status } = event.payload;
    rules.update((current) =>
      current.map((r) =>
        r.id === ruleId ? { ...r, tunnelStatus: status } : r,
      ),
    );
  });
}
