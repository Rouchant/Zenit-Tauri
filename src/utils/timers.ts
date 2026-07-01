export const timers = {
  // ---------- RAF Watchdog ----------
  rafWatchdog: {
    active: false,
    lastTime: 0,
    stallTime: 0,
    frameId: null as number | null,
  },

  // ---------- Overlay timer ----------
  overlay: null as number | null,

  // ---------- Safety timer ----------
  safety: null as number | null,
};
