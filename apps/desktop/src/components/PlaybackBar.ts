import { appState } from '../state';
import { layoutManager } from '../customization';

function formatTime(ms: number): string {
  if (!ms || isNaN(ms) || ms < 0) return '00:00';
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
}

export class PlaybackBarComponent {
  private container: HTMLElement;
  private isScrubbing: boolean = false;
  private scrubValue: number = 0;

  constructor(container: HTMLElement) {
    this.container = container;
    this.render();
    appState.subscribe(() => this.update());
    layoutManager.subscribe(() => this.update());
  }

  private render() {
    this.container.innerHTML = `
      <div class="playback-bar-inner">
        <!-- Left: Track info & artwork -->
        <div class="playback-left">
          <div class="album-thumb-container">
            <div class="album-thumb placeholder-thumb">
              <span class="vinyl-groove"></span>
              <span class="thumb-note">♪</span>
            </div>
            <img class="album-thumb-img" src="" alt="Album Artwork" style="display: none;" />
          </div>
          <div class="track-info">
            <div class="track-title" title="No track playing">No track selected</div>
            <div class="track-artist">Select a track to play</div>
          </div>
        </div>

        <!-- Center: Playback transport & seekbar -->
        <div class="playback-center">
          <div class="transport-controls">
            <button class="transport-btn prev-btn" title="Previous Track (p)" aria-label="Previous Track">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                <path d="M6 6h2v12H6zm3.5 6l8.5 6V6z"/>
              </svg>
            </button>
            <button class="transport-btn play-btn primary-play" title="Play / Pause (Space)" aria-label="Play or Pause">
              <svg class="play-icon" width="22" height="22" viewBox="0 0 24 24" fill="currentColor">
                <path d="M8 5v14l11-7z"/>
              </svg>
              <svg class="pause-icon" width="22" height="22" viewBox="0 0 24 24" fill="currentColor" style="display: none;">
                <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>
              </svg>
            </button>
            <button class="transport-btn next-btn" title="Next Track (n)" aria-label="Next Track">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                <path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z"/>
              </svg>
            </button>
          </div>

          <div class="progress-bar-container">
            <span class="time-label current-time">00:00</span>
            <div class="seek-track">
              <div class="seek-fill"></div>
              <input type="range" class="seek-slider" min="0" max="1000" value="0" aria-label="Seek track position" />
            </div>
            <span class="time-label total-time">00:00</span>
          </div>
        </div>

        <!-- Right: Audio toggles, visualizer, lyrics, volume & queue -->
        <div class="playback-right">
          <button class="icon-button vis-toggle-btn" title="Toggle Spectrum Visualizer (v)" aria-label="Toggle Visualizer">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M4 9h2v6H4zm4-4h2v14H8zm4 7h2v7h-2zm4-6h2v13h-2zm4 3h2v10h-2z"/>
            </svg>
          </button>

          <button class="icon-button lyrics-toggle-btn" title="Toggle Synchronized Lyrics (l)" aria-label="Toggle Lyrics">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z"/>
            </svg>
          </button>

          <div class="volume-container">
            <button class="icon-button volume-mute-btn" title="Toggle Mute" aria-label="Mute or Unmute">
              <svg class="volume-icon" width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                <path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02zM14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 6.71v2.06c4.01-.91 7-4.49 7-8.77s-2.99-7.86-7-8.77z"/>
              </svg>
            </button>
            <div class="volume-slider-wrapper">
              <div class="volume-fill"></div>
              <input type="range" class="volume-slider" min="0" max="100" value="100" aria-label="Volume slider" />
            </div>
          </div>

          <button class="icon-button queue-toggle-btn" title="Toggle Queue Drawer (q)" aria-label="Toggle Queue">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M15 6H3v2h12V6zm0 4H3v2h12v-2zM3 16h8v-2H3v2zM17 6v8.18c-.31-.11-.65-.18-1-.18-1.66 0-3 1.34-3 3s1.34 3 3 3 3-1.34 3-3V8h3V6h-5z"/>
            </svg>
            <span class="queue-badge" style="display: none;">0</span>
          </button>
        </div>
      </div>
    `;

    this.attachEventListeners();
  }

  private attachEventListeners() {
    const playBtn = this.container.querySelector('.primary-play') as HTMLButtonElement;
    const prevBtn = this.container.querySelector('.prev-btn') as HTMLButtonElement;
    const nextBtn = this.container.querySelector('.next-btn') as HTMLButtonElement;
    const seekSlider = this.container.querySelector('.seek-slider') as HTMLInputElement;
    const volumeSlider = this.container.querySelector('.volume-slider') as HTMLInputElement;
    const muteBtn = this.container.querySelector('.volume-mute-btn') as HTMLButtonElement;
    const visBtn = this.container.querySelector('.vis-toggle-btn') as HTMLButtonElement;
    const lyricsBtn = this.container.querySelector('.lyrics-toggle-btn') as HTMLButtonElement;
    const queueBtn = this.container.querySelector('.queue-toggle-btn') as HTMLButtonElement;

    playBtn.addEventListener('click', () => appState.togglePlayPause());
    prevBtn.addEventListener('click', () => appState.previous());
    nextBtn.addEventListener('click', () => appState.next());

    seekSlider.addEventListener('input', (e) => {
      this.isScrubbing = true;
      const target = e.target as HTMLInputElement;
      this.scrubValue = parseFloat(target.value);
      const status = appState.getStatus();
      const currentMs = (this.scrubValue / 1000) * status.duration_ms;
      const currentTimeLabel = this.container.querySelector('.current-time') as HTMLElement;
      if (currentTimeLabel) {
        currentTimeLabel.textContent = formatTime(currentMs);
      }
      const fill = this.container.querySelector('.seek-fill') as HTMLElement;
      if (fill) fill.style.width = `${(this.scrubValue / 1000) * 100}%`;
    });

    seekSlider.addEventListener('change', (e) => {
      const target = e.target as HTMLInputElement;
      const val = parseFloat(target.value);
      const status = appState.getStatus();
      const targetMs = (val / 1000) * status.duration_ms;
      appState.seek(targetMs);
      this.isScrubbing = false;
    });

    volumeSlider.addEventListener('input', (e) => {
      const val = parseFloat((e.target as HTMLInputElement).value) / 100;
      appState.setVolume(val);
    });

    let lastVolume = 1.0;
    muteBtn.addEventListener('click', () => {
      const currentVol = appState.getStatus().volume;
      if (currentVol > 0) {
        lastVolume = currentVol;
        appState.setVolume(0);
      } else {
        appState.setVolume(lastVolume > 0 ? lastVolume : 1.0);
      }
    });

    visBtn.addEventListener('click', () => appState.toggleVisualizer());
    lyricsBtn?.addEventListener('click', () => {
      window.dispatchEvent(new CustomEvent('sonora-toggle-lyrics'));
    });
    queueBtn.addEventListener('click', () => appState.toggleQueue());
  }

