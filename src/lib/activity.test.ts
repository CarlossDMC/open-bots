import { describe, expect, it } from "vitest";
import { mergeRuntimeEvent, toActivityEvent } from "./activity";
import { demoAgents } from "./demo-data";
import type { RuntimeEvent } from "@/types/domain";

function runtimeEvent(overrides: Partial<RuntimeEvent>): RuntimeEvent {
  return {
    id: "event-1",
    eventType: "agent.created",
    aggregateId: null,
    payload: {},
    occurredAt: "2026-01-10T10:00:00.000Z",
    ...overrides
  };
}

describe("toActivityEvent", () => {
  it("names the agent referenced by the payload", () => {
    const activity = toActivityEvent(
      runtimeEvent({ eventType: "memory.added", payload: { agentId: "demo-nova" } }),
      demoAgents
    );
    expect(activity.subject).toBe("Nova");
    expect(activity.detail).toBe("Memory added");
  });

  it("falls back to the aggregate agent and then the payload name", () => {
    expect(toActivityEvent(runtimeEvent({ aggregateId: "demo-atlas" }), demoAgents).subject).toBe(
      "Atlas"
    );
    expect(
      toActivityEvent(runtimeEvent({ aggregateId: "unknown", payload: { name: "Iris" } }), [])
        .subject
    ).toBe("Iris");
    expect(toActivityEvent(runtimeEvent({}), []).subject).toBe("Runtime");
  });

  it("describes routine events by routine name without mistaking it for the agent", () => {
    const activity = toActivityEvent(
      runtimeEvent({
        eventType: "routine.triggered",
        payload: { agentId: "missing", name: "Morning digest" }
      }),
      []
    );
    expect(activity.subject).toBe("Runtime");
    expect(activity.detail).toBe("Routine triggered: Morning digest");
  });

  it("describes conversation turns", () => {
    const detail = (eventType: string, payload: Record<string, unknown> = {}) =>
      toActivityEvent(runtimeEvent({ eventType, payload }), []).detail;
    expect(detail("message.created", { role: "user" })).toBe("Received a message");
    expect(detail("message.created", { role: "agent" })).toBe("Replied");
    expect(detail("agent.failed", { detail: "usage limit" })).toBe("Failed: usage limit");
    expect(detail("agent.failed")).toBe("Failed");
    expect(detail("agent.cancelled")).toBe("Stopped");
  });

  it("describes task events by title", () => {
    const detail = (eventType: string, payload: Record<string, unknown> = {}) =>
      toActivityEvent(runtimeEvent({ eventType, payload }), []).detail;
    expect(detail("task.assigned", { title: "Review the API" })).toBe(
      "Task assigned: Review the API"
    );
    expect(detail("task.completed")).toBe("Task completed");
  });

  it("describes model changes", () => {
    const detail = (payload: Record<string, unknown>) =>
      toActivityEvent(runtimeEvent({ eventType: "agent.updated", payload }), []).detail;
    expect(detail({ model: "gpt-5.5", reasoningEffort: "high" })).toBe(
      "Model set to gpt-5.5 (high)"
    );
    expect(detail({ model: "gpt-5.5", reasoningEffort: null })).toBe("Model set to gpt-5.5");
    expect(detail({ model: null })).toBe("Model reset to provider default");
  });

  it("includes the action in approval details", () => {
    const activity = toActivityEvent(
      runtimeEvent({ eventType: "approval.denied", payload: { action: "git push" } }),
      []
    );
    expect(activity.detail).toBe("Approval denied: git push");
  });

  it("uses a payload detail or the event type for other events", () => {
    expect(
      toActivityEvent(runtimeEvent({ eventType: "tool.started", payload: { detail: "Ran" } }), [])
        .detail
    ).toBe("Ran");
    expect(toActivityEvent(runtimeEvent({ eventType: "process.failed" }), []).detail).toBe(
      "process.failed"
    );
  });
});

describe("mergeRuntimeEvent", () => {
  it("prepends new events within the limit", () => {
    const existing = [runtimeEvent({ id: "a" }), runtimeEvent({ id: "b" })];
    const merged = mergeRuntimeEvent(existing, runtimeEvent({ id: "c" }), 2);
    expect(merged.map((event) => event.id)).toEqual(["c", "a"]);
  });

  it("ignores events that are already present", () => {
    const existing = [runtimeEvent({ id: "a" })];
    expect(mergeRuntimeEvent(existing, runtimeEvent({ id: "a" }), 10)).toBe(existing);
  });
});
