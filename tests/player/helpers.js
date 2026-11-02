const { JSDOM, VirtualConsole } = require('jsdom');
const fs = require('node:fs');
const path = require('node:path');

const fixture = (name) => path.join(__dirname, '.fixtures', name);

// A tiny in-memory SCORM API. `mode`: '1.2' | '2004' | null (no LMS).
function boot(dir, mode = null, init = {}, before = null) {
  const store = { ...init };
  const log = [];
  const get = (k) => (store[k] !== undefined ? store[k] : k.includes('status') ? 'not attempted' : '');
  const set = (k, v) => { store[k] = String(v); log.push(`${k}=${v}`); return 'true'; };
  const api = mode === '1.2'
    ? { LMSInitialize: () => 'true', LMSGetValue: get, LMSSetValue: set, LMSCommit: () => 'true', LMSFinish: () => 'true' }
    : mode === '2004'
      ? { Initialize: () => 'true', GetValue: get, SetValue: set, Commit: () => 'true', Terminate: () => 'true' }
      : null;
  const errors = [];
  const vc = new VirtualConsole();
  vc.on('jsdomError', (e) => errors.push(e.message));
  const dom = new JSDOM(fs.readFileSync(path.join(dir, 'index.html'), 'utf8'), {
    runScripts: 'dangerously',
    pretendToBeVisual: true,
    virtualConsole: vc,
    beforeParse(w) {
      if (api) w[mode === '1.2' ? 'API' : 'API_1484_11'] = api;
      // jsdom has no Web Animations; record calls so tests can check them.
      // jsdom does not play media; a no-op keeps pause/play calls from being reported as errors.
      w.HTMLMediaElement.prototype.pause = function () {};
      w.HTMLMediaElement.prototype.play = function () { return Promise.resolve(); };
      // jsdom does no layout, so scrolling is a no-op.
      w.Element.prototype.scrollIntoView = function () {};
      // jsdom has no canvas: say so quietly (a test can install a fake WebGL context with `before`).
      w.HTMLCanvasElement.prototype.getContext = function () { return null; };
      w.__animations = [];
      w.Element.prototype.animate = function (keyframes, options) { w.__animations.push({ keyframes, options, text: this.textContent }); };
      if (before) before(w);
    },
  });
  return { w: dom.window, store, log, errors };
}

const $ = (w, s) => w.document.querySelector(s);
const $$ = (w, s) => [...w.document.querySelectorAll(s)];
const click = (w, e) => e.dispatchEvent(new w.MouseEvent('click', { bubbles: true }));
const currentSlide = (w) => Number(w.document.getElementById('stage').dataset.slide);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const visibleLayers = (w) => $$(w, '.layer').map((l) => l.style.display);

module.exports = { fixture, boot, $, $$, click, currentSlide, sleep, visibleLayers };
