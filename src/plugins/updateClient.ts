import { pluginCommand } from "./client";
export interface UpdatePolicy { checkOnStartup: boolean; autoInstall: boolean }
export interface Subscription { id: string; location: string; name: string; description: string }
export interface UpdateNotice { id: string; name: string; version: string; installedVersion: string; status: "available" | "deferred" | "installed" | "current" | "failed"; error?: { message: string } }
export interface UpdateState {
  policy: UpdatePolicy; subscriptions: Subscription[]; items: UpdateNotice[]; checking: boolean; changed: boolean;
  checks: { id: string; status: string; error?: { message: string } }[];
}
export const emptyUpdateState: UpdateState = { policy: { checkOnStartup: false, autoInstall: false }, subscriptions: [], items: [], checks: [], checking: false, changed: false };
export const pendingUpdates = (state: UpdateState) => state.items.filter(item => ["available", "deferred"].includes(item.status));
export async function tickPluginUpdates() { return pluginCommand<UpdateState>("updates.tick", { requestId: crypto.randomUUID() }); }