  private async update() {
    const status = appState.getStatus();
    const track = status.current_track;

    // Track Title & Artist
    const titleEl = this.container.querySelector('.track-title') as HTMLElement;
    const artistEl = this.container.querySelector('.track-artist') as HTMLElement;
    if (titleEl && artistEl) {
      if (track) {
        titleEl.textContent = track.title || 'Untitled';
        titleEl.setAttribute('title', track.title || '');
        artistEl.textContent = track.artist || 'Unknown Artist';
      } else {
        titleEl.textContent = 'No track playing';
        titleEl.removeAttribute('title');
        artistEl.textContent = 'Select a track to start playback';
      }
    }

    // Play/Pause icon
    const playIcon = this.container.querySelector('.play-icon') as HTMLElement;
    const pauseIcon = this.container.querySelector('.pause-icon') as HTMLElement;
    if (playIcon && pauseIcon) {
      if (status.state === 'Playing') {
        playIcon.style.display = 'none';
        pauseIcon.style.display = 'block';
      } else {
        playIcon.style.display = 'block';
        pauseIcon.style.display = 'none';
      }
    }

    // Seek progress
    if (!this.isScrubbing) {
      const currentTimeLabel = this.container.querySelector('.current-time') as HTMLElement;
      const totalTimeLabel = this.container.querySelector('.total-time') as HTMLElement;
      const seekSlider = this.container.querySelector('.seek-slider') as HTMLInputElement;
      const seekFill = this.container.querySelector('.seek-fill') as HTMLElement;

      if (currentTimeLabel) currentTimeLabel.textContent = formatTime(status.position_ms);
      if (totalTimeLabel) totalTimeLabel.textContent = formatTime(status.duration_ms);

      if (seekSlider && seekFill) {
        const ratio = status.duration_ms > 0 ? status.position_ms / status.duration_ms : 0;
        const clampedRatio = Math.max(0, Math.min(1, ratio));
        seekSlider.value = (clampedRatio * 1000).toString();
        seekFill.style.width = `${clampedRatio * 100}%`;
      }
    }

    // Volume
    const volSlider = this.container.querySelector('.volume-slider') as HTMLInputElement;
    const volFill = this.container.querySelector('.volume-fill') as HTMLElement;
    if (volSlider && volFill) {
      const volPercent = Math.round(status.volume * 100);
      volSlider.value = volPercent.toString();
      volFill.style.width = `${Math.min(100, volPercent)}%`;
    }

    // Queue button active state & badge
    const queueBtn = this.container.querySelector('.queue-toggle-btn') as HTMLElement;
    const queueBadge = this.container.querySelector('.queue-badge') as HTMLElement;
    if (queueBtn) {
      if (appState.isQueueVisible()) {
        queueBtn.classList.add('active');
      } else {
        queueBtn.classList.remove('active');
      }
    }
    if (queueBadge) {
      if (status.queue_length > 0) {
        queueBadge.textContent = status.queue_length.toString();
        queueBadge.style.display = 'inline-flex';
      } else {
        queueBadge.style.display = 'none';
      }
    }

    // Visualizer button active state
    const visBtn = this.container.querySelector('.vis-toggle-btn') as HTMLElement;
    if (visBtn) {
      if (appState.isVisualizerVisible()) {
        visBtn.classList.add('active');
      } else {
        visBtn.classList.remove('active');
      }
    }

    // Lyrics button active state
    const lyricsBtn = this.container.querySelector('.lyrics-toggle-btn') as HTMLElement;
    if (lyricsBtn) {
      const isLyricsOpen = layoutManager.getRegions().lyrics;
      if (isLyricsOpen) {
        lyricsBtn.classList.add('active');
      } else {
        lyricsBtn.classList.remove('active');
      }
    }

    // Artwork
    const placeholderThumb = this.container.querySelector('.placeholder-thumb') as HTMLElement;
    const imgThumb = this.container.querySelector('.album-thumb-img') as HTMLImageElement;

    if (track?.track_id) {
      const art = await appState.fetchArtwork(track.track_id);
      if (art && imgThumb && placeholderThumb) {
        imgThumb.src = art;
        imgThumb.style.display = 'block';
        placeholderThumb.style.display = 'none';
      } else if (imgThumb && placeholderThumb) {
        imgThumb.style.display = 'none';
        placeholderThumb.style.display = 'flex';
      }
    } else if (imgThumb && placeholderThumb) {
      imgThumb.style.display = 'none';
      placeholderThumb.style.display = 'flex';
    }
  }
}
