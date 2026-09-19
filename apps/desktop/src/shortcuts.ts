import { appState } from './state';

export function setupKeyboardShortcuts(focusSearchCallback: () => void) {
  window.addEventListener('keydown', (e: KeyboardEvent) => {
    // Skip if typing in an input or textarea
    const target = e.target as HTMLElement | null;
    const isInput =
      target &&
      (target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable);

    if (e.key === 'Escape') {
      window.dispatchEvent(new CustomEvent('sonora-close-settings'));
      if (appState.isMatchInspectorVisible()) {
        appState.closeMatchInspector();
        return;
      }
      if (appState.isArtworkFinderVisible()) {
        appState.closeArtworkFinder();
        return;
      }
      if (appState.isLyricsManagerVisible()) {
        appState.closeLyricsManager();
        return;
      }
      if (appState.isScanModalVisible()) {
        appState.toggleScanModal(false);
        return;
      }
      if (appState.isQueueVisible()) {
        appState.toggleQueue(false);
        return;
      }
      if (appState.isVisualizerVisible()) {
        appState.toggleVisualizer(false);
        return;
      }
      const lyricsContainer = document.querySelector('#lyrics-container') as HTMLElement;
      if (lyricsContainer && lyricsContainer.style.display !== 'none') {
        window.dispatchEvent(new CustomEvent('sonora-toggle-lyrics'));
        return;
      }
      if (isInput) {
        (target as HTMLInputElement).blur();
      }
      return;
    }

    if ((e.ctrlKey || e.metaKey) && e.key === ',') {
      e.preventDefault();
      window.dispatchEvent(new CustomEvent('sonora-open-settings'));
      return;
    }

    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'l') {
      e.preventDefault();
      appState.openLyricsManager();
      return;
    }

    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      focusSearchCallback();
      return;
    }

    if (isInput) return;

    switch (e.key) {
      case ' ':
        e.preventDefault();
        appState.togglePlayPause();
        break;

      case 'ArrowRight': {
        e.preventDefault();
        const status = appState.getStatus();
        const nextMs = Math.min(status.duration_ms, status.position_ms + 5000);
        appState.seek(nextMs);
        break;
      }

      case 'ArrowLeft': {
        e.preventDefault();
        const status = appState.getStatus();
        const prevMs = Math.max(0, status.position_ms - 5000);
        appState.seek(prevMs);
        break;
      }

      case 'ArrowUp': {
        e.preventDefault();
        const status = appState.getStatus();
        const newVol = Math.min(1.0, status.volume + 0.05);
        appState.setVolume(newVol);
        break;
      }

      case 'ArrowDown': {
        e.preventDefault();
        const status = appState.getStatus();
        const newVol = Math.max(0.0, status.volume - 0.05);
        appState.setVolume(newVol);
        break;
      }

      case 'n':
      case 'N':
        e.preventDefault();
        appState.next();
        break;

      case 'p':
      case 'P':
        e.preventDefault();
        appState.previous();
        break;

      case 'q':
      case 'Q':
        e.preventDefault();
        appState.toggleQueue();
        break;

      case 'v':
      case 'V':
        e.preventDefault();
        appState.toggleVisualizer();
        break;

      case 'l':
      case 'L':
        e.preventDefault();
        window.dispatchEvent(new CustomEvent('sonora-toggle-lyrics'));
        break;

      case '/':
        e.preventDefault();
        focusSearchCallback();
        break;
    }
  });
}
