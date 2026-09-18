import { appState } from '../state';
import { escapeHtml } from '../escape';

function formatTime(ms: number): string {
  if (!ms || isNaN(ms)) return '00:00';
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
}

export class QueueDrawerComponent {
  private container: HTMLElement;

  constructor(container: HTMLElement) {
    this.container = container;
    this.render();
    appState.subscribe(() => this.update());
  }

  private render() {
    this.container.innerHTML = `
      <div class="queue-drawer-inner">
        <div class="queue-header">
          <div class="queue-title-group">
            <span class="queue-heading">Play Queue</span>
            <span class="queue-count-tag">0 tracks</span>
          </div>
          <div class="queue-header-actions">
            <button class="text-button clear-queue-btn" title="Clear all queue items">Clear</button>
            <button class="icon-button close-queue-btn" title="Close queue (q)" aria-label="Close queue">✕</button>
          </div>
        </div>
        <div class="queue-list-container">
          <div class="queue-empty-state">
            <div class="queue-empty-icon">📭</div>
            <div class="queue-empty-text">Queue is empty</div>
            <div class="queue-empty-sub">Add tracks or albums from your library</div>
          </div>
          <div class="queue-items"></div>
        </div>
      </div>
    `;

    const closeBtn = this.container.querySelector('.close-queue-btn') as HTMLButtonElement;
    closeBtn.addEventListener('click', () => appState.toggleQueue(false));

    const clearBtn = this.container.querySelector('.clear-queue-btn') as HTMLButtonElement;
    clearBtn.addEventListener('click', () => appState.clearQueue());
  }

  private update() {
    const queue = appState.getQueue();
    const status = appState.getStatus();
    const isVisible = appState.isQueueVisible();

    if (isVisible) {
      this.container.classList.add('open');
    } else {
      this.container.classList.remove('open');
    }

    const countTag = this.container.querySelector('.queue-count-tag') as HTMLElement;
    if (countTag) {
      countTag.textContent = `${queue.length} ${queue.length === 1 ? 'track' : 'tracks'}`;
    }

    const emptyState = this.container.querySelector('.queue-empty-state') as HTMLElement;
    const itemsContainer = this.container.querySelector('.queue-items') as HTMLElement;

    if (!itemsContainer) return;

    if (queue.length === 0) {
      emptyState.style.display = 'flex';
      itemsContainer.innerHTML = '';
      return;
    }

    emptyState.style.display = 'none';

    // Render queue items
    itemsContainer.innerHTML = queue
      .map((item, idx) => {
        const isCurrent = status.current_queue_index === idx;
        return `
          <div class="queue-item ${isCurrent ? 'current-playing' : ''}" data-index="${idx}">
            <div class="queue-item-drag-order">
              <span class="queue-index-num">${idx + 1}</span>
              ${
                isCurrent
                  ? `<span class="now-playing-bars">
                       <span class="bar bar-1"></span>
                       <span class="bar bar-2"></span>
                       <span class="bar bar-3"></span>
                     </span>`
                  : ''
              }
            </div>
            <div class="queue-item-meta" title="${escapeHtml(item.title)}">
              <div class="queue-item-title">${escapeHtml(item.title)}</div>
              <div class="queue-item-sub">${escapeHtml(item.artist || 'Unknown Artist')}</div>
            </div>
            <div class="queue-item-duration">${formatTime(item.duration_ms)}</div>
            <div class="queue-item-actions">
              <button class="queue-action-btn move-up" title="Move Up" aria-label="Move ${escapeHtml(item.title)} up" ${idx === 0 ? 'disabled' : ''}>▲</button>
              <button class="queue-action-btn move-down" title="Move Down" aria-label="Move ${escapeHtml(item.title)} down" ${idx === queue.length - 1 ? 'disabled' : ''}>▼</button>
              <button class="queue-action-btn remove-track" title="Remove" aria-label="Remove ${escapeHtml(item.title)} from queue">✕</button>
            </div>
          </div>
        `;
      })
      .join('');

    // Attach row events
    const rows = itemsContainer.querySelectorAll('.queue-item');
    rows.forEach((row) => {
      const idx = parseInt(row.getAttribute('data-index') || '0', 10);
      const meta = row.querySelector('.queue-item-meta');
      meta?.addEventListener('click', () => {
        appState.playQueueIndex(idx);
      });

      const moveUpBtn = row.querySelector('.move-up') as HTMLButtonElement;
      moveUpBtn?.addEventListener('click', (e) => {
        e.stopPropagation();
        if (idx > 0) appState.moveQueueItem(idx, idx - 1);
      });

      const moveDownBtn = row.querySelector('.move-down') as HTMLButtonElement;
      moveDownBtn?.addEventListener('click', (e) => {
        e.stopPropagation();
        if (idx < queue.length - 1) appState.moveQueueItem(idx, idx + 1);
      });

      const removeBtn = row.querySelector('.remove-track') as HTMLButtonElement;
      removeBtn?.addEventListener('click', (e) => {
        e.stopPropagation();
        appState.removeFromQueue(idx);
      });
    });
  }
}
