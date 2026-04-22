export function clearActiveTextInteraction(): void {
  const selection = globalThis.getSelection?.();
  selection?.removeAllRanges();

  const activeElement = globalThis.document?.activeElement;
  if (
    activeElement instanceof globalThis.HTMLInputElement ||
    activeElement instanceof globalThis.HTMLTextAreaElement
  ) {
    const cursorPosition = activeElement.value.length;
    activeElement.setSelectionRange(cursorPosition, cursorPosition);
  }

  if (activeElement instanceof globalThis.HTMLElement) {
    activeElement.blur();
  }
}
