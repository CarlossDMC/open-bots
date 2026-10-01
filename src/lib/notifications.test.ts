import { describe, expect, it } from "vitest";
import { demoAgents } from "./demo-data";
import {
  notificationFor,
  readNotificationPreference,
  shouldNotify,
  writeNotificationPreference
} from "./notifications";
import type { RuntimeEvent } from "@/types/domain";

function runtimeEvent(eventType: string, payload: Record<string, unknown>): RuntimeEvent {
  return { id: "event", eventType, payload, occurredAt: "2026-01-10T10:00:00.000Z" };
}

describe("notificationFor", () => {
  it("asks for attention when an approval is requested", () => {
    expect(
      notificationFor(
        runtimeEvent("approval.requested", { agentId: "demo-atlas", action: "git push" }),
        demoAgents
      )
    ).toEqual({ title: "Approval needed", body: "Atlas wants to run: git push" });
  });

  it("announces triggered routines by agent and routine name", () => {
    expect(
      notificationFor(
        runtimeEvent("routine.triggered", { agentId: "demo-nova", name: "Morning digest" }),
        demoAgents
      )
    ).toEqual({ title: "Nova · routine", body: "Morning digest" });
    expect(notificationFor(runtimeEvent("routine.triggered", {}), [])).toEqual({
      title: "An agent · routine",
      body: "A routine was triggered"
    });
  });

  it("reports finished and failed turns", () => {
    expect(
      notificationFor(
        { ...runtimeEvent("agent.completed", {}), aggregateId: "demo-atlas" },
        demoAgents
      )
    ).toEqual({ title: "Atlas", body: "Finished a turn" });
    expect(
      notificationFor(runtimeEvent("agent.wake_skipped", { agentId: "demo-atlas" }), demoAgents)
    ).toEqual({
      title: "Atlas",
      body: "Stopped after the chained turn limit. Send a message to continue."
    });
    expect(
      notificationFor(runtimeEvent("agent.failed", { detail: "usage limit" }), demoAgents)
    ).toEqual({ title: "An agent", body: "usage limit" });
  });

  it("notifies once per group round instead of once per member", () => {
    expect(
      notificationFor(
        runtimeEvent("agent.completed", { agentId: "demo-atlas", groupId: "group-1" }),
        demoAgents
      )
    ).toBeNull();
    expect(
      notificationFor(
        runtimeEvent("group.round_completed", { groupId: "group-1", name: "Release" }),
        demoAgents
      )
    ).toEqual({ title: "Release", body: "Everyone has answered" });
  });

  it("stays quiet for informational events", () => {
    for (const type of [
      "agent.created",
      "agent.started",
      "message.created",
      "memory.added",
      "routine.created"
    ]) {
      expect(notificationFor(runtimeEvent(type, { agentId: "demo-atlas" }), demoAgents)).toBeNull();
    }
  });
});

describe("shouldNotify", () => {
  it("notifies only when enabled and the window is in the background", () => {
    expect(shouldNotify(true, false)).toBe(true);
    expect(shouldNotify(true, true)).toBe(false);
    expect(shouldNotify(false, false)).toBe(false);
  });
});

describe("notification preference", () => {
  it("defaults to enabled and round-trips through storage", () => {
    const values = new Map<string, string>();
    const storage = {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => void values.set(key, value)
    };
    expect(readNotificationPreference(storage)).toBe(true);
    writeNotificationPreference(storage, false);
    expect(readNotificationPreference(storage)).toBe(false);
    expect(readNotificationPreference(undefined)).toBe(true);
  });
});
