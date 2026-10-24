/** Dismissible overlays currently on screen, innermost last. */
const stack: symbol[] = [];

/** Registers an overlay and returns its handle. Call from the effect that opens it. */
export function pushOverlay(): symbol {
  const handle = Symbol("overlay");
  stack.push(handle);
  return handle;
}

export function popOverlay(handle: symbol): void {
  const index = stack.indexOf(handle);
  if (index !== -1) stack.splice(index, 1);
}

/** True when nothing is stacked above this overlay.
 *
 *  Escape has to reach exactly one layer: a rule dialog opened from the packet inspector
 *  must close the dialog and leave the drawer alone. Window-level key handlers cannot see
 *  each other, so each one asks whether it is on top instead. */
export function isTopOverlay(handle: symbol): boolean {
  return stack[stack.length - 1] === handle;
}
