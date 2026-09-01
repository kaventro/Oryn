// src/modules/sysStatsController.ts

export interface SysStatsDeps {
  api: () => any;
  fmtSize?: ((sz: number | null | undefined, isDir?: boolean) => string) | null;
}

export class SysStatsController {
  public api: () => any;
  public fmtSize: ((sz: number | null | undefined, isDir?: boolean) => string) | null;
  private _timer: ReturnType<typeof setInterval> | null;
  private _inFlight: boolean;
  private _visible: boolean;
  private _STORAGE_KEY: string;

  constructor(deps: SysStatsDeps) {
    this.api = deps.api;
    this.fmtSize = typeof deps.fmtSize === 'function' ? deps.fmtSize : null;
    this._timer = null;
    this._inFlight = false;
    this._visible = false;
    this._STORAGE_KEY = 'Oryn.showSysStats';
  }

  private _storageOn(): boolean {
    const val = localStorage.getItem(this._STORAGE_KEY) ?? localStorage.getItem('Oryn.showSysStats') ?? localStorage.getItem('totalshark.showSysStats');
    return val !== 'false';
  }

  private _formatUptime(sec: number): string {
    const d = Math.floor(sec / 86400);
    const h = Math.floor((sec % 86400) / 3600);
    const m = Math.floor((sec % 3600) / 60);
    const s = Math.floor(sec % 60);
    if (d > 0) return `${d}d ${h}h ${m}m ${String(s).padStart(2, '0')}s`;
    if (h > 0) return `${h}h ${m}m ${String(s).padStart(2, '0')}s`;
    return `${m}m ${String(s).padStart(2, '0')}s`;
  }

  private async _tick(): Promise<void> {
    const wrap = document.getElementById('nx-sys-stats');
    const ow = this.api();
    if (
      !wrap || wrap.hidden || document.hidden || this._inFlight ||
      typeof ow.getSystemStats !== 'function'
    ) return;
    this._inFlight = true;
    try {
      const r = await ow.getSystemStats();
      if (!r || typeof r !== 'object') return;
      const cpu = Number(r.cpuPct ?? r.cpuPercent ?? 0).toFixed(0);
      const ramUsedBytes = Number(r.ramUsed);
      const ramTotalBytes = Number(r.ramTotal);
      const up = Number(r.uptimeSec ?? 0);
      
      const cpuEl = document.getElementById('nx-stat-cpu');
      if (cpuEl) cpuEl.textContent = `CPU ${cpu}%`;

      let ramLabel = 'RAM —';
      if (
        Number.isFinite(ramUsedBytes) &&
        ramUsedBytes >= 0 &&
        Number.isFinite(ramTotalBytes) &&
        ramTotalBytes > 0
      ) {
        const reportedPct = Number(r.ramPct);
        const pct = Number.isFinite(reportedPct)
          ? Math.min(100, Math.max(0, Math.round(reportedPct)))
          : Math.min(100, Math.max(0, Math.round((ramUsedBytes / ramTotalBytes) * 100)));
        ramLabel = `RAM ${pct}%`;
      } else if (Number.isFinite(ramUsedBytes) && ramUsedBytes >= 0 && this.fmtSize) {
        ramLabel = `RAM ${this.fmtSize(ramUsedBytes)}`;
      } else if (Number.isFinite(ramUsedBytes) && ramUsedBytes >= 0) {
        ramLabel = `RAM ${Math.round(ramUsedBytes / (1024 * 1024))}M`;
      }
      
      const ramEl = document.getElementById('nx-stat-ram');
      const upEl = document.getElementById('nx-stat-up');
      if (ramEl) ramEl.textContent = ramLabel;
      if (upEl) upEl.textContent = `UP ${this._formatUptime(up)}`;
    } catch {
      // ignore
    } finally {
      this._inFlight = false;
    }
  }

  private _stopPolling(): void {
    if (this._timer !== null) {
      clearInterval(this._timer);
      this._timer = null;
    }
  }

  public setVisible(show: boolean): void {
    const wrap = document.getElementById('nx-sys-stats');
    const btn = document.getElementById('sys-stats-toggle');
    if (!wrap || !btn) return;
    localStorage.setItem(this._STORAGE_KEY, show ? 'true' : 'false');
    this._visible = show;
    wrap.hidden = !show;
    btn.setAttribute('aria-pressed', show ? 'true' : 'false');
    btn.title = show
      ? 'Hide CPU / RAM / uptime (stops sampling while hidden)'
      : 'Show CPU / RAM / uptime';
    this._stopPolling();
    if (show && !document.hidden && typeof this.api().getSystemStats === 'function') {
      void this._tick();
      this._timer = setInterval(() => void this._tick(), 1000);
    }
  }

  public setup(): void {
    const btn = document.getElementById('sys-stats-toggle');
    const wrap = document.getElementById('nx-sys-stats');
    if (!btn || !wrap || typeof this.api().getSystemStats !== 'function') {
      if (btn) btn.hidden = true;
      if (wrap) wrap.hidden = true;
      return;
    }
    this.setVisible(this._storageOn());
    btn.addEventListener('click', () => {
      const w = document.getElementById('nx-sys-stats');
      if (w) this.setVisible(!!w.hidden);
    });
    document.addEventListener('visibilitychange', () => {
      if (document.hidden) {
        this._stopPolling();
        return;
      }
      if (this._visible) this.setVisible(true);
      this.tickClock();
    });
  }

  public tickClock(): void {
    const el = document.getElementById('nx-clock');
    if (!el) return;
    el.textContent = new Date().toLocaleTimeString('en-GB', { hour12: false });
  }
}
