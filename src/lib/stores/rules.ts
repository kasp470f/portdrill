import { get, writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { HostKeyInfo, Rule, RuleWithStatus, StatusEvent } from "../types";
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

/** Creates several rules in order (the paste-many flow), then refreshes once. */
export async function createRules(list: Rule[]): Promise<Rule[]> {
  const created: Rule[] = [];
  for (const rule of list) {
    created.push(await invoke<Rule>("create_rule", { rule: toCleanRule(rule) }));
  }
  await fetchRules();
  return created;
}

export async function updateRule(rule: Rule): Promise<Rule> {
  const updated = await invoke<Rule>("update_rule", { rule: toCleanRule(rule) });
  await fetchRules();
  return updated;
}

function statusOf(id: string) {
  return get(rules).find((r) => r.id === id)?.tunnelStatus.status;
}

/**
 * Saves an edit even while the tunnel runs: the backend refuses to edit an
 * active tunnel, so stop it, save, and start it again with the new settings.
 */
export async function updateRuleRestarting(rule: Rule): Promise<Rule> {
  // A rule that is reconnecting counts as running: stop it, save, start it.
  const before = statusOf(rule.id);
  const wasConnected = before === "connected" || before === "reconnecting";
  if (wasConnected) {
    await toggleRule(rule.id);
  }
  const updated = await updateRule(rule);
  if (wasConnected) {
    await toggleRule(rule.id);
  }
  return updated;
}

export async function reorderRules(ids: string[]): Promise<void> {
  await invoke("reorder_rules", { ids });
  await fetchRules();
}

export async function duplicateRule(rule: Rule): Promise<Rule> {
  const clone = toCleanRule(rule);
  clone.name = `${clone.name} (copy)`;
  const created = await invoke<Rule>("create_rule", { rule: clone });
  await fetchRules();
  return created;
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

function patchRule(id: string, patch: Partial<RuleWithStatus>) {
  rules.update((current) => current.map((r) => (r.id === id ? { ...r, ...patch } : r)));
}

/** Display-only: allowed while connected. Updates the list immediately. */
export async function setForwardsHidden(id: string, hidden: boolean): Promise<void> {
  patchRule(id, { forwardsHidden: hidden });
  try {
    await invoke("set_forwards_hidden", { id, hidden });
  } catch (e) {
    patchRule(id, { forwardsHidden: !hidden });
    throw e;
  }
}

/**
 * Pauses or resumes auto-reconnect until PortDrill restarts, without editing
 * the rule. Pausing mid-reconnect stops it; the backend reports the new status.
 */
export async function setAutoReconnectPaused(id: string, paused: boolean): Promise<void> {
  patchRule(id, { autoReconnectPaused: paused });
  try {
    await invoke("set_auto_reconnect_paused", { id, paused });
  } catch (e) {
    patchRule(id, { autoReconnectPaused: !paused });
    throw e;
  }
}

/** Display-only: allowed while connected. */
export async function setRuleGroup(id: string, group: string): Promise<void> {
  await invoke("set_rule_group", { id, group });
  await fetchRules();
}

export async function sshAvailable(): Promise<boolean> {
  return await invoke<boolean>("ssh_available");
}

/** Neither host-key call touches rule data, so neither needs a fetchRules() resync. */
export async function inspectHostKey(id: string): Promise<HostKeyInfo> {
  return await invoke<HostKeyInfo>("inspect_host_key", { id });
}

export async function clearHostKey(id: string): Promise<string> {
  return await invoke<string>("clear_host_key", { id });
}

/** Resolves once the listener is registered, so nothing emitted after is missed. */
export async function listenForStatusChanges() {
  await listen<StatusEvent>("tunnel-status-changed", (event) => {
    const { ruleId, status } = event.payload;
    rules.update((current) =>
      current.map((r) =>
        r.id === ruleId ? { ...r, tunnelStatus: status } : r,
      ),
    );
  });
}
