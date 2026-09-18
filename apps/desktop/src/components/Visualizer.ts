/**
 * Sonora Audio Spectrum & Waveform Visualizer Component
 * 
 * Supports user-customizable visualizer modes:
 * - Spectrum Bars (48 logarithmic bands with peak hold caps)
 * - Oscilloscope Waveform (Continuous smoothed spline)
 * - Symmetrical Mirror Spectrum (Bilateral frequency radiation)
 * - Retro LED Ladder Blocks (Segmented VU-style meters)
 * 
 * Customization properties:
 * - Style selection, Sensitivity gain, Temporal smoothing (tau),
 * - Container height, Target FPS (30/60/120), Color source palette,
 * - Master enable/disable with zero-overhead sleep.
 */

import { api } from '../api';
import { visualizerConfigManager, VisualizerConfig } from '../customization';
import { appState } from '../state';

const NUM_BANDS = 48;
const ATTACK_TAU = 0.03;
const PEAK_HOLD_TIME = 0.25;
const PEAK_FALL_SPEED = 1.6;

export class VisualizerComponent {
  private container: HTMLElement;
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D | null;
  private animationFrameId: number | null = null;
  private pollIntervalId: number | null = null;

  // Frame-rate independent physics state
  private targetBars: number[] = new Array(NUM_BANDS).fill(0);
  private smoothedBars: number[] = new Array(NUM_BANDS).fill(0);
  private peakBars: number[] = new Array(NUM_BANDS).fill(0);
  private peakHoldTimers: number[] = new Array(NUM_BANDS).fill(0);
  private lastRenderTimestamp: number = 0;
  private lastDrawCallTimestamp: number = 0;
  private isPollingActive: boolean = false;
  private config: VisualizerConfig;

  constructor(container: HTMLElement) {
    this.container = container;
    this.config = visualizerConfigManager.getConfig();

    this.container.innerHTML = `
      <div class="visualizer-wrapper">
        <div class="visualizer-header">
          <div class="visualizer-title-group">
            <span class="visualizer-title">Audio Visualizer</span>
            <span class="visualizer-meta">48 Bands • FFT Spectrum</span>
          </div>
          <div class="visualizer-actions">
            <button class="icon-button close-vis" title="Close visualizer" aria-label="Close visualizer">✕</button>
          </div>
        </div>
        <div class="visualizer-body">
          <canvas class="visualizer-canvas" role="img" aria-label="Audio spectrum visualizer"></canvas>
          <div class="visualizer-disabled-overlay" style="display: none;">
            <p>Visualizer is disabled in Settings</p>
          </div>
        </div>
      </div>
    `;

    this.canvas = this.container.querySelector('.visualizer-canvas') as HTMLCanvasElement;
    this.ctx = this.canvas.getContext('2d');

    const closeBtn = this.container.querySelector('.close-vis') as HTMLButtonElement;
    closeBtn.addEventListener('click', () => {
      appState.toggleVisualizer(false);
    });

    // Subscribe to visualizer configuration changes
    visualizerConfigManager.subscribe((newConfig) => {
      this.config = newConfig;
      this.applyConfig();
    });

    window.addEventListener('resize', () => this.resizeCanvas());
    this.applyConfig();
    setTimeout(() => this.resizeCanvas(), 50);

    if (this.config.enabled) {
      this.start();
    }
  }

