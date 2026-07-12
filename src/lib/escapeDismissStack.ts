type EscapeEvent = Pick<KeyboardEvent, "key" | "repeat" | "preventDefault" | "stopImmediatePropagation">;

export class EscapeDismissStack {
  private handlers: (() => void)[] = [];
  private keydownHandled = false;

  get size() {
    return this.handlers.length;
  }

  register(handler: () => void) {
    this.handlers.push(handler);
    return () => {
      const index = this.handlers.indexOf(handler);
      if (index >= 0) this.handlers.splice(index, 1);
      if (this.handlers.length === 0) this.keydownHandled = false;
    };
  }

  handleKeyDown(event: EscapeEvent) {
    if (!this.prepare(event)) return;
    if (event.repeat || this.keydownHandled) return;
    this.keydownHandled = true;
    this.dismissTopmost();
  }

  handleKeyUp(event: EscapeEvent) {
    if (!this.prepare(event)) return;
    if (this.keydownHandled) {
      this.keydownHandled = false;
    } else {
      this.dismissTopmost();
    }
  }

  private prepare(event: EscapeEvent) {
    if (event.key !== "Escape" || this.handlers.length === 0) return false;
    event.preventDefault();
    event.stopImmediatePropagation();
    return true;
  }

  private dismissTopmost() {
    this.handlers[this.handlers.length - 1]?.();
  }
}
