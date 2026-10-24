import { describe, expect, it } from "vitest";
import { isTopOverlay, popOverlay, pushOverlay } from "@/lib/overlay-stack";

describe("overlay stack", () => {
  it("treats a single overlay as the top one", () => {
    const drawer = pushOverlay();
    expect(isTopOverlay(drawer)).toBe(true);
    popOverlay(drawer);
  });

  it("gives the innermost overlay the key events", () => {
    const drawer = pushOverlay();
    const dialog = pushOverlay();
    expect(isTopOverlay(dialog)).toBe(true);
    expect(isTopOverlay(drawer)).toBe(false);

    // Closing the dialog hands Escape back to the drawer underneath.
    popOverlay(dialog);
    expect(isTopOverlay(drawer)).toBe(true);
    popOverlay(drawer);
  });

  it("survives overlays closing out of order", () => {
    const drawer = pushOverlay();
    const menu = pushOverlay();
    popOverlay(drawer);
    expect(isTopOverlay(menu)).toBe(true);
    popOverlay(menu);
  });

  it("reports nothing as top once every overlay is gone", () => {
    const only = pushOverlay();
    popOverlay(only);
    expect(isTopOverlay(only)).toBe(false);
    // Popping twice must not corrupt the stack.
    popOverlay(only);
    const next = pushOverlay();
    expect(isTopOverlay(next)).toBe(true);
    popOverlay(next);
  });
});