  private applyConfig() {
    const wrapper = this.container.querySelector('.visualizer-body') as HTMLElement;
    const disabledOverlay = this.container.querySelector('.visualizer-disabled-overlay') as HTMLElement;
    const titleEl = this.container.querySelector('.visualizer-title') as HTMLElement;
    const metaEl = this.container.querySelector('.visualizer-meta') as HTMLElement;

    if (wrapper) {
      wrapper.style.height = `${this.config.height}px`;
    }

    if (titleEl && metaEl) {
      switch (this.config.style) {
        case 'bars':
          titleEl.textContent = 'Spectrum Analyzer';
          metaEl.textContent = `48 Bands • ${this.config.fps} FPS • Gain ${this.config.sensitivity.toFixed(1)}x`;
          break;
        case 'wave':
          titleEl.textContent = 'Oscilloscope Wave';
          metaEl.textContent = `Continuous Spline • ${this.config.fps} FPS`;
          break;
        case 'mirror':
          titleEl.textContent = 'Bilateral Mirror Spectrum';
          metaEl.textContent = `Center Radiation • ${this.config.fps} FPS`;
          break;
        case 'led':
          titleEl.textContent = 'Retro LED Ladder';
          metaEl.textContent = `Segmented Meter • ${this.config.fps} FPS`;
          break;
      }
      if (this.canvas && titleEl.textContent) {
        this.canvas.setAttribute('aria-label', `${titleEl.textContent} (decorative audio visualization)`);
      }
    }

    if (!this.config.enabled) {
      this.stop();
      if (disabledOverlay) disabledOverlay.style.display = 'flex';
      if (this.canvas) this.canvas.style.display = 'none';
    } else {
      if (disabledOverlay) disabledOverlay.style.display = 'none';
      if (this.canvas) this.canvas.style.display = 'block';
      this.resizeCanvas();
      this.start();
    }
  }

  public resizeCanvas() {
    if (!this.canvas) return;
    const rect = this.canvas.parentElement?.getBoundingClientRect();
    if (rect && rect.width > 0 && rect.height > 0) {
      this.canvas.width = rect.width * window.devicePixelRatio;
      this.canvas.height = rect.height * window.devicePixelRatio;
    }
  }

  public start() {
    if (!this.config.enabled) return;
    if (this.animationFrameId !== null) return;

    this.startPolling();

    // Honor reduced-motion: render one static frame instead of animating.
    if (typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) {
      this.draw(performance.now());
      return;
    }

    this.lastRenderTimestamp = performance.now();
    this.lastDrawCallTimestamp = performance.now();
    const render = (timestamp: number) => {
      // FPS throttling
      const minInterval = 1000 / (this.config.fps || 60);
      const elapsed = timestamp - this.lastDrawCallTimestamp;

      if (elapsed >= minInterval) {
        this.draw(timestamp);
        this.lastDrawCallTimestamp = timestamp - (elapsed % minInterval);
      }

      if (this.config.enabled) {
        this.animationFrameId = requestAnimationFrame(render);
      }
    };
    this.animationFrameId = requestAnimationFrame(render);
  }

  public stop() {
    if (this.animationFrameId !== null) {
      cancelAnimationFrame(this.animationFrameId);
      this.animationFrameId = null;
    }
    this.stopPolling();
  }

  private startPolling() {
    if (this.pollIntervalId !== null) return;

    this.pollIntervalId = window.setInterval(async () => {
      if (this.isPollingActive || !this.config.enabled) return;
      if (appState.getStatus().state !== 'Playing') {
        this.targetBars.fill(0);
        return;
      }

      this.isPollingActive = true;
      try {
        const rawData = await api.getVisualizerData();
        const gain = this.config.sensitivity;
        for (let i = 0; i < NUM_BANDS; i++) {
          const raw = i < rawData.length ? rawData[i] : 0.0;
          this.targetBars[i] = Math.min(1.0, raw * gain);
        }
      } catch {
        this.targetBars.fill(0);
      } finally {
        this.isPollingActive = false;
      }
    }, 33);
  }

  private stopPolling() {
    if (this.pollIntervalId !== null) {
      clearInterval(this.pollIntervalId);
      this.pollIntervalId = null;
    }
  }

