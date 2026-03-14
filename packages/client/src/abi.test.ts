import { describe, expect, test } from "bun:test";
import { MarchMadnessAbi } from "./abi";

describe("MarchMadnessAbi", () => {
  test("is a non-empty array", () => {
    expect(Array.isArray(MarchMadnessAbi)).toBe(true);
    expect(MarchMadnessAbi.length).toBeGreaterThan(0);
  });

  test("contains submitBracket function", () => {
    const fn = MarchMadnessAbi.find(
      (item) => item.type === "function" && item.name === "submitBracket",
    );
    expect(fn).toBeDefined();
    expect(fn!.type).toBe("function");
    expect(fn!.stateMutability).toBe("payable");
  });

  test("contains getBracket view function", () => {
    const fn = MarchMadnessAbi.find(
      (item) => item.type === "function" && item.name === "getBracket",
    );
    expect(fn).toBeDefined();
    expect(fn!.stateMutability).toBe("view");
    expect(fn!.outputs[0].type).toBe("bytes8");
  });

  test("contains all expected events", () => {
    const eventNames = MarchMadnessAbi.filter((item) => item.type === "event").map(
      (item) => item.name,
    );
    expect(eventNames).toContain("BracketSubmitted");
    expect(eventNames).toContain("TagSet");
    expect(eventNames).toContain("BracketScored");
    expect(eventNames).toContain("ResultsPosted");
    expect(eventNames).toContain("WinningsCollected");
  });

  test("uses sbytes8 for shielded bracket inputs", () => {
    const submitFn = MarchMadnessAbi.find(
      (item) => item.type === "function" && item.name === "submitBracket",
    );
    expect(submitFn!.inputs[0].type).toBe("sbytes8");

    const updateFn = MarchMadnessAbi.find(
      (item) => item.type === "function" && item.name === "updateBracket",
    );
    expect(updateFn!.inputs[0].type).toBe("sbytes8");
  });
});