  private getColorGradient(ctx: CanvasRenderingContext2D, height: number): CanvasGradient {
    const gradient = ctx.createLinearGradient(0, height, 0, 0);
    const rootStyle = getComputedStyle(document.documentElement);

    switch (this.config.colorSource) {
      case 'monochrome':
        gradient.addColorStop(0.0, 'rgba(200, 210, 225, 0.40)');
        gradient.addColorStop(0.5, 'rgba(230, 240, 255, 0.75)');
        gradient.addColorStop(1.0, 'rgba(255, 255, 255, 0.95)');
        break;

      case 'gradient':
        gradient.addColorStop(0.0, 'rgba(6, 182, 212, 0.50)');  // Cyan
        gradient.addColorStop(0.35, 'rgba(99, 102, 241, 0.80)'); // Indigo
        gradient.addColorStop(0.70, 'rgba(236, 72, 153, 0.90)'); // Pink
        gradient.addColorStop(1.0, 'rgba(245, 158, 11, 1.0)');   // Amber
        break;

      case 'artwork':
      case 'accent':
      default: {
        const primary = rootStyle.getPropertyValue('--accent-primary').trim() || '#6366f1';
        const secondary = rootStyle.getPropertyValue('--accent-secondary').trim() || '#a855f7';
        gradient.addColorStop(0.0, primary + '70');
        gradient.addColorStop(0.6, secondary + 'cc');
        gradient.addColorStop(1.0, primary + 'ff');
        break;
      }
    }

    return gradient;
  }

  private draw(timestamp: number) {
    if (!this.ctx || !this.canvas || !this.config.enabled) return;
    const width = this.canvas.width;
    const height = this.canvas.height;
    if (width === 0 || height === 0) return;

    const ctx = this.ctx;
    ctx.clearRect(0, 0, width, height);

    const dt = Math.min((timestamp - this.lastRenderTimestamp) / 1000, 0.1);
    this.lastRenderTimestamp = timestamp;

    const decayTau = this.config.smoothing;

    // Physics smoothing
    for (let i = 0; i < NUM_BANDS; i++) {
      const target = this.targetBars[i];
      const tau = target > this.smoothedBars[i] ? ATTACK_TAU : decayTau;
      const alpha = 1.0 - Math.exp(-dt / tau);

      this.smoothedBars[i] += alpha * (target - this.smoothedBars[i]);

      if (this.smoothedBars[i] >= this.peakBars[i]) {
        this.peakBars[i] = this.smoothedBars[i];
        this.peakHoldTimers[i] = PEAK_HOLD_TIME;
      } else {
        if (this.peakHoldTimers[i] > 0) {
          this.peakHoldTimers[i] -= dt;
        } else {
          this.peakBars[i] = Math.max(0, this.peakBars[i] - PEAK_FALL_SPEED * dt);
        }
      }
    }

    const gradient = this.getColorGradient(ctx, height);

    switch (this.config.style) {
      case 'wave':
        this.drawWaveform(ctx, width, height, gradient);
        break;
      case 'mirror':
        this.drawMirrorBars(ctx, width, height, gradient);
        break;
      case 'led':
        this.drawLedBars(ctx, width, height, gradient);
        break;
      case 'bars':
      default:
        this.drawSpectrumBars(ctx, width, height, gradient);
        break;
    }
  }

  private drawSpectrumBars(ctx: CanvasRenderingContext2D, width: number, height: number, gradient: CanvasGradient) {
    const barWidth = (width / NUM_BANDS) * 0.72;
    const gap = (width / NUM_BANDS) * 0.28;
    ctx.fillStyle = gradient;

    for (let i = 0; i < NUM_BANDS; i++) {
      const normVal = Math.min(1.0, this.smoothedBars[i]);
      const barHeight = Math.max(3, normVal * (height - 18));
      const x = i * (barWidth + gap) + gap / 2;
      const y = height - barHeight;

      ctx.beginPath();
      const radius = Math.min(barWidth / 2, 4);
      ctx.roundRect(x, y, barWidth, barHeight, [radius, radius, 0, 0]);
      ctx.fill();

      // Peak hold cap
      const peakVal = Math.min(1.0, this.peakBars[i]);
      if (peakVal > 0.02) {
        const peakY = height - Math.max(4, peakVal * (height - 18));
        ctx.fillStyle = 'rgba(255, 255, 255, 0.85)';
        ctx.fillRect(x, peakY - 2, barWidth, 2);
        ctx.fillStyle = gradient;
      }
    }
  }

  private drawWaveform(ctx: CanvasRenderingContext2D, width: number, height: number, gradient: CanvasGradient) {
    ctx.strokeStyle = gradient;
    ctx.lineWidth = 3 * window.devicePixelRatio;
    ctx.lineJoin = 'round';
    ctx.lineCap = 'round';

    const midY = height / 2;
    ctx.beginPath();
    ctx.moveTo(0, midY);

    const step = width / (NUM_BANDS - 1);
    for (let i = 0; i < NUM_BANDS; i++) {
      const norm = this.smoothedBars[i];
      const sign = i % 2 === 0 ? 1 : -1;
      const amp = norm * (height * 0.42) * sign;
      const x = i * step;
      const y = midY - amp;

      if (i === 0) {
        ctx.moveTo(x, y);
      } else {
        const prevX = (i - 1) * step;
        const prevNorm = this.smoothedBars[i - 1];
        const prevSign = (i - 1) % 2 === 0 ? 1 : -1;
        const prevY = midY - (prevNorm * (height * 0.42) * prevSign);
        const cpX = (prevX + x) / 2;
        ctx.quadraticCurveTo(cpX, prevY, x, y);
      }
    }

    ctx.stroke();
  }

  private drawMirrorBars(ctx: CanvasRenderingContext2D, width: number, height: number, gradient: CanvasGradient) {
    const halfBands = Math.floor(NUM_BANDS / 2);
    const midX = width / 2;
    const barWidth = (midX / halfBands) * 0.70;
    const gap = (midX / halfBands) * 0.30;
    ctx.fillStyle = gradient;

    for (let i = 0; i < halfBands; i++) {
      const norm = Math.min(1.0, this.smoothedBars[i * 2]);
      const barHeight = Math.max(3, norm * (height - 18));
      const y = height - barHeight;
      const offset = i * (barWidth + gap);

      // Right bar
      const rightX = midX + offset + gap / 2;
      ctx.beginPath();
      ctx.roundRect(rightX, y, barWidth, barHeight, [3, 3, 0, 0]);
      ctx.fill();

      // Left bar
      const leftX = midX - offset - barWidth - gap / 2;
      ctx.beginPath();
      ctx.roundRect(leftX, y, barWidth, barHeight, [3, 3, 0, 0]);
      ctx.fill();
    }
  }

  private drawLedBars(ctx: CanvasRenderingContext2D, width: number, height: number, gradient: CanvasGradient) {
    const barWidth = (width / NUM_BANDS) * 0.75;
    const gap = (width / NUM_BANDS) * 0.25;
    const numBlocks = 14;
    const blockGap = 2 * window.devicePixelRatio;
    const totalBlockGaps = (numBlocks - 1) * blockGap;
    const blockHeight = (height - 20 - totalBlockGaps) / numBlocks;

    for (let i = 0; i < NUM_BANDS; i++) {
      const norm = Math.min(1.0, this.smoothedBars[i]);
      const activeBlocks = Math.round(norm * numBlocks);
      const x = i * (barWidth + gap) + gap / 2;

      for (let b = 0; b < numBlocks; b++) {
        const blockY = height - 10 - (b + 1) * blockHeight - b * blockGap;
        if (b < activeBlocks) {
          ctx.fillStyle = gradient;
        } else {
          ctx.fillStyle = 'rgba(255, 255, 255, 0.05)';
        }
        ctx.fillRect(x, blockY, barWidth, blockHeight);
      }
    }
  }
}
