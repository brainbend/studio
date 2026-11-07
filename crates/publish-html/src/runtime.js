// BrainBend Studio web player (original implementation). Renders slide objects as absolutely
// positioned elements, runs triggers, plays question and results slides, and talks to a
// SCORM 1.2 / 2004 LMS when packaged for one. Exposes GetPlayer() for course scripts.
(function () {
  'use strict';
  var P = window.__BRAINBEND__;
  var slides = P.slides;
  var stage = document.getElementById('stage');
  stage.style.width = slides[0].width + 'px';
  stage.style.height = slides[0].height + 'px';

  function unwrap(v) { return v === undefined || v === null ? v : Object.values(v)[0]; }
  // Slide background: a bare [r,g,b,a] color, a {angle, stops} gradient or an {asset} picture.
  function gradCss(g) {
    return 'linear-gradient(' + g.angle + 'deg,' + g.stops.map(function (s) { return css(s[1]) + ' ' + (s[0] * 100) + '%'; }).join(',') + ')';
  }
  function bgCss(b, w, h) {
    if (Array.isArray(b)) return css(b);
    if (b.stops) {
      var list = function (st) { return st.map(function (s) { return css(s[1]) + ' ' + (s[0] * 100).toFixed(2) + '%'; }).join(','); };
      var kind = b.kind || 'Linear';
      var g = b.geom || {};
      var ctr = g.center || [0.5, 0.5];
      var cx = (ctr[0] * 100).toFixed(2) + '%', cy = (ctr[1] * 100).toFixed(2) + '%';
      if (kind === 'Radial') {
        if (!b.geom) return 'radial-gradient(ellipse farthest-corner at 50% 50%,' + list(b.stops) + ')';
        var r = g.radius === undefined ? Math.SQRT1_2 : g.radius;
        return 'radial-gradient(ellipse ' + (r * w).toFixed(1) + 'px ' + (r * h).toFixed(1) + 'px at ' + cx + ' ' + cy + ',' + list(b.stops) + ')';
      }
      if (kind === 'Angular') return 'conic-gradient(from ' + b.angle + 'deg at ' + cx + ' ' + cy + ',' + list(b.stops) + ')';
      var st = b.stops;
      if (kind === 'Reflected') st = st.map(function (s) { return [s[0] / 2, s[1]]; }).concat(st.slice().reverse().map(function (s) { return [1 - s[0] / 2, s[1]]; }));
      if (!g.line) return 'linear-gradient(' + b.angle + 'deg,' + list(st) + ')';
      var x0 = g.line[0] * w, y0 = g.line[1] * h, dx = g.line[2] * w - x0, dy = g.line[3] * h - y0;
      var l = Math.max(1, Math.sqrt(dx * dx + dy * dy)), th = Math.atan2(dx, -dy);
      var ux = Math.sin(th), uy = -Math.cos(th), len = Math.abs(w * Math.sin(th)) + Math.abs(h * Math.cos(th));
      var off = (x0 - w / 2) * ux + (y0 - h / 2) * uy + len / 2;
      var deg = ((th * 180 / Math.PI) % 360 + 360) % 360;
      return 'linear-gradient(' + deg.toFixed(2) + 'deg,' + list(st.map(function (s) { return [(off + s[0] * l) / Math.max(1, len), s[1]]; })) + ')';
    }
    if (b.asset && P.assetUrls[b.asset]) return 'url("' + P.assetUrls[b.asset] + '")';
    return 'transparent';
  }
  function css(c) { return c ? 'rgba(' + c[0] + ',' + c[1] + ',' + c[2] + ',' + c[3] / 255 + ')' : 'transparent'; }
  function el(tag, cls, text) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text !== undefined) e.textContent = text;
    return e;
  }
  function kindOf(slide) { return typeof slide.kind === 'string' ? slide.kind : Object.keys(slide.kind)[0]; }

  // ---- accessibility plumbing: a heading naming the slide, a spoken announcement on each change, and
  // a note of when the learner last used the keyboard or pointer (so focus only moves for their actions)
  var titleEl = el('div', 'sr-only'); titleEl.setAttribute('role', 'heading'); titleEl.setAttribute('aria-level', '2'); titleEl.id = 'slide-title'; titleEl.tabIndex = -1; stage.appendChild(titleEl);
  var announcer = el('div', 'sr-only'); announcer.id = 'announce';
  announcer.setAttribute('role', 'status'); announcer.setAttribute('aria-live', 'polite'); document.body.appendChild(announcer);
  var lastInput = 0;
  ['pointerdown', 'mousedown', 'click', 'keydown'].forEach(function (t) {
    document.addEventListener(t, function () { lastInput = Date.now(); }, true);
  });
  function byUser() { return Date.now() - lastInput < 500; }
  function shown(n) {
    for (var p = n; p && p !== document.body; p = p.parentNode) {
      if (p.style && (p.style.display === 'none' || p.style.visibility === 'hidden')) return false;
    }
    return true;
  }
  function focusables(root) {
    return Array.prototype.filter.call(root.querySelectorAll('[tabindex="0"],button,a[href],input,select,textarea,video[controls],audio[controls]'), function (n) {
      return !n.disabled && shown(n);
    });
  }
  function focusIt(n) { try { n.focus({ preventScroll: true }); } catch (e) { /* not focusable right now */ } }

  // ---- variables ---------------------------------------------------------------------
  var vars = {}, byName = {};
  P.variables.forEach(function (v) { vars[v.id] = unwrap(v.initial); byName[v.name] = v.id; });
  var depth = 0, textVarEntries = [], controlUpdaters = [], markerClosers = [];
  var cur = -1;

  function setVar(id, value) {
    if (vars[id] === value) return;
    vars[id] = value;
    textVarEntries.forEach(draw);
    controlUpdaters.forEach(function (f) { f(); });
    saveSoon();
    if (depth > 20) return; // guard against trigger loops
    depth++;
    try {
      (slides[cur] ? slides[cur].triggers : []).forEach(function (t) {
        if (t.event && t.event.VariableChanged === id) runTrigger(t);
      });
    } finally { depth--; }
  }
  window.GetPlayer = function () {
    return {
      GetVar: function (n) { return vars[byName[n]]; },
      SetVar: function (n, v) { if (byName[n] !== undefined) setVar(byName[n], v); }
    };
  };

  // ---- quiz bookkeeping ---------------------------------------------------------------
  var answers = {};            // slide id -> { c: correct, e: earned, s: [chosen ids] }
  var visited = {};
  var questionSlides = slides.filter(function (s) { return kindOf(s) === 'Question'; });
  var hasResults = slides.some(function (s) { return kindOf(s) === 'Results'; });

  // ---- question banks: a bank scene shows a random draw of its questions each attempt ---------------
  // `seq` is the play order (slide indices); slides left out of a draw are not in it (posOf = -1).
  var seq = slides.map(function (_, i) { return i; }), posOf = seq.slice(), drawn = {};
  var banks = (P.scenes || []).filter(function (sc) { return sc.bank; });
  function redraw(fresh) {
    if (fresh) drawn = {};
    var chosen = {};
    banks.forEach(function (sc) {
      var qs = [];
      slides.forEach(function (s, i) { if (s.scene === sc.id && kindOf(s) === 'Question') qs.push(i); });
      var ids = drawn[sc.id], pick = null;
      if (ids && ids.length && ids.every(function (id) { return qs.some(function (i) { return slides[i].id === id; }); })) {
        pick = ids.map(function (id) { return qs.filter(function (i) { return slides[i].id === id; })[0]; });
      }
      if (!pick) {
        var pool = qs.slice();
        var n = sc.bank.draw > 0 && sc.bank.draw < pool.length ? sc.bank.draw : pool.length;
        for (var k = pool.length - 1; k > 0; k--) { var j = Math.floor(Math.random() * (k + 1)); var t = pool[k]; pool[k] = pool[j]; pool[j] = t; }
        pick = pool.slice(0, n);
        if (!sc.bank.shuffle) pick.sort(function (a, b) { return a - b; });
        drawn[sc.id] = pick.map(function (i) { return slides[i].id; });
      }
      chosen[sc.id] = { pick: pick, k: 0 };
    });
    seq = [];
    slides.forEach(function (s, i) {
      var c = chosen[s.scene];
      if (c && kindOf(s) === 'Question') { if (c.k < c.pick.length) seq.push(c.pick[c.k++]); return; }
      seq.push(i);
    });
    posOf = slides.map(function () { return -1; });
    seq.forEach(function (i, p) { posOf[i] = p; });
    questionSlides = seq.filter(function (i) { return kindOf(slides[i]) === 'Question'; }).map(function (i) { return slides[i]; });
  }
  function step(d) { var p = posOf[cur] + d; return p >= 0 && p < seq.length ? seq[p] : -1; }
  function totals() {
    var max = 0, earned = 0;
    questionSlides.forEach(function (s) {
      var q = s.kind.Question; max += q.points;
      if (answers[s.id]) earned += answers[s.id].e;
    });
    var pct = max > 0 ? Math.round(earned / max * 100) : 0;
    return { earned: earned, max: max, pct: pct, passed: pct >= P.pass_mark, answered: Object.keys(answers).length };
  }

  // ---- LMS (SCORM 1.2 / 2004) ---------------------------------------------------------
  // Mode comes from the export: 'none' | '1.2' | '2004'. The player finds the LMS API object
  // by walking up parent frames (and the opener), as the standards describe.
/*@LMS@*/

  // Suspend data (v2): authored variables, quiz answers, visited slides.
  var saveTimer = null;
  function saveSoon() {
    if (!lms) return;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(saveNow, 500);
  }
  function saveNow() {
    if (!lms) return;
    var out = {};
    P.variables.forEach(function (v) {
      if (!v.built_in && vars[v.id] !== unwrap(v.initial)) out[v.id] = vars[v.id];
    });
    var data = JSON.stringify({ v: 2, vars: out, ans: answers, vis: Object.keys(visited), dr: drawn });
    if (data.length > lms.limit) {
      data = JSON.stringify({ v: 2, vars: out, ans: {}, vis: [] });
      console.warn('BrainBend Studio: suspend data too large; saved variables only');
    }
    if (data.length > lms.limit) return;
    lms.set(lms.suspendKey, data);
    lms.commit();
  }
  function restore() {
    if (!lms) return -1;
    var raw = lms.get(lms.suspendKey);
    if (raw) {
      try {
        var d = JSON.parse(raw);
        if (d && (d.v === 1 || d.v === 2)) {
          Object.keys(d.vars || {}).forEach(function (id) {
            if (id in vars && typeof d.vars[id] === typeof vars[id]) vars[id] = d.vars[id];
          });
          if (d.v === 2) {
            answers = d.ans || {};
            drawn = d.dr || {};
            (d.vis || []).forEach(function (k) { visited[k] = true; });
          }
        }
      } catch (e) { console.warn('BrainBend Studio: ignoring unreadable suspend data'); }
    }
    var loc = parseInt(lms.get(lms.locationKey), 10);
    return isNaN(loc) ? -1 : loc;
  }

  // ---- slides, layers and objects ------------------------------------------------------
  var layers = {}, objects = {}, slideEls = [], objectSlide = {};
  slides.forEach(function (slide, si) {
    var sEl = el('div', 'slide');
    sEl.style.display = 'none';
    if (slide.background) {
      sEl.style.background = bgCss(slide.background, slide.width, slide.height);
      if (slide.background.asset) { sEl.style.backgroundSize = 'cover'; sEl.style.backgroundPosition = 'center'; sEl.style.backgroundRepeat = 'no-repeat'; }
    }
    stage.appendChild(sEl);
    slideEls.push(sEl);
    slide.layers.forEach(function (layer, li) {
      var lEl = el('div', 'layer');
      lEl.dataset.layer = layer.id;
      if (li > 0 && layer.name) { lEl.setAttribute('role', 'group'); lEl.setAttribute('aria-label', layer.name); }
      lEl.style.display = li === 0 ? 'block' : 'none';
      // On a question or results slide the base layer lets clicks through to the question beneath (its objects still take them).
      if (li === 0 && kindOf(slide) !== 'Normal') lEl.style.pointerEvents = 'none';
      // A layer that is not modal lets clicks through to what is beneath it (its own objects still take them).
      if (li > 0 && layer.modal === false) lEl.style.pointerEvents = 'none';
      sEl.appendChild(lEl);
      layers[layer.id] = { el: lEl, base: li === 0, layer: layer, si: si };
      layer.objects.forEach(function (o) {
        var node = el('div', 'obj');
        node.style.left = o.x + 'px'; node.style.top = o.y + 'px';
        if (o.rotation) node.style.rotate = o.rotation + 'deg';
        node.style.width = o.w + 'px'; node.style.height = o.h + 'px';
        if (o.hidden) node.style.display = 'none';
        if (o.kind === 'Ellipse') { node.style.borderRadius = '50%'; node.style.overflow = 'hidden'; }
        else if (o.radius) { node.style.borderRadius = o.radius + 'px'; node.style.overflow = 'hidden'; }
        if (o.kind === 'Image' && o.asset && P.assetUrls[o.asset]) {
          var img = el('img');
          img.src = P.assetUrls[o.asset]; img.draggable = false; img.alt = o.alt || '';
          if (o.crop) {
            // The whole picture is stretched so the part left visible fills the frame; the frame clips the rest.
            var cw = Math.max(0.01, 1 - o.crop[0] - o.crop[2]), ch = Math.max(0.01, 1 - o.crop[1] - o.crop[3]);
            img.style.cssText = 'max-width:none;inset:auto;left:' + (-o.crop[0] / cw * 100) + '%;top:' + (-o.crop[1] / ch * 100) + '%;width:' + (100 / cw) + '%;height:' + (100 / ch) + '%';
            node.style.overflow = 'hidden';
          }
          if (o.mask_clip) {
            // Cropped to a shape: the shape is the frame's, whatever the crop does to the picture inside it.
            var shapeFrame = el('div'); shapeFrame.className = 'maskframe';
            shapeFrame.style.cssText = 'position:absolute;inset:0;clip-path:' + o.mask_clip;
            shapeFrame.appendChild(img); node.appendChild(shapeFrame);
          } else node.appendChild(img);
          node._img = img;
        }
        // A web object: the page in a frame, named for screen readers by the object's alt text.
        if (o.kind && o.kind.Web) {
          var wurl = String(o.kind.Web.url || '').trim();
          node.classList.add('media');
          if (/^https?:\/\//i.test(wurl)) {
            var fr = el('iframe');
            fr.src = wurl; fr.title = o.alt || o.name || 'Web object';
            fr.setAttribute('referrerpolicy', 'no-referrer'); fr.setAttribute('loading', 'lazy');
            fr.setAttribute('sandbox', 'allow-scripts allow-same-origin allow-forms allow-popups allow-popups-to-escape-sandbox');
            if (!o.kind.Web.scroll) fr.setAttribute('scrolling', 'no');
            fr.style.cssText = 'display:block;width:100%;height:100%;border:0;background:#fff';
            node.appendChild(fr);
          }
        }
        if ((o.kind === 'Video' || o.kind === 'Audio') && o.asset && P.assetUrls[o.asset]) {
          var media = el(o.kind === 'Video' ? 'video' : 'audio');
          media.src = P.assetUrls[o.asset]; media.preload = 'metadata';
          if (o.poster && P.assetUrls[o.poster]) media.poster = P.assetUrls[o.poster];
          var mp = o.media || { autoplay: false, controls: true, looped: false, muted: false, volume: 1 };
          media.controls = o.kind === 'Audio' && mp.controls; media.loop = mp.looped; media.muted = mp.muted; media.dataset.authorMuted = mp.muted ? '1' : ''; media.volume = mp.volume;
          media.setAttribute('aria-label', o.alt || o.name || (o.kind === 'Video' ? 'Video' : 'Audio'));
          // Cue points: "media reaches" fires once each time playback passes the time (again after seeking back).
          var cues = {};
          function cueTriggers() {
            return (objectSlide[o.id] !== undefined ? slides[objectSlide[o.id]].triggers : []).filter(function (t) {
              return t.owner.Object === o.id && t.event && typeof t.event === 'object' && t.event.MediaReaches !== undefined;
            });
          }
          media.addEventListener('timeupdate', function () {
            var ms = media.currentTime * 1000;
            cueTriggers().forEach(function (t) {
              var at = t.event.MediaReaches;
              if (ms >= at && !cues[t.id]) { cues[t.id] = true; runTrigger(t); }
              else if (ms < at - 250) cues[t.id] = false;
            });
          });
          media.addEventListener('ended', function () {
            cues = {};
          });
          media.addEventListener('ended', function () {
            (objectSlide[o.id] !== undefined ? slides[objectSlide[o.id]].triggers : []).forEach(function (t) {
              if (t.event === 'MediaEnded' && t.owner.Object === o.id) runTrigger(t);
            });
          });
          if (o.kind === 'Video') {
            media.setAttribute('playsinline', '');
            if (o.captions && P.assetUrls[o.captions]) {
              var track = el('track'); track.kind = 'captions'; track.src = P.assetUrls[o.captions];
              track.srclang = 'en'; track.label = 'Captions'; track.default = true;
              media.appendChild(track);
            }
          }
          node.appendChild(media);
          node.classList.add('media');
        }
        if (o.kind === 'Line' || o.kind === 'LineUp') {
          var svgNs = 'http://www.w3.org/2000/svg';
          var svg = document.createElementNS(svgNs, 'svg');
          // A perfectly vertical or horizontal line has no width or height, and an SVG of size zero draws nothing.
          svg.setAttribute('width', o.w > 0 ? '100%' : '1'); svg.setAttribute('height', o.h > 0 ? '100%' : '1');
          svg.setAttribute('aria-hidden', 'true');
          svg.style.cssText = 'position:absolute;left:0;top:0;overflow:visible';
          var ln = document.createElementNS(svgNs, 'line');
          ln.setAttribute('x1', '0'); ln.setAttribute('x2', o.w);
          ln.setAttribute('y1', o.kind === 'Line' ? '0' : String(o.h)); ln.setAttribute('y2', o.kind === 'Line' ? String(o.h) : '0');
          svg.appendChild(ln); node.appendChild(svg);
          node._line = ln;
        }
        if (o.kind && o.kind.Path) {
          var ns = 'http://www.w3.org/2000/svg';
          var psvg = document.createElementNS(ns, 'svg');
          psvg.setAttribute('viewBox', '0 0 1 1'); psvg.setAttribute('preserveAspectRatio', 'none');
          psvg.setAttribute('width', '100%'); psvg.setAttribute('height', '100%'); psvg.setAttribute('aria-hidden', 'true');
          psvg.style.cssText = 'position:absolute;left:0;top:0;overflow:visible';
          var pth = document.createElementNS(ns, 'path');
          pth.setAttribute('d', o.kind.Path.map(function (c) {
            if (c === 'Close') return 'Z';
            if (c.Move) return 'M' + c.Move.join(' ');
            if (c.Line) return 'L' + c.Line.join(' ');
            return 'C' + c.Curve.join(' ');
          }).join(''));
          pth.setAttribute('vector-effect', 'non-scaling-stroke');
          psvg.appendChild(pth); node.appendChild(psvg);
          node._path = pth;
          if (o.gradient) {
            var defs = document.createElementNS(ns, 'defs'), lg = document.createElementNS(ns, 'linearGradient');
            var gid = 'grad-' + o.id.replace(/[^A-Za-z0-9_-]/g, '_'), a = o.gradient.angle * Math.PI / 180;
            lg.setAttribute('id', gid);
            lg.setAttribute('x1', 0.5 - Math.sin(a) / 2); lg.setAttribute('y1', 0.5 + Math.cos(a) / 2);
            lg.setAttribute('x2', 0.5 + Math.sin(a) / 2); lg.setAttribute('y2', 0.5 - Math.cos(a) / 2);
            o.gradient.stops.forEach(function (st) {
              var stop = document.createElementNS(ns, 'stop');
              stop.setAttribute('offset', st[0]); stop.setAttribute('stop-color', css(st[1]));
              lg.appendChild(stop);
            });
            defs.appendChild(lg); psvg.insertBefore(defs, pth);
            node._gradId = gid;
          }
        }
        // A hotspot with an outline is clickable only inside it.
        if (o.kind === 'Hotspot' && o.hit_area && o.hit_area.length > 2) {
          node.style.clipPath = 'polygon(' + o.hit_area.map(function (p) { return (p[0] * 100).toFixed(2) + '% ' + (p[1] * 100).toFixed(2) + '%'; }).join(',') + ')';
        }
        // A heading is published as a real heading element (h1 to h4); other text stays a plain block.
        var label = el({ H1: 'h1', H2: 'h2', H3: 'h3', H4: 'h4' }[o.role] || 'div', 'label');
        var m = o.margins || [0, 0, 0, 0];
        label.style.padding = m[1] + 'px ' + m[2] + 'px ' + m[3] + 'px ' + m[0] + 'px';
        node.appendChild(label);
        if (o.decorative && !o.states.length && !slides[si].triggers.some(function (t) { return t.event === 'Click' && t.owner.Object === o.id; })) node.setAttribute('aria-hidden', 'true');
        lEl.appendChild(node);
        var entry = { node: node, label: label, o: o, si: si, layerId: layer.id, state: o.initial_state, hover: false, down: false };
        objects[o.id] = entry;
        objectSlide[o.id] = si;
        if (/%[A-Za-z0-9_.]+%/.test(o.text) || o.states.some(function (x) { return /%[A-Za-z0-9_.]+%/.test(x.text); })) textVarEntries.push(entry);
        draw(entry);
        if (o.kind && o.kind.Slider) buildSlider(entry, o.kind.Slider);
        if (o.dial) buildDial(entry, o.dial);
        if (o.check) buildCheck(entry, o.check);
        if (o.kind && o.kind.Input) buildInput(entry, o.kind.Input);
        // Hover is also what keyboard focus does, so nothing is reachable only with a mouse.
        function fire(name) { slides[si].triggers.forEach(function (t) { if (t.event === name && t.owner.Object === o.id) runTrigger(t); }); }
        function enterHover(src) {
          var was = entry.hover || entry.focusHover;
          entry[src] = true; draw(entry);
          if (was || entry.state === 'Disabled') return;
          fire('Hover');
        }
        function leaveHover(src) {
          var other = src === 'hover' ? entry.focusHover : entry.hover;   // still hovered the other way?
          entry[src] = false; if (src === 'hover') entry.down = false;
          draw(entry);
          if (other || entry.state === 'Disabled') return;
          fire('MouseLeave');
        }
        node.addEventListener('mouseenter', function () { enterHover('hover'); });
        node.addEventListener('dblclick', function () {
          if (entry.state === 'Disabled') return;
          slides[si].triggers.forEach(function (t) { if (t.event === 'DoubleClick' && t.owner.Object === o.id) runTrigger(t); });
        });
        node.addEventListener('mouseleave', function () { leaveHover('hover'); });
        node.addEventListener('mousedown', function () { entry.down = true; draw(entry); });
        node.addEventListener('mouseup', function () { entry.down = false; draw(entry); });
        node.addEventListener('click', function () {
          if (entry.state === 'Disabled') return;
          selectInSet(entry);
          slides[si].triggers.forEach(function (t) {
            if (t.event === 'Click' && t.owner.Object === o.id) runTrigger(t);
          });
          // A button with a Visited state shows it from the first press on (unless a trigger or a set has moved it elsewhere).
          if (has(o, 'Visited') && (entry.state === 'Normal' || entry.state === o.initial_state)) setObjectState(entry, 'Visited');
        });
        // Anything with a click trigger (or button states) behaves as a button: pointer cursor,
        // keyboard focus, Enter/Space activation, and an accessible name.
        var clickable = !o.check && (buttonLike(o) || slides[si].triggers.some(function (t) { return t.event === 'Click' && t.owner.Object === o.id; }));
        if (clickable) {
          node.setAttribute('role', 'button'); node.tabIndex = 0;
          if (o.alt) node.setAttribute('aria-label', o.alt);
          entry.clickable = true;
          node.addEventListener('keydown', function (ev) { if (ev.key === 'Enter' || ev.key === ' ') { ev.preventDefault(); node.click(); } });
          draw(entry);
        }
        var hoverOnly = !clickable && node.getAttribute('tabindex') === null && slides[si].triggers.some(function (t) { return (t.event === 'Hover' || t.event === 'MouseLeave') && t.owner.Object === o.id; });
        if (hoverOnly) node.tabIndex = 0;
        // A picture or shape with alt text and nothing to read is announced as an image of that.
        if (o.alt && !o.decorative && node.getAttribute('role') === null && node.getAttribute('tabindex') === null &&
            ['Image', 'Video', 'Audio'].indexOf(o.kind) < 0 && !(o.kind && (o.kind.Input || o.kind.Web || o.kind.Pano)) && !o.equation && !(o.text && o.text.trim())) {
          node.setAttribute('role', 'img'); node.setAttribute('aria-label', o.alt);
        }
        if (node.getAttribute('tabindex') !== null) {
          node.addEventListener('focus', function () {
            var vis = true;
            try { vis = node.matches(':focus-visible'); } catch (e) { /* older engines: show the ring */ }
            entry.ring = vis;
            if (vis) enterHover('focusHover'); else draw(entry);
          });
          node.addEventListener('blur', function () {
            entry.ring = false;
            if (entry.focusHover) leaveHover('focusHover'); else draw(entry);
          });
        }
        if (o.marker) buildMarker(entry, o.marker, lEl);
        if (o.equation) buildEquation(node, o);
        if (o.kind && o.kind.Pano) buildPano(node, o, entry);
      });
      // Keyboard and screen-reader order: stacking order unless the author set a focus order. Stacking is by z-index,
      // so the nodes can then be put in the DOM in focus order without changing what is in front of what.
      if (layer.focus_order && layer.focus_order.length) {
        var byId = {};
        layer.objects.forEach(function (o, n) { byId[o.id] = n; });
        var nodes = Array.prototype.slice.call(lEl.children).filter(function (c) { return c.classList.contains('obj'); });
        nodes.forEach(function (c, n) { c.style.zIndex = String(n + 1); });
        var seen = {};
        var order = [];
        layer.focus_order.forEach(function (id) { if (byId[id] !== undefined && !seen[id]) { seen[id] = 1; order.push(byId[id]); } });
        layer.objects.forEach(function (o, n) { if (!seen[o.id]) order.push(n); });
        order.forEach(function (n) { if (nodes[n]) lEl.appendChild(nodes[n]); });
      }
    });
  });

  // An equation is drawn as outlines (hidden from assistive technology) and carries its MathML, which screen readers
  // read as maths. The MathML comes from the project file, so it is rebuilt element by element from a short list of
  // MathML names and attributes: nothing else from it reaches the page.
  function safeMathML(src) {
    var MATHML_ATTRS = ['display', 'mathvariant', 'mathsize', 'stretchy', 'fence', 'separator', 'columnalign', 'rowalign', 'columnspan', 'rowspan',
      'linethickness', 'displaystyle', 'scriptlevel', 'form', 'lspace', 'rspace', 'largeop', 'movablelimits', 'accent', 'accentunder', 'open', 'close',
      'symmetric', 'minsize', 'maxsize', 'depth', 'height', 'width', 'columnspacing', 'rowspacing', 'frame', 'notation', 'columnlines', 'rowlines',
      'equalrows', 'equalcolumns', 'align', 'voffset'];
    if (!src || typeof DOMParser === 'undefined') return null;
    var ns = 'http://www.w3.org/1998/Math/MathML';
    var root = new DOMParser().parseFromString(src, 'text/html').querySelector('math');
    if (!root) return null;
    function copy(n) {
      if (n.nodeType === 3) return document.createTextNode(n.nodeValue);
      if (n.nodeType !== 1) return null;
      var tag = n.localName.toLowerCase();
      if (!/^(math|m[a-z]+|semantics|annotation)$/.test(tag) || tag === 'annotation-xml') return null;
      var e = document.createElementNS(ns, tag);
      MATHML_ATTRS.forEach(function (a) { if (n.hasAttribute(a) && !/^(javascript|data):/i.test(n.getAttribute(a))) e.setAttribute(a, n.getAttribute(a)); });
      for (var c = n.firstChild; c; c = c.nextSibling) { var k = copy(c); if (k) e.appendChild(k); }
      return e;
    }
    return copy(root);
  }
  function buildEquation(node, o) {
    node.setAttribute('role', 'math');
    var svg = node.querySelector('svg'); if (svg) svg.setAttribute('aria-hidden', 'true');
    var ml = safeMathML(o.equation.mathml);
    if (ml) { var wrap = el('span', 'sr-only'); wrap.appendChild(ml); node.appendChild(wrap); if (o.alt) node.setAttribute('aria-label', o.alt); }
    else node.setAttribute('aria-label', o.alt || o.equation.latex);
  }

  // A marker: its title shows on hover or keyboard focus; clicking it opens a card with the title and a description,
  // and clicking anywhere else (or Escape) closes the card.
  function buildMarker(entry, m, layerEl) {
    var node = entry.node, o = entry.o, S = slides[entry.si];
    var rm = !!(window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);
    var title = String(m.title || '').trim(), desc = String(m.description || '').trim();
    node.setAttribute('role', 'button'); node.tabIndex = 0; node.style.cursor = 'pointer';
    node.setAttribute('aria-label', title || o.alt || o.name || 'Marker');
    // The marker swells a little under the pointer and presses in on a click; its focus ring follows its round shape.
    node.style.borderRadius = '50%';
    if (!rm) node.style.transition = 'scale .14s ease-out';
    node.addEventListener('mouseenter', function () { if (!rm) node.style.scale = '1.1'; });
    node.addEventListener('mouseleave', function () { node.style.scale = ''; });
    node.addEventListener('mousedown', function () { if (!rm) node.style.scale = '0.94'; });
    node.addEventListener('mouseup', function () { if (!rm) node.style.scale = '1.1'; });
    entry.clickable = true;
    var FONT = '"Open Sans", Montserrat, Arial, sans-serif', EDGE = '#4a4a4a';
    function card(cls) {
      var c = el('div', 'marker-card ' + cls);
      c.style.cssText = 'position:absolute;display:none;z-index:60;box-sizing:border-box;background:#fff;color:#111;border:1.5px solid ' + EDGE + ';border-radius:14px;font-family:' + FONT + ';pointer-events:auto;cursor:default';
      var ns = 'http://www.w3.org/2000/svg', svg = document.createElementNS(ns, 'svg');
      svg.setAttribute('width', '40'); svg.setAttribute('height', '18'); svg.setAttribute('aria-hidden', 'true');
      svg.style.cssText = 'position:absolute;pointer-events:none';
      var cover = document.createElementNS(ns, 'path'), line = document.createElementNS(ns, 'path');
      cover.setAttribute('fill', '#fff'); line.setAttribute('fill', 'none'); line.setAttribute('stroke', EDGE); line.setAttribute('stroke-width', '1.5');
      svg.appendChild(cover); svg.appendChild(line); c.appendChild(svg);
      c._svg = svg; c._cover = cover; c._line = line;
      layerEl.appendChild(c);
      return c;
    }
    // Put a card above (or below) the marker, with its pointer on the marker.
    function place(c, cw, ch) {
      var cx = o.x + o.w / 2, above = o.y - 18 - ch >= 0;
      var left = Math.min(Math.max(cx - cw / 2, 8), Math.max(8, S.width - cw - 8));
      var top = above ? o.y - 18 - ch : o.y + o.h + 18;
      var nx = Math.min(Math.max(cx - left, 34), Math.max(34, cw - 34));
      c.style.left = left + 'px'; c.style.top = top + 'px';
      c._svg.style.left = (nx - 20) + 'px';
      if (above) { c._svg.style.top = ''; c._svg.style.bottom = '-17px'; c._cover.setAttribute('d', 'M0.8 0 L20 15.8 L39.2 0 Z'); c._line.setAttribute('d', 'M0 0 L20 16.5 L40 0'); }
      else { c._svg.style.bottom = ''; c._svg.style.top = '-17px'; c._cover.setAttribute('d', 'M0.8 18 L20 2.2 L39.2 18 Z'); c._line.setAttribute('d', 'M0 18 L20 1.5 L40 18'); }
    }
    function addTitle(c, text) { var t = el('div', 'mk-title', text); t.style.cssText = 'font-weight:700;font-size:40px;line-height:1.25;text-align:center'; c.appendChild(t); return t; }
    var tip = null, pop = null, open = false;
    if (title) {
      tip = card('marker-tip'); tip.style.padding = '14px 36px'; tip.style.whiteSpace = 'nowrap'; addTitle(tip, title);
    }
    if (desc) {
      pop = card('marker-pop'); pop.id = 'marker-' + o.id.replace(/[^A-Za-z0-9_-]/g, '_');
      pop.setAttribute('role', 'region'); pop.setAttribute('aria-label', title || o.alt || 'Marker');
      pop.style.padding = '18px 28px'; pop.style.width = (m.card_w || 640) + 'px'; pop.style.height = (m.card_h || 340) + 'px';
      // The text scrolls inside the card; the card itself must not clip its pointer.
      var scroll = el('div', 'mk-scroll'); scroll.style.cssText = 'height:100%;overflow:auto'; pop.appendChild(scroll);
      if (title) addTitle(scroll, title).style.marginBottom = '14px';
      var body = el('div', 'mk-body', desc); body.style.cssText = 'font-size:32px;line-height:1.4;white-space:pre-wrap;text-align:left'; scroll.appendChild(body);
      node.setAttribute('aria-expanded', 'false'); node.setAttribute('aria-controls', pop.id);
    }
    // Cards grow out of the marker (see the `mk-in` animation in the page's CSS).
    function pop_in(c) {
      c.classList.remove('mk-show'); void c.offsetWidth; c.classList.add('mk-show');
      c.style.transformOrigin = (parseFloat(c._svg.style.left) + 20) + 'px ' + (c._svg.style.bottom ? '100%' : '0%');
    }
    function showTip() {
      if (!tip || open) return;
      tip.style.display = 'block';
      place(tip, tip.offsetWidth || 260, tip.offsetHeight || 80);
      pop_in(tip);
    }
    function hideTip() { if (tip) tip.style.display = 'none'; }
    function close(refocus) {
      if (!open) return;
      open = false; if (pop) pop.style.display = 'none'; node.setAttribute('aria-expanded', 'false');
      if (refocus) node.focus();
    }
    function openCard() {
      markerClosers.forEach(function (f) { if (f !== close) f(false); });
      hideTip(); open = true; pop.style.display = 'block';
      place(pop, m.card_w || 640, m.card_h || 340); pop_in(pop); node.setAttribute('aria-expanded', 'true');
    }
    node.addEventListener('mouseenter', showTip); node.addEventListener('mouseleave', hideTip);
    node.addEventListener('focus', showTip); node.addEventListener('blur', hideTip);
    node.addEventListener('click', function (ev) {
      if (!pop) return;
      ev.stopPropagation();
      if (open) close(false); else openCard();
    });
    document.addEventListener('click', function (ev) {
      if (open && !node.contains(ev.target) && !pop.contains(ev.target)) close(false);
    });
    document.addEventListener('keydown', function (ev) { if (ev.key === 'Escape' && open) close(true); });
    markerClosers.push(close);
    entry.closeMarker = close;
  }


  // ---- 360° images: a panorama the learner looks around by dragging, with annotations -------------
  // The picture is equirectangular. WebGL shows it (a fragment shader turns each pixel's direction into a place in the
  // picture; the same maths as brainbend_doc::pano). Where WebGL is missing, or the browser will not let it read the file
  // (a page opened straight from disk), the flat picture is shown instead, scrolling sideways.
  function buildPano(node, o, entry) {
    var cfg = o.kind.Pano, url = o.asset && ((window.__panoPix && window.__panoPix[o.asset]) || P.assetUrls[o.asset]), pins = cfg.pins || [], D = Math.PI / 180;
    var rm = !!(window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);
    var name = (o.alt && o.alt.trim()) || o.name || '360° image', zoomOk = cfg.zoom !== false;
    var view = { yaw: cfg.yaw || 0, pitch: cfg.pitch || 0, fov: cfg.fov || 90 };
    var spinning = !!cfg.auto_rotate && !rm, dragging = false, dirty = true, raf = 0, last = 0, tween = null, gl = null, prog = null, locs = {}, flat = false;
    var FONT = '"Open Sans", Montserrat, Arial, sans-serif';
    node.classList.add('media'); node.style.overflow = 'hidden'; node.style.background = '#1b1b1b';
    node.setAttribute('role', 'application'); node.setAttribute('aria-roledescription', '360° image');
    node.setAttribute('aria-label', name + '. Drag, or use the arrow keys, to look around' + (zoomOk ? '; plus and minus zoom' : '') + '.');
    node.tabIndex = 0;
    function clamp(v, a, b) { return Math.min(b, Math.max(a, v)); }
    function wrap(y) { y = ((y + 180) % 360 + 360) % 360 - 180; return y === -180 ? 180 : y; }
    function basis(v) {
      var y = v.yaw * D, p = v.pitch * D, f = [Math.cos(p) * Math.sin(y), Math.sin(p), Math.cos(p) * Math.cos(y)], r = [Math.cos(y), 0, -Math.sin(y)];
      return { f: f, r: r, u: [f[1] * r[2] - f[2] * r[1], f[2] * r[0] - f[0] * r[2], f[0] * r[1] - f[1] * r[0]] };
    }
    // Where a place appears in the view (-1..1 from the middle), or null when it is behind the viewer.
    function project(yaw, pitch, aspect) {
      var b = basis(view), a = yaw * D, e = pitch * D, d = [Math.cos(e) * Math.sin(a), Math.sin(e), Math.cos(e) * Math.cos(a)];
      var dot = function (m, n) { return m[0] * n[0] + m[1] * n[1] + m[2] * n[2]; }, depth = dot(d, b.f);
      if (depth <= 0.02) return null;
      var t = Math.tan(view.fov * D / 2);
      return { x: dot(d, b.r) / depth / t, y: dot(d, b.u) / depth / t * aspect };
    }
    function visible() { return node.isConnected && node.getClientRects().length > 0; }
    function schedule() { if (!raf) raf = requestAnimationFrame(frame); }
    function invalidate() { dirty = true; schedule(); }

    // The canvas and, over it, the controls and the annotations.
    var canvas = el('canvas'); canvas.setAttribute('aria-hidden', 'true');
    canvas.style.cssText = 'position:absolute;inset:0;width:100%;height:100%;touch-action:none;cursor:grab;display:block';
    node.appendChild(canvas);
    function btn(label, glyph, size) {
      var b = el('button'); b.type = 'button'; b.setAttribute('aria-label', label); b.title = label;
      var g = el('span', '', glyph); g.setAttribute('aria-hidden', 'true'); b.appendChild(g);
      b.style.cssText = 'width:' + size + 'px;height:' + size + 'px;border-radius:50%;border:3px solid #fff;background:rgba(0,0,0,.7);color:#fff;font:700 ' + Math.round(size * 0.5) + 'px/1 ' + FONT + ';cursor:pointer;padding:0;display:flex;align-items:center;justify-content:center';
      b.addEventListener('pointerdown', function (ev) { ev.stopPropagation(); });
      return b;
    }
    var bar = el('div', 'pano-controls');
    bar.style.cssText = 'position:absolute;right:16px;bottom:16px;display:flex;gap:10px;align-items:center;z-index:5';
    node.appendChild(bar);
    var SZ = 56, pauseBtn = null, listBtn = null, list = null, zin = null, zout = null;
    function stopSpin() { if (spinning) { spinning = false; syncPause(); } }
    function syncPause() { if (!pauseBtn) return; pauseBtn.setAttribute('aria-pressed', spinning ? 'false' : 'true'); pauseBtn.setAttribute('aria-label', spinning ? 'Pause turning' : 'Turn automatically'); pauseBtn.title = pauseBtn.getAttribute('aria-label'); pauseBtn.firstChild.textContent = spinning ? '❚❚' : '▶'; }
    if (cfg.auto_rotate) { pauseBtn = btn('Pause turning', '❚❚', SZ); pauseBtn.firstChild.style.fontSize = Math.round(SZ * 0.34) + 'px'; pauseBtn.addEventListener('click', function () { spinning = !spinning; syncPause(); if (spinning) { last = 0; schedule(); } }); bar.appendChild(pauseBtn); syncPause(); }
    function zoomBy(k) { view.fov = clamp(view.fov * k, 30, 110); invalidate(); }
    if (zoomOk) {
      zin = btn('Zoom in', '+', SZ); zout = btn('Zoom out', '−', SZ);
      zin.addEventListener('click', function () { stopSpin(); zoomBy(0.8); }); zout.addEventListener('click', function () { stopSpin(); zoomBy(1.25); });
      bar.appendChild(zout); bar.appendChild(zin);
    }
    var home = btn('Back to the starting view', '↺', SZ);
    home.addEventListener('click', function () { stopSpin(); lookAt(cfg.yaw || 0, cfg.pitch || 0, cfg.fov || 90); });
    bar.appendChild(home);

    // Looking somewhere: a short turn (at once when the learner prefers reduced motion).
    function lookAt(yaw, pitch, fov) {
      var dy = wrap(yaw - view.yaw);
      if (rm || flat) { view.yaw = wrap(view.yaw + dy); view.pitch = clamp(pitch, -85, 85); if (fov) view.fov = fov; invalidate(); return; }
      tween = { t0: 0, from: { yaw: view.yaw, pitch: view.pitch, fov: view.fov }, dy: dy, to: { pitch: clamp(pitch, -85, 85), fov: fov || view.fov } };
      schedule();
    }

    // Annotations: buttons over the picture while they are in view, and a list that reaches every one of them.
    var card = null, openPin = -1, pinEls = [], visited = [], dark = cfg.dark !== false, hsEls = [], polyEls = [], tourAt = -1;
    function closeCard(refocus) {
      if (!card) return;
      var was = openPin; card.style.display = 'none'; openPin = -1;
      pinEls.forEach(function (b) { b.setAttribute('aria-expanded', 'false'); });
      if (refocus && pinEls[was]) pinEls[was].focus();
    }
    function showCard(i) {
      var p = pins[i]; if (!p) return;
      if (!card) {
        card = el('div', 'pano-card'); card.setAttribute('role', 'region'); card.id = 'pano-card-' + o.id.replace(/[^A-Za-z0-9_-]/g, '_');
        card.style.cssText = 'position:absolute;z-index:8;display:none;box-sizing:border-box;background:' + (dark ? '#1b1b1b' : '#fff') + ';color:' + (dark ? '#fff' : '#111') + ';border:2px solid ' + (dark ? '#000' : '#4a4a4a') + ';border-radius:' + (cfg.rounded === false ? '0' : '14px') + ';padding:18px 26px;font-family:' + FONT + ';overflow:visible;cursor:default;text-align:left';
        card.addEventListener('pointerdown', function (ev) { ev.stopPropagation(); });
        node.appendChild(card);
      }
      card.setAttribute('aria-label', p.title || 'Annotation');
      card.textContent = '';
      var h = el('div', 'pano-card-title', p.title || ''); h.style.cssText = 'font-weight:700;font-size:36px;line-height:1.25;margin-bottom:10px'; card.appendChild(h);
      if (p.description) { var d = el('div', 'pano-card-body', p.description); d.style.cssText = 'font-size:30px;line-height:1.4;white-space:pre-wrap'; card.appendChild(d); }
      var x = el('button', '', 'Close'); x.type = 'button'; x.style.cssText = 'margin-top:14px;font:700 26px ' + FONT + ';padding:8px 22px;border-radius:8px;border:2px solid ' + (dark ? '#fff' : '#4a4a4a') + ';background:' + (dark ? '#333' : '#f1f1f1') + ';color:' + (dark ? '#fff' : '#111') + ';cursor:pointer';
      x.addEventListener('click', function () { closeCard(true); }); card.appendChild(x);
      var cw = Math.min(640, o.w - 32), at = { x: pinEls[i] ? parseFloat(pinEls[i].style.left) : NaN, y: pinEls[i] ? parseFloat(pinEls[i].style.top) : NaN };
      if (!isFinite(at.x) || !isFinite(at.y)) at = { x: o.w / 2, y: o.h / 2 };
      var pad = cfg.tour ? 104 : 8; // a guided tour keeps its buttons clear
      card.style.width = cw + 'px'; card.style.maxHeight = Math.max(160, o.h - 24 - pad) + 'px'; card.style.display = 'block';
      var ch = card.offsetHeight, above = at.y - 40 - ch >= 8;
      var cl = clamp(at.x - cw / 2, 16, Math.max(16, o.w - cw - 16));
      card.style.left = cl + 'px';
      card.style.top = clamp(above ? at.y - 40 - ch : at.y + 40, 8, Math.max(8, o.h - ch - pad)) + 'px';
      // The connector: a small pointer from the card to the item.
      if (cfg.connector !== false && p.kind !== 'Hotspot' || (cfg.connector !== false && p.kind === 'Hotspot')) {
        var tip = el('div'); tip.setAttribute('aria-hidden', 'true');
        tip.style.cssText = 'position:absolute;width:22px;height:22px;transform:rotate(45deg);box-sizing:border-box;background:' + (dark ? '#1b1b1b' : '#fff') + ';border:2px solid ' + (dark ? '#000' : '#4a4a4a') + ';left:' + (clamp(at.x - cl, 30, cw - 30) - 11) + 'px;' + (above ? 'bottom:-12px;border-left:0;border-top:0' : 'top:-12px;border-right:0;border-bottom:0');
        card.appendChild(tip);
      }
      openPin = i; pinEls.forEach(function (b, k) { b.setAttribute('aria-expanded', k === i ? 'true' : 'false'); });
      markerClosers.forEach(function (f) { if (f !== closeMine) f(false); });
    }
    function closeMine() { closeCard(false); }
    // Pressing an item runs the triggers that watch for it, then opens its card when it has something to say.
    function press(p) {
      if (depth > 20) return; // guard against trigger loops
      depth++;
      try {
        slides[entry.si].triggers.forEach(function (t) {
          var ev = t.event && t.event.PanoPin;
          if (ev && ev[0] === o.id && ev[1] === p.id) runTrigger(t);
        });
      } finally { depth--; }
    }
    function hasCard(p) { return !p.button && !!((p.title && p.title.trim()) || (p.description && p.description.trim())); }
    function rgb(p) { var c = p.color || (p.kind === 'Hotspot' ? [13, 218, 239, 255] : [79, 129, 189, 255]); return 'rgb(' + c[0] + ',' + c[1] + ',' + c[2] + ')'; }
    // Visiting an item: remembered for the progress count, and an unvisited one pulses until then.
    function visit(i) {
      if (!visited[i]) { visited[i] = true; syncProgress(); }
      if (pinEls[i]) pinEls[i].classList.remove('pano-pulse');
      if (polyEls[i]) polyEls[i].classList.remove('pano-pulse');
      paintHotspot(i);
    }
    function activate(i) {
      var p = pins[i]; if (!p) return;
      stopSpin(); visit(i); press(p);
      if (hasCard(p)) { if (openPin === i) closeCard(false); else showCard(i); } else closeCard(false);
    }
    var cssEl = document.getElementById('pano-css');
    if (!cssEl) {
      cssEl = el('style'); cssEl.id = 'pano-css';
      cssEl.textContent = '@keyframes pano-pulse{0%{box-shadow:0 0 0 0 rgba(255,255,255,.75)}100%{box-shadow:0 0 0 26px rgba(255,255,255,0)}}' +
        '@keyframes pano-glow{0%,100%{opacity:.35}50%{opacity:.8}}' +
        '.pano-pulse{animation:pano-pulse 1.8s ease-out infinite}polygon.pano-pulse{animation:pano-glow 1.8s ease-in-out infinite}' +
        '@media (prefers-reduced-motion:reduce){.pano-pulse,polygon.pano-pulse{animation:none}}' +
        '.pano-item:focus-visible{outline:4px solid #ffd60a;outline-offset:4px}';
      document.head.appendChild(cssEl);
    }
    var pulses = cfg.pulse !== false;
    // Hotspots are polygons over the picture, drawn in the object's own units.
    var hsSvg = null;
    if (pins.some(function (p) { return p.kind === 'Hotspot'; })) {
      var ns = 'http://www.w3.org/2000/svg'; hsSvg = document.createElementNS(ns, 'svg');
      hsSvg.setAttribute('viewBox', '0 0 ' + o.w + ' ' + o.h); hsSvg.setAttribute('width', '100%'); hsSvg.setAttribute('height', '100%'); hsSvg.setAttribute('aria-hidden', 'true');
      hsSvg.style.cssText = 'position:absolute;inset:0;z-index:3;pointer-events:none;overflow:hidden';
      node.appendChild(hsSvg);
    }
    function paintHotspot(i) {
      var poly = polyEls[i], p = pins[i]; if (!poly) return;
      var shown = cfg.reveal !== false || visited[i] || poly._hover || tourAt === i;
      poly.setAttribute('fill', rgb(p)); poly.setAttribute('fill-opacity', shown ? (poly._hover ? '0.55' : '0.36') : '0.02');
      poly.setAttribute('stroke', rgb(p)); poly.setAttribute('stroke-opacity', shown ? '1' : '0'); poly.setAttribute('stroke-width', '4');
    }
    pins.forEach(function (p, i) {
      var b = el('button', 'pano-item'); b.type = 'button'; b.setAttribute('aria-label', p.title || ((p.kind === 'Hotspot' ? 'Hotspot ' : 'Marker ') + (i + 1)));
      if (hasCard(p)) { b.setAttribute('aria-haspopup', 'true'); b.setAttribute('aria-expanded', 'false'); }
      b.title = p.title || '';
      if (p.kind === 'Hotspot') {
        // A keyboard stop at the middle of the area; the shaded area itself takes the pointer.
        b.style.cssText = 'position:absolute;display:none;width:44px;height:44px;margin:-22px 0 0 -22px;border-radius:50%;border:0;background:transparent;padding:0;z-index:4;cursor:pointer';
        var poly = document.createElementNS('http://www.w3.org/2000/svg', 'polygon');
        poly.style.cssText = 'pointer-events:all;cursor:pointer'; poly.style.display = 'none';
        poly.addEventListener('pointerdown', function (ev) { ev.stopPropagation(); });
        poly.addEventListener('click', function (ev) { ev.stopPropagation(); activate(i); });
        poly.addEventListener('pointerenter', function () { poly._hover = true; paintHotspot(i); });
        poly.addEventListener('pointerleave', function () { poly._hover = false; paintHotspot(i); });
        b.addEventListener('focus', function () { poly._hover = true; paintHotspot(i); }); b.addEventListener('blur', function () { poly._hover = false; paintHotspot(i); });
        if (pulses && !visited[i]) poly.classList.add('pano-pulse');
        hsSvg.appendChild(poly); polyEls[i] = poly; paintHotspot(i);
      } else {
        b.style.cssText = 'position:absolute;display:none;width:52px;height:52px;margin:-26px 0 0 -26px;border-radius:50%;border:3px solid #fff;background:' + rgb(p) + ';color:#fff;padding:0;z-index:4;cursor:pointer;box-shadow:0 2px 8px rgba(0,0,0,.5);align-items:center;justify-content:center';
        if (p.icon_path && p.icon_path.length) {
          var ns2 = 'http://www.w3.org/2000/svg', sv = document.createElementNS(ns2, 'svg'), pa = document.createElementNS(ns2, 'path');
          sv.setAttribute('viewBox', '0 0 1 1'); sv.setAttribute('width', '34'); sv.setAttribute('height', '34'); sv.setAttribute('aria-hidden', 'true');
          pa.setAttribute('fill', '#fff'); pa.setAttribute('fill-rule', 'evenodd');
          pa.setAttribute('d', p.icon_path.map(function (c) { return c === 'Close' ? 'Z' : c.Move ? 'M' + c.Move.join(' ') : c.Line ? 'L' + c.Line.join(' ') : 'C' + c.Curve.join(' '); }).join(''));
          sv.appendChild(pa); b.appendChild(sv);
        }
        if (pulses && !visited[i]) b.classList.add('pano-pulse');
      }
      b.addEventListener('pointerdown', function (ev) { ev.stopPropagation(); });
      b.addEventListener('click', function (ev) { ev.stopPropagation(); activate(i); });
      node.appendChild(b); pinEls.push(b);
    });
    // How many items the learner has visited.
    var progEl = null;
    function syncProgress() {
      if (!progEl) return;
      var n = visited.filter(Boolean).length;
      progEl.textContent = cfg.tour ? ('Step ' + (tourAt + 1) + ' of ' + pins.length) : ('Visited ' + n + ' of ' + pins.length);
    }
    if (pins.length && cfg.progress !== false) {
      progEl = el('div', 'pano-progress'); progEl.setAttribute('role', 'status');
      progEl.style.cssText = 'position:absolute;left:16px;top:16px;z-index:9;padding:8px 16px;border-radius:999px;background:rgba(0,0,0,.7);color:#fff;font:700 24px ' + FONT + ';pointer-events:none';
      node.appendChild(progEl); syncProgress();
    }
    if (pins.length) {
      listBtn = btn('Markers and hotspots', '☰', SZ); listBtn.setAttribute('aria-haspopup', 'true'); listBtn.setAttribute('aria-expanded', 'false');
      list = el('ul', 'pano-list'); list.style.cssText = 'display:none;position:absolute;right:16px;bottom:86px;z-index:9;margin:0;padding:8px;list-style:none;background:#fff;color:#111;border:2px solid #4a4a4a;border-radius:12px;font:600 28px ' + FONT + ';max-width:' + Math.round(o.w * 0.7) + 'px;max-height:' + Math.round(o.h - 130) + 'px;overflow:auto';
      pins.forEach(function (p, i) {
        var li = el('li'), b = el('button', '', (i + 1) + '  ' + (p.title || ((p.kind === 'Hotspot' ? 'Hotspot ' : 'Marker ') + (i + 1)))); b.type = 'button';
        b.style.cssText = 'display:block;width:100%;text-align:left;font:inherit;padding:8px 14px;border:0;background:transparent;color:inherit;cursor:pointer;border-radius:8px';
        b.addEventListener('click', function () {
          stopSpin(); toggleList(false);
          if (cfg.tour) { tourStep(i); return; }
          lookAt(p.yaw, p.pitch);
          // Once the view has turned, the item is on screen: use it from there.
          var tries = 0; (function wait() { if (tween && tries++ < 60) return requestAnimationFrame(wait); activate(i); if (hasCard(p) && pinEls[i] && pinEls[i].style.display !== 'none') pinEls[i].focus(); })();
        });
        li.appendChild(b); list.appendChild(li);
      });
      function toggleList(on) { list.style.display = on ? 'block' : 'none'; listBtn.setAttribute('aria-expanded', on ? 'true' : 'false'); if (on) list.querySelector('button').focus(); }
      listBtn.addEventListener('click', function () { toggleList(list.style.display === 'none'); });
      list.addEventListener('keydown', function (ev) { if (ev.key === 'Escape') { ev.stopPropagation(); toggleList(false); listBtn.focus(); } });
      list.addEventListener('pointerdown', function (ev) { ev.stopPropagation(); });
      bar.appendChild(listBtn); node.appendChild(list);
    }
    markerClosers.push(closeMine);
    document.addEventListener('click', function (ev) { if (openPin >= 0 && card && !card.contains(ev.target)) closeCard(false); });
    node.addEventListener('keydown', function (ev) {
      if (ev.key === 'Escape' && openPin >= 0) { ev.stopPropagation(); closeCard(true); return; }
      if (ev.target !== node || flat) return;
      var step = view.fov * 0.08, used = true;
      if (ev.key === 'ArrowLeft') view.yaw = wrap(view.yaw - step);
      else if (ev.key === 'ArrowRight') view.yaw = wrap(view.yaw + step);
      else if (ev.key === 'ArrowUp') view.pitch = clamp(view.pitch + step, -85, 85);
      else if (ev.key === 'ArrowDown') view.pitch = clamp(view.pitch - step, -85, 85);
      else if (zoomOk && (ev.key === '+' || ev.key === '=')) view.fov = clamp(view.fov * 0.9, 30, 110);
      else if (zoomOk && (ev.key === '-' || ev.key === '_')) view.fov = clamp(view.fov * 1.1, 30, 110);
      else if (ev.key === 'Home') { view.yaw = cfg.yaw || 0; view.pitch = cfg.pitch || 0; view.fov = cfg.fov || 90; }
      else used = false;
      if (used) { ev.preventDefault(); stopSpin(); invalidate(); }
    });

    // Dragging (and pinching) to look around.
    var pointers = {}, pinch0 = 0;
    canvas.addEventListener('pointerdown', function (ev) {
      pointers[ev.pointerId] = { x: ev.clientX, y: ev.clientY }; try { canvas.setPointerCapture(ev.pointerId); } catch (e) { /* not capturable */ }
      dragging = true; canvas.style.cursor = 'grabbing'; stopSpin(); tween = null; node.focus({ preventScroll: true });
      var ids = Object.keys(pointers); if (ids.length === 2) { var a = pointers[ids[0]], b = pointers[ids[1]]; pinch0 = Math.hypot(a.x - b.x, a.y - b.y); }
    });
    canvas.addEventListener('pointermove', function (ev) {
      var pt = pointers[ev.pointerId]; if (!pt) return;
      var ids = Object.keys(pointers), rect = node.getBoundingClientRect();
      if (ids.length >= 2 && zoomOk) {
        pt.x = ev.clientX; pt.y = ev.clientY;
        var a = pointers[ids[0]], b = pointers[ids[1]], dist = Math.hypot(a.x - b.x, a.y - b.y);
        if (pinch0 > 0 && dist > 0) { view.fov = clamp(view.fov * pinch0 / dist, 30, 110); pinch0 = dist; invalidate(); }
        return;
      }
      var k = view.fov / Math.max(1, rect.width);
      view.yaw = wrap(view.yaw - (ev.clientX - pt.x) * k); view.pitch = clamp(view.pitch + (ev.clientY - pt.y) * k, -85, 85);
      pt.x = ev.clientX; pt.y = ev.clientY; invalidate();
    });
    function release(ev) { delete pointers[ev.pointerId]; pinch0 = 0; if (!Object.keys(pointers).length) { dragging = false; canvas.style.cursor = 'grab'; last = 0; schedule(); } }
    canvas.addEventListener('pointerup', release); canvas.addEventListener('pointercancel', release);
    if (zoomOk) canvas.addEventListener('wheel', function (ev) { ev.preventDefault(); stopSpin(); zoomBy(Math.exp(ev.deltaY * 0.0015)); }, { passive: false });

    // WebGL.
    var VS = 'attribute vec2 a;varying vec2 p;void main(){p=a;gl_Position=vec4(a,0.,1.);}';
    var FS = 'precision highp float;uniform sampler2D tex;uniform vec3 f,r,u;uniform float t,aspect;varying vec2 p;' +
      'void main(){vec3 d=normalize(f+r*(p.x*t)+u*(p.y*t/aspect));float lon=atan(d.x,d.z),lat=asin(clamp(d.y,-1.,1.));' +
      'gl_FragColor=texture2D(tex,vec2(.5+lon/6.2831853,.5-lat/3.1415927));}';
    function shader(type, src) { var s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s); if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error('shader'); return s; }
    function fallBack() {
      if (flat) return; flat = true; spinning = false; if (pauseBtn) pauseBtn.style.display = 'none';
      canvas.style.display = 'none'; home.style.display = 'none';
      if (zin) zin.style.display = 'none'; if (zout) zout.style.display = 'none';
      node.setAttribute('role', 'group'); node.removeAttribute('aria-roledescription'); node.tabIndex = -1;
      if (url) {
        var sc = el('div', 'pano-flat'); sc.tabIndex = 0; sc.setAttribute('role', 'region'); sc.setAttribute('aria-label', name + ' (scrolls sideways)');
        sc.style.cssText = 'position:absolute;inset:0;overflow-x:auto;overflow-y:hidden';
        var im = el('img'); im.src = url; im.alt = ''; im.draggable = false; im.style.cssText = 'position:static;height:100%;width:auto;max-width:none;display:block';
        sc.appendChild(im); node.insertBefore(sc, node.firstChild);
      }
    }
    try {
      gl = canvas.getContext('webgl', { antialias: false, alpha: false });
      if (!gl || !url) throw new Error('no gl');
      prog = gl.createProgram(); gl.attachShader(prog, shader(gl.VERTEX_SHADER, VS)); gl.attachShader(prog, shader(gl.FRAGMENT_SHADER, FS)); gl.linkProgram(prog);
      if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error('link');
      gl.useProgram(prog);
      var buf = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, buf); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
      var al = gl.getAttribLocation(prog, 'a'); gl.enableVertexAttribArray(al); gl.vertexAttribPointer(al, 2, gl.FLOAT, false, 0, 0);
      ['f', 'r', 'u', 't', 'aspect', 'tex'].forEach(function (n) { locs[n] = gl.getUniformLocation(prog, n); });
      var tex = gl.createTexture(), img = new Image();
      img.onload = function () {
        try {
          var src = img, max = gl.getParameter(gl.MAX_TEXTURE_SIZE);
          if (img.naturalWidth > max || img.naturalHeight > max) {
            var k = max / Math.max(img.naturalWidth, img.naturalHeight), c = document.createElement('canvas');
            c.width = Math.floor(img.naturalWidth * k); c.height = Math.floor(img.naturalHeight * k); c.getContext('2d').drawImage(img, 0, 0, c.width, c.height); src = c;
          }
          gl.bindTexture(gl.TEXTURE_2D, tex);
          gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, src);
          gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
          gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
          node._panoReady = true; invalidate();
        } catch (e) { fallBack(); }
      };
      img.onerror = function () { fallBack(); };
      img.src = url;
    } catch (e) { fallBack(); }

    function frame(ts) {
      raf = 0;
      if (!visible()) { last = 0; return; }
      var dt = last ? Math.min(0.1, (ts - last) / 1000) : 0; last = ts;
      if (tween) {
        if (!tween.t0) tween.t0 = ts;
        var k = clamp((ts - tween.t0) / 450, 0, 1), e = k < 0.5 ? 2 * k * k : 1 - Math.pow(-2 * k + 2, 2) / 2;
        view.yaw = wrap(tween.from.yaw + tween.dy * e); view.pitch = tween.from.pitch + (tween.to.pitch - tween.from.pitch) * e; view.fov = tween.from.fov + (tween.to.fov - tween.from.fov) * e;
        dirty = true; if (k >= 1) tween = null;
      }
      if (spinning && !dragging && !tween) { view.yaw = wrap(view.yaw + (cfg.speed || 6) * dt); dirty = true; }
      if (dirty && !flat) draw();
      dirty = false;
      if (spinning || tween) schedule();
    }
    function draw() {
      var rect = node.getBoundingClientRect(), dpr = Math.min(2, window.devicePixelRatio || 1);
      var w = Math.max(2, Math.round(rect.width * dpr)), h = Math.max(2, Math.round(rect.height * dpr));
      if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; gl.viewport(0, 0, w, h); }
      if (node._panoReady) {
        var b = basis(view);
        gl.uniform3fv(locs.f, b.f); gl.uniform3fv(locs.r, b.r); gl.uniform3fv(locs.u, b.u);
        gl.uniform1f(locs.t, Math.tan(view.fov * D / 2)); gl.uniform1f(locs.aspect, w / h); gl.uniform1i(locs.tex, 0);
        gl.drawArrays(gl.TRIANGLES, 0, 3);
      }
      var aspect = o.w / o.h;
      pins.forEach(function (p, i) {
        var away = cfg.tour && tourAt !== i, q = !away && project(p.yaw, p.pitch, aspect), on = q && Math.abs(q.x) <= 1.02 && Math.abs(q.y) <= 1.02;
        if (p.kind === 'Hotspot') {
          var pts = away ? [] : (p.points || []).map(function (c) { return project(c[0], c[1], aspect); }), ok = pts.length >= 3 && pts.every(Boolean), poly = polyEls[i];
          if (ok) poly.setAttribute('points', pts.map(function (c) { return ((c.x * 0.5 + 0.5) * o.w).toFixed(1) + ',' + ((0.5 - c.y * 0.5) * o.h).toFixed(1); }).join(' '));
          poly.style.display = ok ? '' : 'none';
        }
        pinEls[i].style.display = on ? 'flex' : 'none';
        if (on) { pinEls[i].style.left = ((q.x * 0.5 + 0.5) * o.w) + 'px'; pinEls[i].style.top = ((0.5 - q.y * 0.5) * o.h) + 'px'; }
        else if (openPin === i && !(p.kind === 'Hotspot' && polyEls[i] && polyEls[i].style.display !== 'none')) closeCard(false);
      });
    }
    if (pins.length && flat) { pinEls.forEach(function (b) { b.style.display = 'none'; }); polyEls.forEach(function (q) { if (q) q.style.display = 'none'; }); }
    // A guided tour: one item at a time, in the order of the list, with Previous and Next.
    var tourPrev = null, tourNext = null, hintEl = null;
    function tourStep(i) {
      i = clamp(i, 0, pins.length - 1);
      var p = pins[i]; tourAt = i; stopSpin(); closeCard(false); visit(i); syncProgress();
      tourPrev.disabled = i === 0; tourNext.disabled = i === pins.length - 1;
      tourPrev.style.opacity = tourPrev.disabled ? '.4' : '1'; tourNext.style.opacity = tourNext.disabled ? '.4' : '1';
      hintEl.textContent = p.kind === 'Hotspot' && cfg.reveal === false && p.hint ? 'Hint: ' + p.hint : '';
      hintEl.style.display = hintEl.textContent ? 'block' : 'none';
      lookAt(p.yaw, p.pitch); invalidate();
      var tries = 0; (function wait() { if (tween && tries++ < 60) return requestAnimationFrame(wait); if (tourAt === i && hasCard(p)) showCard(i); })();
    }
    if (cfg.tour && pins.length) {
      var tb = el('div', 'pano-tour'); tb.setAttribute('role', 'group'); tb.setAttribute('aria-label', 'Guided tour');
      tb.style.cssText = 'position:absolute;left:16px;bottom:16px;z-index:10;display:flex;gap:12px;align-items:center;max-width:42%';
      tb.addEventListener('pointerdown', function (ev) { ev.stopPropagation(); });
      tourPrev = btn('Previous', '‹', SZ); tourNext = btn('Next', '›', SZ);
      tourPrev.title = 'Previous'; tourNext.title = 'Next';
      tourPrev.addEventListener('click', function () { tourStep(tourAt - 1); }); tourNext.addEventListener('click', function () { tourStep(tourAt + 1); });
      hintEl = el('div', 'pano-hint'); hintEl.style.cssText = 'display:none;padding:8px 16px;border-radius:12px;background:rgba(0,0,0,.75);color:#fff;font:600 24px ' + FONT;
      tb.appendChild(tourPrev); tb.appendChild(hintEl); tb.appendChild(tourNext); node.appendChild(tb);
      tourStep(0);
    }
    window.addEventListener('resize', invalidate);
    if (window.IntersectionObserver) new IntersectionObserver(function (es) { if (es.some(function (e) { return e.isIntersecting; })) { last = 0; invalidate(); } }).observe(node);
    // Seen again after another slide: draw at once.
    node.addEventListener('panoshow', invalidate);
    schedule();
  }

  // States that make an object behave as a button (a picture's own states, like Car-01, do not).
  function buttonLike(o) {
    return o.states.some(function (s) { return /^(Hover|Down|Disabled|Visited|Selected)$/.test(s.name); });
  }

  // ---- sliders and dials: a number variable and a control that sets it and shows it -------------
  function rangeFraction(min, max, v) { return max <= min ? 0 : Math.min(1, Math.max(0, (v - min) / (max - min))); }
  function snapValue(min, max, step, v) {
    if (step > 0) v = min + Math.round((v - min) / step) * step;
    v = Math.min(Math.max(v, Math.min(min, max)), Math.max(min, max));
    return Math.round(v * 1e6) / 1e6;
  }
  function numVar(id, fallback) { var v = vars[id]; return typeof v === 'number' && isFinite(v) ? v : fallback; }
  // Pointer drags and the arrow keys move a control; `get` and `put` read and write its value.
  function wireControl(node, o, min, max, step, get, put, fromPointer) {
    node.setAttribute('role', 'slider'); node.tabIndex = 0;
    node.setAttribute('aria-valuemin', String(min)); node.setAttribute('aria-valuemax', String(max));
    node.setAttribute('aria-label', o.alt || o.name || 'Slider');
    node.style.touchAction = 'none';
    var dragging = false;
    node.addEventListener('pointerdown', function (ev) {
      if (ev.pointerType === 'mouse' && ev.button) return;
      dragging = true;
      try { node.setPointerCapture(ev.pointerId); } catch (e) { /* not every browser lets us */ }
      fromPointer(ev); ev.preventDefault();
    });
    node.addEventListener('pointermove', function (ev) { if (dragging) fromPointer(ev); });
    var stop = function () { dragging = false; };
    node.addEventListener('pointerup', stop); node.addEventListener('pointercancel', stop);
    node.addEventListener('keydown', function (ev) {
      var by = step > 0 ? step : (max - min) / 10, v = get();
      var next = ev.key === 'ArrowRight' || ev.key === 'ArrowUp' ? v + by : ev.key === 'ArrowLeft' || ev.key === 'ArrowDown' ? v - by : ev.key === 'Home' ? min : ev.key === 'End' ? max : null;
      if (next === null) return;
      ev.preventDefault(); put(snapValue(min, max, step, next));
    });
  }
  function buildSlider(entry, sp) {
    var node = entry.node, o = entry.o, local = sp.min;
    entry.cursor = 'pointer';
    if (sp.track) {
      var track = el('div'); track.setAttribute('aria-hidden', 'true');
      track.style.cssText = 'position:absolute;left:0;right:0;top:50%;height:4px;margin-top:-2px;border-radius:2px;background:' + css(sp.track);
      node.appendChild(track);
    }
    var th = Math.max(4, o.h * sp.thumb_height), url = sp.thumb ? P.assetUrls[sp.thumb] : '', thumb;
    if (url) {
      thumb = el('img'); thumb.src = url; thumb.alt = ''; thumb.draggable = false;
      thumb.style.cssText = 'position:absolute;max-width:none;width:auto;pointer-events:none;height:' + th + 'px;top:' + ((o.h - th) / 2) + 'px';
      thumb.addEventListener('load', place);
    } else {
      thumb = el('div');
      thumb.style.cssText = 'position:absolute;pointer-events:none;border-radius:50%;box-shadow:0 1px 4px rgba(0,0,0,.4);width:' + th + 'px;height:' + th + 'px;top:' + ((o.h - th) / 2) + 'px;background:' + css(sp.thumb_color);
    }
    thumb.setAttribute('aria-hidden', 'true');
    node.appendChild(thumb);
    function get() { return sp.variable ? numVar(sp.variable, sp.min) : local; }
    function thumbW() { return url && thumb.naturalWidth && thumb.naturalHeight ? th * thumb.naturalWidth / thumb.naturalHeight : th; }
    function place() {
      var v = get();
      thumb.style.left = (rangeFraction(sp.min, sp.max, v) * Math.max(0, o.w - thumbW())) + 'px';
      node.setAttribute('aria-valuenow', String(v));
    }
    function put(v) { if (sp.variable && vars[sp.variable] !== undefined) setVar(sp.variable, v); else local = v; place(); }
    wireControl(node, o, sp.min, sp.max, sp.step, get, put, function (ev) {
      var r = node.getBoundingClientRect(), x = (ev.clientX - r.left) / (r.width || 1) * o.w, span = Math.max(1, o.w - thumbW());
      put(snapValue(sp.min, sp.max, sp.step, sp.min + (x - thumbW() / 2) / span * (sp.max - sp.min)));
    });
    controlUpdaters.push(place); place();
  }
  // A data-entry field: what is typed is kept in the field's variable, and a variable changed by a trigger shows in it.
  function buildInput(entry, f) {
    var node = entry.node, o = entry.o, st = o.style || {};
    node.classList.add('media');   // the (empty) label must not sit over the field
    var inp = el('input');
    inp.type = f.numeric ? 'number' : 'text';
    inp.setAttribute('aria-label', o.alt || o.name || (f.numeric ? 'Number entry' : 'Text entry'));
    inp.autocomplete = 'off';
    if (f.numeric) { inp.min = String(f.min); inp.max = String(f.max); inp.step = f.step > 0 ? String(f.step) : 'any'; }
    else if (f.placeholder) inp.placeholder = f.placeholder;
    inp.style.cssText = 'display:block;box-sizing:border-box;width:100%;height:100%;margin:0;padding:0 14px;border:0;outline-offset:-3px;background:transparent;color:inherit;font:inherit';
    inp.style.fontFamily = '"' + (st.font_family || 'Open Sans') + '", "Open Sans", Montserrat, Arial, sans-serif';
    inp.style.fontSize = (st.size || 36) + 'px';
    inp.style.color = css(st.color || [30, 30, 30, 255]);
    node.appendChild(inp);
    function shown() { var v = f.variable && vars[f.variable] !== undefined ? vars[f.variable] : ''; return v === '' || v === undefined || v === null ? '' : String(v); }
    function sync() { var v = shown(); if (document.activeElement !== inp && inp.value !== v) inp.value = v; }
    inp.addEventListener('input', function () {
      if (!f.variable || vars[f.variable] === undefined) return;
      if (f.numeric) { var n = parseFloat(inp.value); if (isFinite(n)) setVar(f.variable, n); }
      else setVar(f.variable, inp.value);
    });
    controlUpdaters.push(sync);
    inp.value = shown();
  }
  function buildDial(entry, d) {
    var node = entry.node, o = entry.o, local = d.min;
    entry.cursor = 'grab';
    node.style.transformOrigin = (o.w / 2 + d.pivot[0]) + 'px ' + (o.h / 2 + d.pivot[1]) + 'px';
    function get() { return d.variable && vars[d.variable] !== undefined ? numVar(d.variable, d.min) : local; }
    function place() {
      var v = get();
      node.style.rotate = ((o.rotation || 0) + rangeFraction(d.min, d.max, v) * d.sweep - d.sweep / 2) + 'deg';
      node.setAttribute('aria-valuenow', String(v));
    }
    function put(v) { if (d.variable && vars[d.variable] !== undefined) setVar(d.variable, v); else local = v; place(); }
    function valueAt(a) {
      var half = d.sweep / 2;
      a = d.sweep >= 360 ? ((a + 180) % 360 + 360) % 360 - 180 : Math.min(half, Math.max(-half, a));
      return snapValue(d.min, d.max, d.step, d.min + (a + half) / Math.max(1, d.sweep) * (d.max - d.min));
    }
    wireControl(node, o, d.min, d.max, d.step, get, put, function (ev) {
      // The turn is the pointer's direction from the pivot, measured clockwise from straight up.
      var layer = node.parentNode, lr = layer.getBoundingClientRect(), k = lr.width / (layer.offsetWidth || lr.width || 1);
      var cx = lr.left + (o.x + o.w / 2 + d.pivot[0]) * k, cy = lr.top + (o.y + o.h / 2 + d.pivot[1]) * k;
      if (Math.abs(ev.clientX - cx) + Math.abs(ev.clientY - cy) < 4) return;
      var theta = Math.atan2(ev.clientX - cx, -(ev.clientY - cy)) * 180 / Math.PI;
      put(valueAt(((theta - (o.rotation || 0) + 180) % 360 + 360) % 360 - 180));
    });
    controlUpdaters.push(place); place();
  }

  // ---- check boxes: clicking moves the object between Normal and Selected ----------------------
  function buildCheck(entry, ck) {
    var node = entry.node, o = entry.o, ns = 'http://www.w3.org/2000/svg';
    var side = Math.max(1, Math.min(ck.size, o.h)), left = ck.size * 0.3, top = (o.h - side) / 2;
    var box = el('div'); box.setAttribute('aria-hidden', 'true');
    box.style.cssText = 'position:absolute;box-sizing:border-box;border-radius:' + (ck.radio ? '50%' : '3px') + ';pointer-events:none;left:' + left + 'px;top:' + top + 'px;width:' + side + 'px;height:' + side + 'px;background:' + css(ck.box_fill) + ';border:' + (ck.box_line ? '1px solid ' + css(ck.box_line) : 'none');
    var svg = document.createElementNS(ns, 'svg'), tick = document.createElementNS(ns, 'path');
    svg.setAttribute('viewBox', '0 0 20 20'); svg.style.cssText = 'position:absolute;inset:0;width:100%;height:100%';
    if (ck.radio) {
      // A radio button: a dot in the middle where a check box has its tick.
      var dot = document.createElementNS(ns, 'circle');
      dot.setAttribute('cx', '10'); dot.setAttribute('cy', '10'); dot.setAttribute('r', '5'); dot.setAttribute('fill', css(ck.tick));
      svg.appendChild(dot);
    } else {
      tick.setAttribute('d', 'M4.5 10.5l3.5 3.5 7.5-8'); tick.setAttribute('fill', 'none'); tick.setAttribute('stroke', css(ck.tick));
      tick.setAttribute('stroke-width', '2.2'); tick.setAttribute('stroke-linecap', 'round'); tick.setAttribute('stroke-linejoin', 'round');
      svg.appendChild(tick);
    }
    box.appendChild(svg); node.appendChild(box);
    node.setAttribute('role', ck.radio ? 'radio' : 'checkbox'); node.tabIndex = 0;
    if (o.alt) node.setAttribute('aria-label', o.alt);
    node.style.cursor = 'pointer';
    entry.clickable = true;
    entry.drawCheck = function () {
      var on = entry.state === 'Selected', off = entry.state === 'Disabled';
      svg.style.display = on ? '' : 'none';
      box.style.opacity = off ? '0.5' : '1';
      node.style.cursor = off ? 'default' : 'pointer';
      node.setAttribute('aria-checked', on ? 'true' : 'false');
      if (off) node.setAttribute('aria-disabled', 'true'); else node.removeAttribute('aria-disabled');
    };
    node.addEventListener('click', function () {
      if (entry.state === 'Disabled') return;
      if (!ck.radio) { setObjectState(entry, entry.state === 'Selected' ? 'Normal' : 'Selected'); return; }
      // Choosing a radio button clears the others of its group on the slide; clicking the chosen one changes nothing.
      Object.keys(objects).forEach(function (id) {
        var e = objects[id];
        if (e !== entry && e.si === entry.si && e.o.check && e.o.check.radio === ck.radio && e.state === 'Selected') setObjectState(e, 'Normal');
      });
      setObjectState(entry, 'Selected');
    });
    node.addEventListener('keydown', function (ev) { if (ev.key === ' ' || ev.key === 'Enter') { ev.preventDefault(); node.click(); } });
    entry.drawCheck();
  }
  // Put an object in a state, and run the triggers that watch for it.
  function setObjectState(entry, name) {
    if (entry.state === name) return;
    entry.state = name; draw(entry);
    if (depth > 20) return; // guard against trigger loops
    depth++;
    try {
      slides[entry.si].triggers.forEach(function (t) {
        var ev = t.event && t.event.StateChanged;
        if (ev && ev[0] === entry.o.id && (ev[1] === '' || ev[1] === name)) runTrigger(t);
      });
    } finally { depth--; }
  }

  // A button set (a row of tabs): pressing one selects it and puts the others of the set back to where they started.
  function selectInSet(entry) {
    var o = entry.o;
    if (!o.button_set || !has(o, 'Selected')) return;
    Object.keys(objects).forEach(function (id) {
      var e = objects[id];
      if (e === entry || e.si !== entry.si || e.o.button_set !== o.button_set || e.state !== 'Selected') return;
      setObjectState(e, e.o.initial_state || 'Normal');
    });
    setObjectState(entry, 'Selected');
  }

  function stateFor(entry) {
    var o = entry.o;
    if (!o.states.length) return null;
    var name = entry.state;
    if (name === 'Normal' || name === o.initial_state || name === 'Visited') {
      if (entry.down && has(o, 'Down')) name = 'Down';
      else if (entry.hover && has(o, 'Hover')) name = 'Hover';
    }
    return o.states.find(function (s) { return s.name === name; }) || null;
  }
  // %Name% in text shows a variable's current value; Quiz.* shows live quiz numbers.
  // The slide whose text is being drawn, for %Slide.Number%.
  var substSlide = -1;
  function subst(t) {
    return t.replace(/%([A-Za-z0-9_.]+)%/g, function (m, n) {
      if (n === 'Slide.Number') return substSlide >= 0 && posOf[substSlide] >= 0 ? String(posOf[substSlide] + 1) : m;
      if (n === 'Slide.Total') return String(seq.length);
      if (n === 'Quiz.ScorePercent') return String(totals().pct);
      if (n === 'Quiz.PointsEarned') return String(totals().earned);
      if (n === 'Quiz.PointsPossible') return String(totals().max);
      var v = vars[byName[n]];
      if (v === undefined) return m;
      return typeof v === 'boolean' ? (v ? 'True' : 'False') : String(v);
    });
  }
  // An outline is solid, dotted or dashed.
  function dashCss(d) { return d === 'Dotted' ? 'dotted' : d === 'Dashed' ? 'dashed' : 'solid'; }
  function dashArray(d, w) { w = Math.max(1, w); return d === 'Dotted' ? w + ' ' + w * 1.5 : d === 'Dashed' ? w * 4 + ' ' + w * 2 : ''; }
  function has(o, n) { return o.states.some(function (s) { return s.name === n; }); }
  function draw(entry) {
    substSlide = entry.si;
    // The built-in Hidden state takes an object off the slide until another state is set.
    if (entry.state === 'Hidden') { entry.node.style.display = 'none'; entry.hiddenByState = true; }
    else if (entry.hiddenByState) { entry.node.style.display = ''; entry.hiddenByState = false; }
    if (entry.drawCheck) entry.drawCheck();
    var o = entry.o, s = stateFor(entry);
    var fill = s ? s.fill : o.fill, text = s ? s.text : o.text, st = s ? s.style : o.style, rich = s ? s.rich : o.rich;
    var stroke = s ? s.stroke : o.stroke, shadow = s ? s.shadow : o.shadow, grad = s ? s.gradient : o.gradient;
    if (entry.node._path) {
      entry.node.style.background = 'none';
      entry.node._path.setAttribute('fill', (!s && entry.node._gradId) ? 'url(#' + entry.node._gradId + ')' : fill ? css(fill) : 'none');
      entry.node._path.setAttribute('stroke', stroke ? css(stroke.color) : 'none');
      entry.node._path.setAttribute('stroke-width', stroke ? stroke.width : 0);
      entry.node._path.setAttribute('stroke-dasharray', stroke ? dashArray(o.dash, stroke.width) : '');
      stroke = null;
    } else if (s && s.shine && (fill || grad)) {
      // A button's "shine": a light gradient over the fill, stronger on hover.
      var sh = s.shine;
      entry.node.style.background = 'linear-gradient(' + (sh.reverse ? '0deg' : '180deg') + ',rgba(255,255,255,' + sh.alpha + '),rgba(255,255,255,' + (sh.alpha * 0.1) + ')),' + (grad ? gradCss(grad) : css(fill));
    } else entry.node.style.background = grad ? gradCss(grad) : css(fill);
    if (entry.node._line) {
      entry.node.style.border = 'none';
      entry.node._line.setAttribute('stroke', stroke ? css(stroke.color) : 'none');
      entry.node._line.setAttribute('stroke-width', stroke ? stroke.width : 0);
      entry.node._line.setAttribute('stroke-dasharray', stroke ? dashArray(o.dash, stroke.width) : '');
    } else if (o.table_borders && !s) {
      var sides = ['Left', 'Right', 'Top', 'Bottom'];
      entry.node.style.border = 'none';
      sides.forEach(function (n, i) {
        var b = o.table_borders[i];
        entry.node.style['border' + n] = b ? b.width + 'px solid ' + css(b.color) : 'none';
      });
    } else entry.node.style.border = stroke ? stroke.width + 'px ' + dashCss(o.dash) + ' ' + css(stroke.color) : 'none';
    var sh = shadow ? shadow.dx + 'px ' + shadow.dy + 'px ' + shadow.blur + 'px ' + (shadow.spread || 0) + 'px ' + css(shadow.color) : '';
    // A freeform shape's shadow follows its outline (a box shadow would be a rectangle behind it).
    if (entry.node._path && entry.node._path.ownerSVGElement) {
      var seP = o.soft_edge || 0;
      var fx = seP > 0 ? 'blur(' + (seP / 4) + 'px)'
        : (shadow ? 'drop-shadow(' + shadow.dx + 'px ' + shadow.dy + 'px ' + Math.max(0, shadow.blur / 2) + 'px ' + css(shadow.color) + ') ' : '') +
          (o.glow ? 'drop-shadow(0 0 ' + Math.max(0, o.glow.radius / 2) + 'px ' + css(o.glow.color) + ')' : '');
      entry.node._path.ownerSVGElement.style.filter = fx;
      entry.node._path.ownerSVGElement.style.overflow = seP > 0 ? 'visible' : '';
      sh = '';
    } else if (o.glow) {
      var gl = '0 0 ' + o.glow.radius + 'px ' + Math.round(o.glow.radius / 4) + 'px ' + css(o.glow.color);
      sh = sh ? sh + ',' + gl : gl;
    }
    // The focus ring is dark with a white inner band, so it shows on any background.
    if (entry.ring) sh = (sh ? sh + ',' : '') + '0 0 0 3px #fff';
    entry.node.style.boxShadow = sh || 'none';
    // Soft edges fade the object out towards its border, following its outline. A picture is faded with a mask; a box or
    // an oval draws its fill on a smaller, blurred layer behind its text; a freeform shape is blurred as a whole (above).
    // Reflection is not supported by every browser; where it is not, the object simply has none.
    var se = o.soft_edge || 0, mask = '';
    if (se > 0 && entry.node._img) {
      var fade = function (dir) { return 'linear-gradient(' + dir + ',transparent,#000 ' + se + 'px,#000 calc(100% - ' + se + 'px),transparent)'; };
      mask = fade('to right') + ',' + fade('to bottom');
    }
    entry.node.style.webkitMaskImage = mask; entry.node.style.maskImage = mask;
    entry.node.style.webkitMaskComposite = mask ? 'source-in' : ''; entry.node.style.maskComposite = mask ? 'intersect' : '';
    if (se > 0 && !entry.node._img && !entry.node._path && !entry.node._line) {
      var back = entry._soft;
      if (!back) {
        back = el('div', 'soft-back'); back.setAttribute('aria-hidden', 'true');
        back.style.cssText = 'position:absolute;pointer-events:none;z-index:-1';
        entry.node.insertBefore(back, entry.node.firstChild); entry._soft = back; entry.node.style.isolation = 'isolate';
      }
      back.style.display = '';
      back.style.inset = se / 2 + 'px';
      back.style.filter = 'blur(' + se / 4 + 'px)';
      back.style.borderRadius = o.kind === 'Ellipse' ? '50%' : Math.max(0, (o.radius || 0) - se / 2) + 'px';
      back.style.background = entry.node.style.background;
      entry.node.style.background = 'none'; entry.node.style.border = 'none'; entry.node.style.boxShadow = entry.ring ? '0 0 0 3px #fff' : 'none';
    } else if (entry._soft) entry._soft.style.display = 'none';
    var rf = o.reflection;
    entry.node.style.webkitBoxReflect = rf
      ? 'below ' + rf.gap + 'px linear-gradient(to bottom,transparent,transparent ' + Math.round((1 - rf.size) * 100) + '%,rgba(255,255,255,' + rf.opacity + '))'
      : '';
    if (entry.clickable && !entry.drawCheck) {
      if (entry.state === 'Disabled') entry.node.setAttribute('aria-disabled', 'true'); else entry.node.removeAttribute('aria-disabled');
    }
    if (entry.node._img) {
      // A picture's states can each show a different picture.
      var u = P.assetUrls[(s && s.asset) || o.asset];
      if (u && entry.node._img.getAttribute('src') !== u) entry.node._img.src = u;
    }
    if (o.states.some(function (x) { return x.size; })) {
      // ...at their own size, about the object's centre.
      var sw = s && s.size ? s.size[0] : o.w, sh = s && s.size ? s.size[1] : o.h;
      entry.node.style.width = sw + 'px'; entry.node.style.height = sh + 'px';
      entry.node.style.left = (o.x + (o.w - sw) / 2) + 'px'; entry.node.style.top = (o.y + (o.h - sh) / 2) + 'px';
    }
    var l = entry.label;
    // Plain text with list markers is drawn paragraph by paragraph, so a long item can hang under its own text.
    var LIST_MARK = /^(\u00a0*)(•|\d+\.) /;
    if (!rich && text && text.split('\n').some(function (t) { return LIST_MARK.test(t); })) {
      rich = text.split('\n').map(function (t) { return { runs: [{ text: t }], indent: 0 }; });
    }
    // The label's content is rebuilt only when it changed. Redrawing on every hover or press would replace the
    // element under the pointer, and the browser then drops the click that follows the press.
    var sig = rich ? 'r' + JSON.stringify(rich.map(function (para) { return para.runs.map(function (r) { return [subst(r.text), r.bold, r.italic, r.underline, r.color, r.size, r.font_family, r.link, r.strike, r.highlight, r.spacing]; }).concat([para.indent || 0]); })) : 't' + subst(text || '');
    var rebuild = l._sig !== sig;
    l._sig = sig;
    if (!rebuild) {
      // Same text: keep the nodes, restyle below.
    } else if (rich) {
      // Mixed-style text: one block per paragraph, one span per run (overrides only).
      l.textContent = '';
      rich.forEach(function (para) {
        var pEl = el('div');
        var ptext = para.runs.map(function (r) { return r.text; }).join('');
        var mk = LIST_MARK.exec(ptext);
        var hang = mk && ptext.length > mk[0].length ? (mk[2] === '•' ? 1.3 : 1.6) : 0;
        var pad = (para.indent || 0) * 40;
        // A list item hangs: the marker sits at the indent, wrapped lines line up with the text after it.
        if (hang) { pEl.style.paddingLeft = 'calc(' + pad + 'px + ' + hang + 'em)'; pEl.style.textIndent = '-' + hang + 'em'; }
        else if (pad) pEl.style.paddingLeft = pad + 'px';
        var runs = para.runs;
        if (hang && runs.length && runs[0].text.indexOf(mk[0]) === 0 && runs[0].text.length > mk[0].length) {
          // The marker shares a run with its text: give it a run of its own.
          var head = JSON.parse(JSON.stringify(runs[0])); head.text = mk[0];
          var rest = JSON.parse(JSON.stringify(runs[0])); rest.text = runs[0].text.slice(mk[0].length);
          runs = [head, rest].concat(runs.slice(1));
        }
        var markerRun = hang ? runs[0] : null;
        runs.forEach(function (r) {
          var sp = el('span', null, subst(r.text));
          if (r === markerRun) { sp.style.display = 'inline-block'; sp.style.minWidth = hang + 'em'; sp.style.textIndent = '0'; }
          if (r.link && /^(https?:|mailto:)/i.test(r.link)) {
            sp = el('a', null, subst(r.text)); sp.href = r.link; sp.target = '_blank'; sp.rel = 'noopener noreferrer';
            // A link is the theme's hyperlink color, underlined; the run's own color and underline, applied below, win.
            var lc = P.theme && P.theme.colors && P.theme.colors[4];
            if (lc) sp.style.color = css(lc);
            sp.style.textDecoration = 'underline';
            sp.addEventListener('click', function (ev) { ev.stopPropagation(); });
          }
          if (r.bold !== null && r.bold !== undefined) sp.style.fontWeight = r.bold ? '700' : '400';
          if (r.italic !== null && r.italic !== undefined) sp.style.fontStyle = r.italic ? 'italic' : 'normal';
          if (r.underline !== null && r.underline !== undefined) sp.style.textDecoration = r.underline ? 'underline' : 'none';
          if (r.strike) sp.style.textDecoration = (sp.style.textDecoration && sp.style.textDecoration !== 'none' ? sp.style.textDecoration + ' ' : '') + 'line-through';
          if (r.highlight) sp.style.backgroundColor = css(r.highlight);
          if (r.spacing) sp.style.letterSpacing = r.spacing + 'px';
          if (r.color) sp.style.color = css(r.color);
          if (r.size) sp.style.fontSize = r.size + 'px';
          if (r.font_family) sp.style.fontFamily = '"' + r.font_family + '", "Open Sans", Montserrat, Arial, sans-serif';
          pEl.appendChild(sp);
        });
        // An empty paragraph, or one that ends in a line break, still takes the line it asks for.
        var ptxt = ptext;
        if (ptxt === '' || ptxt.slice(-1) === '\n') pEl.appendChild(document.createTextNode('\u200b'));
        l.appendChild(pEl);
      });
    } else {
      l.textContent = subst(text || '');
    }
    l.style.fontFamily = '"' + st.font_family + '", "Open Sans", Montserrat, Arial, sans-serif';
    l.style.fontSize = st.size + 'px';
    l.style.fontWeight = st.bold ? '700' : '400';
    l.style.fontStyle = st.italic ? 'italic' : 'normal';
    l.style.textDecoration = st.underline ? 'underline' : 'none';
    l.style.color = css(st.color);
    l.style.textAlign = st.align.toLowerCase();
    l.style.lineHeight = o.line_spacing ? String(o.line_spacing) : '';
    if (o.text_rotation) {
      // Lay the text out in a box with width and height swapped, then turn the box: 270 reads
      // bottom-to-top, 90 top-to-bottom. The normal layout (wrapping, alignment) still applies.
      l.style.inset = 'auto';
      l.style.left = '0px'; l.style.top = '0px';
      l.style.width = o.h + 'px'; l.style.height = o.w + 'px';
      l.style.transformOrigin = '0 0';
      if (o.text_rotation === 270) { l.style.top = o.h + 'px'; l.style.transform = 'rotate(-90deg)'; }
      else { l.style.left = o.w + 'px'; l.style.transform = 'rotate(90deg)'; }
    }
    l.style.justifyContent = st.valign === 'Top' ? 'flex-start' : st.valign === 'Bottom' ? 'flex-end' : 'center';
    entry.node.style.cursor = entry.cursor || ((buttonLike(o) || entry.clickable) && entry.state !== 'Disabled' ? 'pointer' : 'default');
  }

  // ---- animation (skipped when the viewer prefers reduced motion) ----------------------------
  var reduceMotion = !!(window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);
  var FROM = {
    Fade: { opacity: 0 },
    SlideLeft: { opacity: 0, transform: 'translateX(-80px)' },
    SlideRight: { opacity: 0, transform: 'translateX(80px)' },
    SlideUp: { opacity: 0, transform: 'translateY(-60px)' },
    SlideDown: { opacity: 0, transform: 'translateY(60px)' },
    Zoom: { opacity: 0, transform: 'scale(0.6)' },
    Wipe: { clipPath: 'inset(0 100% 0 0)' }
  };
  var TO = { opacity: 1, transform: 'none', clipPath: 'inset(0 0 0 0)' };
  var layerTimers = {}, layerAnims = {};

  function animate(layerId, node, frames, options) {
    if (reduceMotion || !node.animate) return;
    var a = node.animate(frames, options);
    (layerAnims[layerId] = layerAnims[layerId] || []).push(a);
  }
  // Effects that need more than a start frame: spin turns while growing, wheel sweeps round like a clock
  // hand, split opens from (or closes to) a line. Masks cannot be tweened, so they are stepped finely.
  var SPIN = { SpinQuarter: 90, SpinHalf: 180, SpinFull: 360 };
  var SPLIT = { SplitVerticalOut: [true, true], SplitVerticalIn: [true, false], SplitHorizontalOut: [false, true], SplitHorizontalIn: [false, false] };
  function maskFrames(n, at) {
    var out = [];
    for (var i = 0; i <= n; i++) { var g = at(i / n); out.push({ maskImage: g, webkitMaskImage: g }); }
    return out;
  }
  // The frames that bring an object in (`enter`) or take it out.
  function effectFrames(kind, enter) {
    var frames;
    if (SPIN[kind] !== undefined) {
      frames = [{ opacity: 0, transform: 'scale(0.1) rotate(' + (-SPIN[kind]) + 'deg)' }, { opacity: 1, transform: 'none' }];
      if (!enter) frames = [{ opacity: 1, transform: 'none' }, { opacity: 0, transform: 'scale(0.1) rotate(' + SPIN[kind] + 'deg)' }];
      return frames;
    }
    if (kind === 'Wheel') {
      frames = maskFrames(24, function (f) { var a = (f * 360).toFixed(1); return 'conic-gradient(#000 0deg ' + a + 'deg, transparent ' + a + 'deg 360deg)'; });
    } else if (SPLIT[kind]) {
      var vertical = SPLIT[kind][0], out = SPLIT[kind][1], dir = vertical ? '90deg' : '180deg';
      if (out) {
        frames = [{ clipPath: vertical ? 'inset(0 50% 0 50%)' : 'inset(50% 0 50% 0)' }, { clipPath: 'inset(0 0 0 0)' }];
      } else {
        frames = maskFrames(20, function (f) {
          var p = (f * 50).toFixed(1), q = (100 - f * 50).toFixed(1);
          return 'linear-gradient(' + dir + ', #000 0%, #000 ' + p + '%, transparent ' + p + '%, transparent ' + q + '%, #000 ' + q + '%, #000 100%)';
        });
      }
    } else {
      var from = FROM[kind] || FROM.Fade;
      frames = enter ? [from, TO] : [TO, from];
      return frames;
    }
    return enter ? frames : frames.slice().reverse();
  }
  function playEntrance(layerId, o) {
    var e = objects[o.id];
    if (!o.entrance || !e) return;
    animate(layerId, e.node, effectFrames(o.entrance.kind, true),
      { duration: o.entrance.duration_ms, delay: o.entrance.delay_ms, easing: SPIN[o.entrance.kind] !== undefined || o.entrance.kind in SPLIT || o.entrance.kind === 'Wheel' ? 'linear' : 'ease-out', fill: 'backwards' });
  }
  // Emphasis (pulse, spin, ...) and motion paths. Spin/Swing use the `rotate` property and motion uses
  // `translate`, so they combine with the entrance effect and with the object's own rotation.
  var EMPH = {
    Pulse: function () { return [{ transform: 'scale(1)' }, { transform: 'scale(1.12)' }, { transform: 'scale(1)' }]; },
    Grow: function () { return [{ transform: 'scale(1)' }, { transform: 'scale(1.3)' }]; },
    Shake: function () { return [{ transform: 'translateX(0)' }, { transform: 'translateX(-14px)' }, { transform: 'translateX(14px)' }, { transform: 'translateX(-14px)' }, { transform: 'translateX(14px)' }, { transform: 'translateX(0)' }]; },
    Flash: function () { return [{ opacity: 1 }, { opacity: 0.15 }, { opacity: 1 }]; },
    Bounce: function () { return [{ transform: 'translateY(0)' }, { transform: 'translateY(-40px)' }, { transform: 'translateY(0)' }]; },
    Spin: function (base) { return [{ rotate: base + 'deg' }, { rotate: (base + 360) + 'deg' }]; },
    Swing: function (base) { return [{ rotate: base + 'deg' }, { rotate: (base - 14) + 'deg' }, { rotate: (base + 14) + 'deg' }, { rotate: base + 'deg' }]; }
  };
  function playEmphasis(layerId, o, now) {
    var e = objects[o.id], em = o.emphasis;
    if (!em || !e || !EMPH[em.kind]) return;
    animate(layerId, e.node, EMPH[em.kind](o.rotation || 0), {
      duration: em.duration_ms, delay: now ? 0 : em.delay_ms, easing: 'ease-in-out', composite: 'add',
      iterations: em.repeat === 0 ? Infinity : Math.max(1, em.repeat), fill: em.kind === 'Grow' ? 'forwards' : 'none'
    });
  }
  // The points of a path, with a smooth curve (Catmull-Rom) through them when asked.
  function pathPoints(m) {
    var p = m.points;
    if (!m.smooth || p.length < 3) return p.slice();
    var out = [p[0]], N = 12;
    for (var i = 0; i < p.length - 1; i++) {
      var p0 = p[Math.max(i - 1, 0)], p1 = p[i], p2 = p[i + 1], p3 = p[Math.min(i + 2, p.length - 1)];
      for (var k = 1; k <= N; k++) {
        var t = k / N, t2 = t * t, t3 = t2 * t;
        var f = function (a, b, c, d) { return 0.5 * ((2 * b) + (-a + c) * t + (2 * a - 5 * b + 4 * c - d) * t2 + (-a + 3 * b - 3 * c + d) * t3); };
        out.push([f(p0[0], p1[0], p2[0], p3[0]), f(p0[1], p1[1], p2[1], p3[1])]);
      }
    }
    return out;
  }
  function playMotion(layerId, o, now) {
    var e = objects[o.id], m = o.motion;
    if (!m || !e || !m.points || m.points.length < 2) return;
    var pts = pathPoints(m), cum = [0], total = 0;
    for (var i = 1; i < pts.length; i++) { total += Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]); cum.push(total); }
    if (!total) return;
    var frames = pts.map(function (p, i) { return { translate: p[0] + 'px ' + p[1] + 'px', offset: cum[i] / total }; });
    animate(layerId, e.node, frames, { duration: m.duration_ms, delay: now ? 0 : m.delay_ms, easing: 'ease-in-out', fill: 'forwards' });
  }
  // `auto` = started by the timeline: effects that wait for a click are left for runStep.
  function clickOf(o, k) { return (o.click_steps && o.click_steps[k]) || 0; }
  function playExtras(layerId, o, now, auto) {
    if (!(auto && clickOf(o, 'emphasis') > 0)) playEmphasis(layerId, o, now);
    if (!(auto && clickOf(o, 'motion') > 0)) playMotion(layerId, o, now);
  }
  function playExit(layerId, o, delay) {
    var e = objects[o.id];
    if (!o.exit || !e) return;
    animate(layerId, e.node, effectFrames(o.exit.kind, false),
      { duration: o.exit.duration_ms, delay: delay || 0, easing: SPIN[o.exit.kind] !== undefined || o.exit.kind in SPLIT || o.exit.kind === 'Wheel' ? 'linear' : 'ease-in', fill: 'forwards' });
  }

  // A layer's timeline starts when the layer is shown: objects with a start time appear then,
  // objects with a finite duration (or an exit effect) leave, and the end of the timeline fires
  // the slide's "timeline ends" triggers (base layer) or moves on when the slide auto-advances.
  function stopLayer(id) {
    (layerTimers[id] || []).forEach(clearTimeout);
    layerTimers[id] = [];
    (layerAnims[id] || []).forEach(function (a) { try { a.cancel(); } catch (e) {} });
    layerAnims[id] = [];
    var l = layers[id];
    if (l) l.layer.objects.forEach(function (o) { objects[o.id].node.style.visibility = 'visible'; });
  }
  // `from` starts the timeline part-way through (a "seek"): objects whose time has come are on screen
  // without replaying their entrance, and objects whose time has gone are already off.
  function startLayer(id, from) {
    var l = layers[id]; if (!l) return;
    from = from || 0;
    l.startedAt = Date.now() - from;
    stopLayer(id);
    var timers = layerTimers[id];
    var total = l.layer.duration_ms;
    var wait = function (ms) { return Math.max(0, ms - from); };
    l.layer.objects.forEach(function (o) {
      var e = objects[o.id], t = o.timing;
      var start = t ? t.start_ms : 0;
      var end = t && t.dur_ms !== null && t.dur_ms !== undefined ? t.start_ms + t.dur_ms : null;
      if (end === null && o.exit && total) end = total;   // an exit effect on an until-end object plays at the end
      var byClick = l.base;   // effects that wait for a click only exist on the base layer
      // Media set to start by itself plays when its turn on the timeline comes (again from the top when the timeline restarts).
      var mediaEl = o.media && o.media.autoplay && e.node.querySelector('video,audio');
      if (mediaEl && !(end !== null && end <= from && from > 0)) {
        var playIt = function () {
          try { mediaEl.currentTime = Math.max(0, (from - start) / 1000); var pr = mediaEl.play(); if (pr && pr.catch) pr.catch(function () {}); } catch (err) {}
        };
        if (start > from) timers.push(setTimeout(playIt, start - from)); else playIt();
      }
      if (end !== null && end <= from && from > 0) { e.node.style.visibility = 'hidden'; return; }
      if (byClick && clickOf(o, 'entrance') > 0) {
        e.node.style.visibility = 'hidden';   // appears when its click comes
      } else if (start > from) {
        e.node.style.visibility = 'hidden';
        timers.push(setTimeout(function () { e.node.style.visibility = 'visible'; playEntrance(id, o); playExtras(id, o, false, byClick); }, start - from));
      } else if (from > 0) {
        e.node.style.visibility = 'visible';
      } else {
        playEntrance(id, o);
        playExtras(id, o, false, byClick);
      }
      if (end !== null && !(byClick && clickOf(o, 'exit') > 0)) {
        var exitMs = o.exit && !reduceMotion ? o.exit.duration_ms : 0;
        if (o.exit) timers.push(setTimeout(function () { playExit(id, o); }, wait(end - exitMs)));
        timers.push(setTimeout(function () { e.node.style.visibility = 'hidden'; }, wait(end)));
      }
    });
    if (total) timers.push(setTimeout(function () { layerEnded(id); }, wait(total)));
    else if (l.base) timelineDone[l.si] = true;
    // "Timeline reaches" triggers of the slide (base layer) or of this layer.
    slides[l.si].triggers.forEach(function (t) {
      if (!t.event || typeof t.event !== 'object' || t.event.TimelineReaches === undefined) return;
      var at = t.event.TimelineReaches;
      if (at < from || (at === from && from > 0)) return;
      var mine = l.base ? t.owner === 'Slide' : t.owner.Layer === id;
      if (mine) timers.push(setTimeout(function () { if (slides[cur] === slides[l.si]) runTrigger(t); }, at - from));
    });
  }
  var timelineDone = {};  // slide index -> its base timeline has finished (or it has none)

  // ---- click steps (PowerPoint-style): each click plays the effects numbered for it ----------------
  var steps = { si: -1, order: [], done: 0 };
  function computeSteps(si) {
    var seen = {};
    slides[si].layers[0].objects.forEach(function (o) {
      ['entrance', 'emphasis', 'motion', 'exit'].forEach(function (k) { if (clickOf(o, k) > 0) seen[clickOf(o, k)] = 1; });
    });
    return Object.keys(seen).map(Number).sort(function (a, b) { return a - b; });
  }
  function pendingSteps() { return steps.si === cur ? steps.order.length - steps.done : 0; }
  function runStep() {
    if (pendingSteps() <= 0) return false;
    var n = steps.order[steps.done++];
    var layer = slides[cur].layers[0];
    layer.objects.forEach(function (o) {
      var e = objects[o.id]; if (!e) return;
      if (clickOf(o, 'entrance') === n) { e.node.style.visibility = 'visible'; playEntrance(layer.id, o); }
      if (clickOf(o, 'emphasis') === n) playEmphasis(layer.id, o, false);
      if (clickOf(o, 'motion') === n) playMotion(layer.id, o, false);
      if (clickOf(o, 'exit') === n) {
        var delay = (o.exit && o.exit.delay_ms) || 0, dur = o.exit ? o.exit.duration_ms : 0;
        playExit(layer.id, o, delay);
        (layerTimers[layer.id] = layerTimers[layer.id] || []).push(setTimeout(function () { e.node.style.visibility = 'hidden'; }, delay + (reduceMotion ? 0 : dur)));
      }
    });
    updateNav();
    return true;
  }
  stage.addEventListener('click', function (ev) {
    if (pendingSteps() <= 0) return;
    if (ev.target && ev.target.closest && ev.target.closest('button,input,select,textarea,a,[role=button],#nav,#menu,.q')) return;
    runStep();
  });
  document.addEventListener('keydown', function (ev) {
    if (ev.defaultPrevented || pendingSteps() <= 0 || /^(INPUT|TEXTAREA|SELECT|BUTTON|A|VIDEO|AUDIO)$/.test(ev.target && ev.target.tagName)) return;
    if (ev.target && ev.target.closest && ev.target.closest('[role=button],[role=slider],[role=checkbox],[role=radio]')) return;
    if (ev.key === ' ' || ev.key === 'ArrowRight' || ev.key === 'PageDown') { ev.preventDefault(); runStep(); }
  });
  function layerEnded(id) {
    var l = layers[id];
    if (!l || !l.base || slides[cur] !== slides[l.si]) return;
    timelineDone[l.si] = true;
    updateNav();
    var handled = 0;
    slides[l.si].triggers.forEach(function (t) {
      if (t.event === 'SlideEnd' && holds(t)) { runTrigger(t); handled++; }
    });
    if (!handled && slides[l.si].auto_advance && pendingSteps() === 0) goto(step(1));
  }

  function showLayer(id) {
    var l = layers[id]; if (!l || l.base) return;
    var was = l.el.style.display !== 'none';
    // Some layers hide the slide's other layers when they show.
    if (!was && l.layer.hide_others) {
      Object.keys(layers).forEach(function (k) { if (k !== id && layers[k].si === l.si && !layers[k].base && layers[k].el.style.display !== 'none') hideLayer(k); });
    }
    if (!was) startLayer(id);
    l.el.style.display = 'block';
    if (!was) hideBaseFor(id, true);
    if (!was && l.layer.pause_base) { (pausing[l.si] = pausing[l.si] || {})[id] = true; basePauseChanged(l.si); }
    if (!was) {
      if (byUser()) {
        var from = document.activeElement;
        l.returnTo = from && from !== document.body && !l.el.contains(from) ? from : null;
        var first = focusables(l.el)[0];
        if (first) focusIt(first);
      }
      slides[l.si].triggers.forEach(function (t) {
        if (t.event === 'LayerStart' && t.owner.Layer === id) runTrigger(t);
      });
    }
  }
  // A layer with "pause the base layer" stops the slide's timeline and its media while it shows (an interactive video's
  // questions), and everything runs on from where it stopped once the last such layer hides.
  var pausing = {};   // slide index -> { layer id: true }
  function basePauseChanged(si) {
    var base = layers[slides[si].layers[0].id]; if (!base) return;
    var any = Object.keys(pausing[si] || {}).length > 0;
    if (any && !base.paused) {
      base.paused = true; base.pausedAt = Date.now(); base.wasPlaying = [];
      (layerTimers[slides[si].layers[0].id] || []).forEach(clearTimeout);
      layerTimers[slides[si].layers[0].id] = [];
      Array.prototype.forEach.call(base.el.querySelectorAll('video,audio'), function (m) { if (!m.paused) { base.wasPlaying.push(m); try { m.pause(); } catch (e) {} } });
    } else if (!any && base.paused) {
      base.paused = false;
      startLayer(slides[si].layers[0].id, base.pausedAt - base.startedAt);
      (base.wasPlaying || []).forEach(function (m) { try { var pr = m.play(); if (pr && pr.catch) pr.catch(function () {}); } catch (e) {} });
    }
  }
  // Base-layer objects that a layer hides while it shows ("hide objects on base layer").
  function hideBaseFor(layerId, on) {
    var l = layers[layerId]; if (!l || l.base) return;
    slides[l.si].layers[0].objects.forEach(function (o) {
      if (!o.hide_on_layers || o.hide_on_layers.indexOf(layerId) < 0) return;
      var e = objects[o.id]; if (!e) return;
      e._hl = e._hl || {};
      if (on) e._hl[layerId] = 1; else delete e._hl[layerId];
      e.node.classList.toggle('hidden-by-layer', Object.keys(e._hl).length > 0);
    });
  }
  function pauseMedia(root) {
    Array.prototype.forEach.call(root.querySelectorAll('video,audio'), function (m) { try { m.pause(); } catch (e) {} });
  }
  function hideLayer(id) {
    var l = layers[id]; if (!l || l.base) return;
    pauseMedia(l.el);
    stopLayer(id);
    hideBaseFor(id, false);
    if (pausing[l.si] && pausing[l.si][id]) { delete pausing[l.si][id]; basePauseChanged(l.si); }
    var held = document.activeElement && l.el.contains(document.activeElement);
    l.el.style.display = 'none';
    if (held) { focusIt(l.returnTo && document.body.contains(l.returnTo) && shown(l.returnTo) ? l.returnTo : titleEl); }
    l.returnTo = null;
    // Objects return to their initial state when a layer is hidden.
    l.layer.objects.forEach(function (o) {
      var e = objects[o.id]; e.state = o.initial_state; draw(e);
    });
  }
  function resetSlide(si) {
    slides[si].layers.forEach(function (layer, li) {
      stopLayer(layer.id);
      layers[layer.id].el.style.display = li === 0 ? 'block' : 'none';
      if (li > 0) hideBaseFor(layer.id, false);
      if (li === 0) { pausing[si] = {}; layers[layer.id].paused = false; }
      layer.objects.forEach(function (o) { var e = objects[o.id]; e.state = o.initial_state; e.node.style.display = o.hidden ? 'none' : ''; draw(e); });
    });
  }

  // ---- triggers --------------------------------------------------------------------------
  function cmp(a, op, b) {
    switch (op) {
      case 'Eq': return a === b; case 'Ne': return a !== b;
      case 'Gt': return a > b; case 'Lt': return a < b;
      case 'Ge': return a >= b; case 'Le': return a <= b;
    }
    return false;
  }
  function holds(t) {
    var r = true;
    t.conditions.forEach(function (c, i) {
      var v = cmp(vars[c.variable], c.cmp, unwrap(c.value));
      r = i === 0 ? v : (c.or_with_previous ? (r || v) : (r && v));
    });
    return r;
  }
  function runTrigger(t) {
    if (!holds(t)) return;
    var a = t.action, key = typeof a === 'string' ? a : Object.keys(a)[0], arg = a[key];
    if (key === 'ShowLayer') showLayer(arg);
    else if (key === 'HideLayer') hideLayer(arg);
    else if (key === 'SetVariable') setVar(arg.variable, unwrap(arg.value));
    else if (key === 'SetObjectState') {
      var e = objects[arg.object];
      if (e) setObjectState(e, arg.state);
    } else if (key === 'RunJavaScript') {
      try { (new Function(arg))(); } catch (err) { console.error('Course script failed', err); }
    } else if (key === 'JumpToSlide') {
      var idx = slides.findIndex(function (s) { return s.id === arg; });
      if (idx >= 0) goto(idx);
    } else if (key === 'NextSlide') goto(step(1));
    else if (key === 'PreviousSlide') goto(step(-1));
    else if (key === 'ShowObject' || key === 'HideObject') {
      var oe = objects[arg];
      if (oe) oe.node.style.display = key === 'ShowObject' ? '' : 'none';
    } else if (key === 'PlayMedia' || key === 'PauseMedia' || key === 'StopMedia') {
      var me = objects[arg], m = me && me.node.querySelector('video,audio');
      if (m) { try { if (key === 'PlayMedia') { var pr = m.play(); if (pr && pr.catch) pr.catch(function () {}); } else { m.pause(); if (key === 'StopMedia') m.currentTime = 0; } } catch (err) {} }
    } else if (key === 'AdjustVariable') {
      var cur0 = vars[arg.variable], by = unwrap(arg.value), next;
      if (arg.op === 'Append') next = String(cur0 === undefined ? '' : cur0) + String(by);
      else {
        var x = Number(cur0), y = Number(by);
        if (isNaN(x)) x = 0;
        next = arg.op === 'Add' ? x + y : arg.op === 'Subtract' ? x - y : arg.op === 'Multiply' ? x * y : (y === 0 ? x : x / y);
      }
      setVar(arg.variable, next);
    } else if (key === 'PlayAnimation') {
      var ae = objects[arg];
      if (ae) playExtras(ae.layerId, ae.o, true);
    } else if (key === 'SeekTimeline') {
      seekTimeline(arg.target, arg.ms, t);
    } else if (key === 'CompleteCourse') {
      if (lms) { lms.complete(); lms.commit(); }
    } else if (key === 'ExitCourse') {
      exitCourse();
    } else if (key === 'RestartCourse') {
      restartCourse();
    } else if (key === 'OpenUrl') {
      if (/^(https?:|mailto:)/i.test(arg)) window.open(arg, '_blank', 'noopener');
    }
  }

  // Seek a layer's timeline (an empty target is the layer the trigger sits on), or a video or audio object.
  function seekTimeline(target, ms, t) {
    var id = target;
    if (!id) {
      var o = t.owner;
      id = o.Layer !== undefined ? o.Layer : cur >= 0 ? slides[cur].layers[0].id : '';
      if (o.Object !== undefined && objects[o.Object]) id = objects[o.Object].layerId;
    }
    if (layers[id]) { if (layers[id].el.style.display !== 'none') startLayer(id, ms); return; }
    var e = objects[id], m = e && e.node.querySelector('video,audio');
    if (m) { try { m.currentTime = ms / 1000; } catch (err) {} }
  }
  function exitCourse() {
    saveNow();
    if (lms) lms.finish();
    try { window.close(); } catch (err) {}
    // A browser tab the course did not open cannot be closed by it; say so instead of staying silent.
    setTimeout(function () {
      if (document.getElementById('exited')) return;
      var box = el('div'); box.id = 'exited'; box.setAttribute('role', 'status');
      box.style.cssText = 'position:fixed;inset:0;display:flex;align-items:center;justify-content:center;background:#fff;color:#222;font:20px "Open Sans",Arial,sans-serif;z-index:99999';
      box.textContent = 'You have left the course. You can close this window.';
      document.body.appendChild(box);
    }, 100);
  }
  function restartCourse() {
    P.variables.forEach(function (v) { vars[v.id] = unwrap(v.initial); });
    answers = {}; visited = {};
    slides.forEach(function (s, i) { timelineDone[i] = false; resetSlide(i); });
    questionSlides.forEach(function (s) { buildQuestion(slides.indexOf(s)); });
    textVarEntries.forEach(draw);
    cur = -1;
    goto(seq[0]);
  }

  // ---- question and results slides ---------------------------------------------------------
  var questionUi = {};  // slide id -> { rebuild }
  // Each question style supplies: node (the answer controls), collect() -> array of answer tokens
  // or null when nothing is answered, correct(tokens) -> bool, lock(record) to show a stored answer.
/*@QUESTIONS@*/
  // The question (or results) UI lives in a host beneath the slide's layers, so shapes, pictures, buttons and
  // triggers placed on the slide work as on any other slide and are never wiped when the question is rebuilt.
  function questionHost(si) {
    var sEl = slideEls[si];
    if (!sEl._qhost) { sEl._qhost = el('div', 'q-host'); sEl.insertBefore(sEl._qhost, sEl.firstChild); }
    sEl._qhost.textContent = '';
    return sEl._qhost;
  }
  function buildQuestion(si) {
    var slide = slides[si], q = slide.kind.Question, root = questionHost(si);
    var box = renderQuestion(q, si, slide.id, answers[slide.id], function (rec) {
      answers[slide.id] = rec;
      textVarEntries.forEach(draw);
      updateNav();
      saveSoon();
      reportProgress();
    });
    var n = questionSlides.indexOf(slide) + 1;
    box.insertBefore(el('div', 'q-count', 'Question ' + n + ' of ' + questionSlides.length), box.firstChild);
    if (q.area) {
      // The author gave the question its own area, so the slide's background and objects show around it; the
      // text scales down with a smaller area.
      var a = q.area, k = Math.max(0.3, Math.min(1, a.w / 1560, a.h / 860));
      var wrap = el('div', 'q-place');
      wrap.style.cssText = 'position:absolute;overflow:auto;left:' + a.x + 'px;top:' + a.y + 'px;width:' + a.w + 'px;height:' + a.h + 'px';
      box.style.cssText = 'position:static;background:transparent;padding:24px 30px;box-sizing:border-box;overflow:visible;zoom:' + k + ';width:' + (a.w / k) + 'px';
      wrap.appendChild(box); root.appendChild(wrap);
    } else root.appendChild(box);
  }
  function buildResults(si) {
    var root = questionHost(si);
    var t = totals();
    var box = el('div', 'q');
    box.appendChild(el('h2', 'q-prompt', 'Results'));
    box.appendChild(el('div', 'r-score', t.pct + '%'));
    box.appendChild(el('div', 'r-detail', t.earned + ' of ' + t.max + ' points. Pass mark: ' + P.pass_mark + '%.'));
    var verdict = el('div', 'r-verdict ' + (t.passed ? 'ok' : 'bad'), t.passed ? 'You passed.' : 'You did not pass.');
    verdict.setAttribute('role', 'status');
    box.appendChild(verdict);
    var retry = el('button', 'q-submit', 'Retry'); retry.type = 'button';
    retry.addEventListener('click', function () {
      answers = {};
      redraw(true);
      questionSlides.forEach(function (s) { buildQuestion(slides.indexOf(s)); });
      saveNow();
      goto(seq[0]);
    });
    box.appendChild(retry);
    root.appendChild(box);
  }

  // ---- navigation ------------------------------------------------------------------------
  var nav = null, prevBtn = null, nextBtn = null, counter = null;
  function nextButtonTriggers(si) {
    return slides[si].triggers.filter(function (t) { return t.event === 'NextButton'; });
  }
  function prevButtonTriggers(si) {
    return slides[si].triggers.filter(function (t) { return t.event === 'PreviousButton'; });
  }
  var anyNextTriggers = slides.some(function (_, i) { return nextButtonTriggers(i).length > 0 || prevButtonTriggers(i).length > 0; });
  var PL = P.player || {};
  var PC = PL.controls || {};
  var wantControls = !!(PC.play_pause || PC.seekbar || PC.replay || PC.volume || PC.captions || PC.speed || PC.fullscreen);
  var navButtons = P.show_nav && (slides.length > 1 || anyNextTriggers);
  // A button as the author chose to draw them: an icon, an icon with its words, or the words alone.
  var ICONS = {
    prev: '<path d="M12 4l-6 6 6 6" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>',
    next: '<path d="M8 4l6 6-6 6" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>',
    play: '<path d="M6 4l10 6-10 6z" fill="currentColor"/>',
    pause: '<path d="M6 4h3v12H6zM11 4h3v12h-3z" fill="currentColor"/>',
    replay: '<path d="M4 10a6 6 0 1 0 2-4.5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/><path d="M3 3v4h4" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>',
    menu: '<path d="M3 5h14M3 10h14M3 15h14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>',
    volume: '<path d="M3 8h3l4-3v10l-4-3H3z" fill="currentColor"/><path d="M13 7.5c1.4 1.4 1.4 3.6 0 5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>',
    muted: '<path d="M3 8h3l4-3v10l-4-3H3z" fill="currentColor"/><path d="M13 7.5l4 5M17 7.5l-4 5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>',
    captions: '<rect x="2" y="4" width="16" height="12" rx="2" fill="none" stroke="currentColor" stroke-width="1.7"/><text x="10" y="13" font-size="7" font-weight="700" text-anchor="middle" fill="currentColor" font-family="Arial,sans-serif">CC</text>',
    fullscreen: '<path d="M3 7V3h4M13 3h4v4M17 13v4h-4M7 17H3v-4" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>'
  };
  function setButton(btn, label, icon) {
    var style = PL.buttons || 'Text';
    btn.textContent = '';
    if (style !== 'Text' && icon && ICONS[icon]) {
      var svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
      svg.setAttribute('width', '18'); svg.setAttribute('height', '18'); svg.setAttribute('viewBox', '0 0 20 20'); svg.setAttribute('aria-hidden', 'true');
      svg.innerHTML = ICONS[icon];
      if (icon === 'prev' && style === 'IconText') { btn.appendChild(svg); btn.appendChild(el('span', null, label)); }
      else if (style === 'IconText') { btn.appendChild(el('span', null, label)); btn.appendChild(svg); }
      else btn.appendChild(svg);
      btn.classList.add('has-icon');
    } else btn.textContent = label;
    btn.setAttribute('aria-label', label);
    btn.title = label;
  }
  if (navButtons || wantControls) {
    nav = el('nav'); nav.id = 'nav'; nav.setAttribute('aria-label', 'Course navigation');
    document.body.appendChild(nav);
  }
  if (navButtons) {
    prevBtn = el('button'); prevBtn.type = 'button'; setButton(prevBtn, 'Previous', 'prev');
    nextBtn = el('button'); nextBtn.type = 'button'; setButton(nextBtn, 'Next', 'next');
    counter = el('span', 'nav-count');
    nav.appendChild(prevBtn); nav.appendChild(counter); nav.appendChild(nextBtn);
    prevBtn.addEventListener('click', function () {
      // Slide triggers on the player's Previous button replace the default move to the previous slide.
      var handled = false;
      prevButtonTriggers(cur).forEach(function (t) { if (holds(t)) { runTrigger(t); handled = true; } });
      if (!handled) goto(step(-1));
    });
    nextBtn.addEventListener('click', function () {
      // Waiting animation steps play before the player moves on.
      if (runStep()) return;
      // Slide triggers on the player's Next button replace the default move to the next slide.
      var handled = false;
      nextButtonTriggers(cur).forEach(function (t) { if (holds(t)) { runTrigger(t); handled = true; } });
      if (!handled) goto(step(1));
    });
  }
  // Slide menu. By default a drawer over the slide, opened from a Menu button in the bar; with a sidebar it is a panel
  // beside the slide (Menu and Notes tabs, an optional logo above them), opened and closed from the top bar. Unless the
  // author allows free movement, a learner cannot jump past a question they have not answered.
  var sideMode = PL.sidebar || 'Off';
  var sidebarOn = sideMode !== 'Off' && (!!P.show_menu || !!PL.tab_notes || !!PL.tab_glossary);
  var logoUrl = PL.logo && P.assetUrls[PL.logo] ? P.assetUrls[PL.logo] : null;
  // A banner photo: a picture across the top of the sidebar, above the navigation. It belongs to the sidebar, so a
  // player without one (or with it collapsed) does not show it. It is decoration, so it has no alt text.
  var bannerUrl = PL.banner && P.assetUrls[PL.banner] ? P.assetUrls[PL.banner] : null;
  var menu = null, menuBtn = null, progress = null, topbar = null, sideToggle = null, notesBody = null, sbTabs = {}, resPanel = null;
  function filterMenu(q) {
    q = (q || '').trim().toLowerCase();
    if (!menu) return;
    Array.prototype.forEach.call(menu.querySelectorAll('button[data-index]'), function (b) {
      b.parentNode.classList.toggle('nomatch', !!q && b.textContent.toLowerCase().indexOf(q) < 0);
    });
  }
  function jumpAllowed(i) {
    if (P.menu_free) return true;
    for (var k = 0; k < i; k++) if (kindOf(slides[k]) === 'Question' && posOf[k] >= 0 && !answers[slides[k].id]) return false;
    return true;
  }
  var resLinks = (P.resources || []).map(function (r) {
    var href = r.asset && P.assetUrls[r.asset] ? P.assetUrls[r.asset] : (/^(https?:|mailto:)/i.test(r.url || '') ? r.url : '');
    return href ? { title: r.title || r.url || 'Resource', href: href, file: !!r.asset } : null;
  }).filter(Boolean);
  var glossTerms = (P.glossary || []).filter(function (g) { return (g.term || '').trim(); }).sort(function (a, b) { return a.term.localeCompare(b.term); });
  var showResources = !!PL.tab_resources && resLinks.length > 0;
  // The top bar: the sidebar's toggle, the small logo and the title. It starts beside an open left sidebar (see fit).
  if (sidebarOn || PL.show_title || showResources || logoUrl) {
    topbar = el('header'); topbar.id = 'topbar';
    if (sidebarOn) {
      sideToggle = el('button', 'tb-toggle'); sideToggle.type = 'button';
      sideToggle.setAttribute('aria-label', 'Show or hide the sidebar'); sideToggle.setAttribute('aria-controls', 'menu');
      sideToggle.innerHTML = '<svg width="20" height="20" viewBox="0 0 20 20" aria-hidden="true"><path d="M3 5h14M3 10h14M3 15h14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>';
      topbar.appendChild(sideToggle);
    }
    if (logoUrl) { var tl = el('img', 'logo'); tl.src = logoUrl; tl.alt = ''; topbar.appendChild(tl); }
    if (PL.show_title) { var tt = el('h1', 'tb-title', (PL.title && PL.title.trim()) ? PL.title : (P.name || '')); topbar.appendChild(tt); }
    if (showResources) {
      var rb = el('button', 'tb-resources', 'Resources'); rb.type = 'button'; rb.setAttribute('aria-expanded', 'false'); rb.setAttribute('aria-controls', 'resources');
      var rp = el('div'); rp.id = 'resources'; rp.hidden = true; rp.setAttribute('role', 'dialog'); rp.setAttribute('aria-label', 'Resources');
      var rl = el('ul');
      resLinks.forEach(function (r) {
        var li = el('li'); var a = el('a', null, r.title); a.href = r.href;
        if (r.file) a.setAttribute('download', r.title); else { a.target = '_blank'; a.rel = 'noopener noreferrer'; }
        li.appendChild(a); rl.appendChild(li);
      });
      rp.appendChild(rl);
      rb.addEventListener('click', function () { rp.hidden = !rp.hidden; rb.setAttribute('aria-expanded', String(!rp.hidden)); });
      document.addEventListener('keydown', function (ev) { if (ev.key === 'Escape' && !rp.hidden) { rp.hidden = true; rb.setAttribute('aria-expanded', 'false'); rb.focus(); } });
      topbar.appendChild(rb); document.body.appendChild(rp); resPanel = rp;
    }
    document.body.appendChild(topbar);
  }
  // Whether or not it is shown, the project's name is the page's one Heading 1.
  if (!PL.show_title) { var h1 = el('h1', 'sr-only', (PL.title && PL.title.trim()) ? PL.title : (P.name || '')); stage.insertBefore(h1, stage.firstChild); }
  if ((nav && P.show_menu) || sidebarOn) {
    menu = el('nav'); menu.id = 'menu'; menu.setAttribute('aria-label', sidebarOn ? 'Sidebar' : 'Slides');
    var list = el('ol');
    if (sidebarOn) {
      menu.className = 'sidebar ' + (sideMode === 'Left' ? 'side-left' : 'side-right');
      menu.hidden = !!PL.sidebar_collapsed;
      if (bannerUrl) { var sb = el('img', 'banner'); sb.src = bannerUrl; sb.alt = ''; menu.appendChild(sb); }
      var tabs = [];
      if (P.show_menu) tabs.push(['menu', 'Menu']);
      if (PL.tab_glossary && glossTerms.length) tabs.push(['glossary', 'Glossary']);
      if (PL.tab_notes) tabs.push(['notes', 'Notes']);
      var tablist = el('div', 'sb-tabs'); tablist.setAttribute('role', 'tablist');
      var panels = {};
      function selectTab(id) {
        Object.keys(panels).forEach(function (k) { panels[k].hidden = k !== id; sbTabs[k].setAttribute('aria-selected', String(k === id)); });
      }
      tabs.forEach(function (t) {
        var b = el('button', null, t[1]); b.type = 'button'; b.setAttribute('role', 'tab'); b.dataset.tab = t[0];
        b.addEventListener('click', function () { selectTab(t[0]); });
        sbTabs[t[0]] = b; tablist.appendChild(b);
        panels[t[0]] = el('div', 'sb-panel'); panels[t[0]].setAttribute('role', 'tabpanel');
      });
      if (PL.search && P.show_menu) {
        var sbtn = el('button', 'sb-search'); sbtn.type = 'button'; sbtn.setAttribute('aria-label', 'Search slides'); sbtn.setAttribute('aria-expanded', 'false');
        sbtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 20 20" aria-hidden="true"><circle cx="8.5" cy="8.5" r="5.5" fill="none" stroke="currentColor" stroke-width="2"/><path d="M13 13l5 5" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>';
        var sbox = el('div', 'sb-searchbox'); sbox.hidden = true;
        var sin = el('input'); sin.type = 'search'; sin.placeholder = 'Search slides'; sin.setAttribute('aria-label', 'Search slides');
        sbox.appendChild(sin);
        sin.addEventListener('input', function () { filterMenu(sin.value); });
        sbtn.addEventListener('click', function () {
          sbox.hidden = !sbox.hidden; sbtn.setAttribute('aria-expanded', String(!sbox.hidden));
          if (!sbox.hidden) { selectTab('menu'); sin.focus(); } else { sin.value = ''; filterMenu(''); }
        });
        var head = el('div', 'sb-head'); head.appendChild(tablist); head.appendChild(sbtn);
        menu.appendChild(head); menu.appendChild(sbox);
      } else if (tabs.length > 1) menu.appendChild(tablist);
      if (panels.menu) { panels.menu.appendChild(list); menu.appendChild(panels.menu); }
      if (panels.notes) { notesBody = el('div', 'notes'); panels.notes.appendChild(notesBody); menu.appendChild(panels.notes); }
      if (panels.glossary) {
        var dl = el('dl', 'notes');
        glossTerms.forEach(function (g) { dl.appendChild(el('dt', null, g.term)); dl.appendChild(el('dd', null, g.definition || '')); });
        panels.glossary.appendChild(dl); menu.appendChild(panels.glossary);
      }
      selectTab(tabs[0][0]);
    } else {
      menu.hidden = true;
      menu.appendChild(list);
    }
    var lastScene = null;
    slides.forEach(function (s, i) {
      if (sidebarOn && s.scene !== lastScene) {
        lastScene = s.scene;
        var sc = (P.scenes || []).filter(function (c) { return c.id === s.scene; })[0];
        if (sc) {
          var hl = el('li', 'scene-head'); var hb = el('button', null, sc.name); hb.type = 'button'; hb.setAttribute('aria-expanded', 'true'); hb.dataset.scene = sc.id;
          hb.addEventListener('click', function () {
            var open = hb.getAttribute('aria-expanded') !== 'true';
            hb.setAttribute('aria-expanded', String(open));
            Array.prototype.forEach.call(list.querySelectorAll('li[data-scene="' + sc.id + '"]'), function (n) { n.classList.toggle('collapsed', !open); });
          });
          hl.appendChild(hb); list.appendChild(hl);
        }
      }
      var li = el('li'); li.dataset.scene = s.scene; var b = el('button', null, s.name || ('Slide ' + (i + 1)));
      b.type = 'button'; b.dataset.index = String(i);
      b.addEventListener('click', function () {
        if (!sidebarOn) { menu.hidden = true; if (menuBtn) menuBtn.setAttribute('aria-expanded', 'false'); }
        goto(i); focusIt(titleEl);
      });
      li.appendChild(b); list.appendChild(li);
    });
    document.body.appendChild(menu);
    if (sidebarOn) {
      sideToggle.setAttribute('aria-expanded', String(!menu.hidden));
      sideToggle.addEventListener('click', function () {
        var opening = menu.hidden || sideClosing;
        sideToggle.setAttribute('aria-expanded', String(opening));
        slideSidebar(opening);
      });
    } else {
      menuBtn = el('button'); menuBtn.type = 'button'; setButton(menuBtn, 'Menu', 'menu');
      menuBtn.setAttribute('aria-expanded', 'false'); menuBtn.setAttribute('aria-controls', 'menu');
      menuBtn.style.order = '-2'; if (!wantControls) menuBtn.style.marginRight = 'auto'; nav.insertBefore(menuBtn, nav.firstChild);
      menuBtn.addEventListener('click', function () {
        menu.hidden = !menu.hidden;
        menuBtn.setAttribute('aria-expanded', String(!menu.hidden));
        if (!menu.hidden) { var f = menu.querySelector('button:not([disabled])'); if (f) f.focus(); }
      });
      document.addEventListener('keydown', function (ev) {
        if (ev.key === 'Escape' && !menu.hidden) { menu.hidden = true; menuBtn.setAttribute('aria-expanded', 'false'); menuBtn.focus(); }
      });
    }
  }
  if (nav && P.show_progress) {
    progress = el('div'); progress.id = 'progress'; progress.setAttribute('role', 'progressbar');
    progress.setAttribute('aria-label', 'Course progress'); progress.setAttribute('aria-valuemin', '0'); progress.setAttribute('aria-valuemax', '100');
    progress.appendChild(el('div'));
    nav.appendChild(progress);
  }
  // Player controls: volume, captions, speed and full-screen, along the right of the bar.
  if (nav && wantControls) {
    var cbox = el('div', 'nav-controls'); nav.appendChild(cbox); nav.classList.add('has-controls');
    var playerMuted = false, captionsOn = true, SPEEDS = [1, 1.25, 1.5, 2, 0.75], speedAt = 0;
    var medias = function () { return Array.prototype.slice.call(document.querySelectorAll('video,audio')); };
    var applyMedia = function (m) {
      if (playerMuted) m.muted = true;
      if (PC.speed) m.playbackRate = SPEEDS[speedAt];
      if (PC.captions) Array.prototype.forEach.call(m.textTracks || [], function (t) { t.mode = captionsOn ? 'showing' : 'hidden'; });
    };
    // Media that starts later picks the choices up when it plays.
    document.addEventListener('play', function (ev) { if (ev.target && ev.target.tagName && /^(VIDEO|AUDIO)$/.test(ev.target.tagName)) applyMedia(ev.target); }, true);
    var control = function (id, label, icon) {
      var b = el('button', 'ctl-' + id); b.type = 'button'; setButton(b, label, icon); cbox.appendChild(b); return b;
    };
    // Play / pause, replay and a seekbar for the current slide. They drive its timeline when it has a length, and
    // otherwise its video or audio; a slide with neither leaves them greyed out.
    if (PC.play_pause || PC.seekbar || PC.replay) {
      var tMs = function () { return cur >= 0 ? (slides[cur].layers[0].duration_ms || 0) : 0; };
      var baseL = function () { return layers[slides[cur].layers[0].id]; };
      var tMedia = function () { var b = cur >= 0 && baseL(); return b ? b.el.querySelector('video,audio') : null; };
      var tKind = function () { return cur < 0 ? 'none' : tMs() > 0 ? 'timeline' : tMedia() ? 'media' : 'none'; };
      var tLength = function () {
        var k = tKind(); if (k === 'timeline') return tMs();
        var m = k === 'media' ? tMedia() : null; return m && isFinite(m.duration) ? Math.round(m.duration * 1000) : 0;
      };
      var tPos = function () {
        var k = tKind(); if (k === 'timeline') {
          if (timelineDone[cur]) return tMs();
          var b = baseL(); return Math.max(0, Math.min(tMs(), (b.paused ? b.pausedAt : Date.now()) - b.startedAt));
        }
        var m = k === 'media' ? tMedia() : null; return m ? Math.round(m.currentTime * 1000) : 0;
      };
      var tPaused = function () {
        var k = tKind(); if (k === 'timeline') return !!baseL().paused || !!timelineDone[cur];
        var m = k === 'media' ? tMedia() : null; return !m || m.paused;
      };
      var fmtTime = function (ms) { var n = Math.max(0, Math.floor(ms / 1000)); return Math.floor(n / 60) + ':' + ('0' + (n % 60)).slice(-2); };
      var tReplay = function () {
        var si = cur, k = tKind(); if (k === 'none') return;
        if (k === 'media') { var m = tMedia(); try { m.currentTime = 0; var pr = m.play(); if (pr && pr.catch) pr.catch(function () {}); } catch (e) {} tRefresh(); return; }
        resetSlide(si); timelineDone[si] = false;
        steps = { si: si, order: computeSteps(si), done: 0 };
        startLayer(slides[si].layers[0].id);
        tRefresh();
      };
      var tToggle = function () {
        var k = tKind(); if (k === 'none') return;
        if (k === 'media') { var m = tMedia(); try { if (m.paused) { var pr = m.play(); if (pr && pr.catch) pr.catch(function () {}); } else m.pause(); } catch (e) {} tRefresh(); return; }
        if (timelineDone[cur]) { tReplay(); return; }
        var held = pausing[cur] = pausing[cur] || {};
        if (held.__user) delete held.__user; else if (!baseL().paused) held.__user = true; else return;   // a layer holds it: only closing the layer runs it on
        basePauseChanged(cur); tRefresh();
      };
      var tSeek = function (ms) {
        var k = tKind(); if (k === 'none') return;
        if (k === 'media') { try { tMedia().currentTime = ms / 1000; } catch (e) {} tRefresh(); return; }
        var id = slides[cur].layers[0].id, b = baseL(), was = b.paused;
        timelineDone[cur] = false;
        startLayer(id, ms);
        if (was) {   // seeking while paused shows that moment and stays paused
          (layerTimers[id] || []).forEach(clearTimeout); layerTimers[id] = []; pauseMedia(b.el);
          var now = Date.now(); b.pausedAt = now; b.startedAt = now - ms;
        }
        tRefresh();
      };
      var pb = null, rb = null, sk = null, tm = null, scrubbing = false, lastLabel = '';
      if (PC.play_pause) { pb = control('play', 'Pause', 'pause'); pb.addEventListener('click', tToggle); }
      if (PC.replay) { rb = control('replay', 'Replay', 'replay'); rb.addEventListener('click', tReplay); }
      if (PC.seekbar) {
        sk = el('input', 'ctl-seek'); sk.type = 'range'; sk.min = '0'; sk.max = '0'; sk.value = '0'; sk.setAttribute('aria-label', 'Seek');
        tm = el('span', 'ctl-time', '0:00 / 0:00'); tm.setAttribute('aria-hidden', 'true');
        sk.addEventListener('input', function () { scrubbing = true; tm.textContent = fmtTime(+sk.value) + ' / ' + fmtTime(+sk.max); });
        sk.addEventListener('change', function () { scrubbing = false; tSeek(+sk.value); });
        sk.addEventListener('blur', function () { scrubbing = false; });
        cbox.appendChild(sk); cbox.appendChild(tm);
      }
      var tTimer = null;
      var tRefresh = function () {
        clearTimeout(tTimer); tTimer = null;
        var k = tKind(), none = k === 'none', len = tLength();
        // While something is playing the bar follows it; otherwise there is nothing to poll for.
        if (!none && !tPaused()) tTimer = setTimeout(tRefresh, 250);
        if (pb) {
          var label = none || !tPaused() ? 'Pause' : 'Play';
          if (label !== lastLabel) { setButton(pb, label, label === 'Pause' ? 'pause' : 'play'); lastLabel = label; }
          pb.disabled = none;
        }
        if (rb) rb.disabled = none;
        if (sk) {
          sk.disabled = none || len <= 0;
          sk.max = String(len); sk.step = len >= 2000 ? '1000' : '100';
          if (!scrubbing) { var pos = tPos(); sk.value = String(pos); tm.textContent = fmtTime(pos) + ' / ' + fmtTime(len); sk.setAttribute('aria-valuetext', fmtTime(pos) + ' of ' + fmtTime(len)); }
        }
      };
      controlUpdaters.push(tRefresh);
      ['play', 'pause', 'ended', 'loadedmetadata', 'durationchange'].forEach(function (n) { document.addEventListener(n, tRefresh, true); });
      tRefresh();
    }
    if (PC.volume) {
      var vb = control('volume', 'Mute', 'volume'); vb.setAttribute('aria-pressed', 'false');
      vb.addEventListener('click', function () {
        playerMuted = !playerMuted;
        medias().forEach(function (m) { m.muted = playerMuted || (m.dataset.authorMuted === '1'); });
        setButton(vb, playerMuted ? 'Unmute' : 'Mute', playerMuted ? 'muted' : 'volume'); vb.setAttribute('aria-pressed', String(playerMuted));
      });
    }
    if (PC.captions) {
      var cb = control('captions', 'Captions', 'captions'); cb.setAttribute('aria-pressed', 'true');
      cb.addEventListener('click', function () {
        captionsOn = !captionsOn; medias().forEach(applyMedia); cb.setAttribute('aria-pressed', String(captionsOn));
      });
    }
    if (PC.speed) {
      var sb = control('speed', 'Speed', null); sb.textContent = '1x'; sb.setAttribute('aria-label', 'Playback speed 1x');
      sb.addEventListener('click', function () {
        speedAt = (speedAt + 1) % SPEEDS.length; medias().forEach(applyMedia);
        sb.textContent = SPEEDS[speedAt] + 'x'; sb.setAttribute('aria-label', 'Playback speed ' + SPEEDS[speedAt] + 'x');
      });
    }
    if (PC.fullscreen && document.fullscreenEnabled !== false && document.documentElement.requestFullscreen) {
      var fb = control('fullscreen', 'Full screen', 'fullscreen');
      fb.addEventListener('click', function () {
        if (document.fullscreenElement) document.exitFullscreen(); else document.documentElement.requestFullscreen();
      });
      document.addEventListener('fullscreenchange', fit);
    }
  }
  // The Notes tab: the current slide's notes.
  function updateNotes() {
    if (!notesBody) return;
    notesBody.textContent = '';
    var t = (slides[cur].notes || '').trim();
    var p = el('div', t ? 'notes-text' : 'notes-empty', t || 'No notes for this slide.');
    notesBody.appendChild(p);
  }
  function updateMenu() {
    if (menu) {
      Array.prototype.forEach.call(menu.querySelectorAll('button[data-index]'), function (b) {
        var i = parseInt(b.dataset.index, 10);
        b.parentNode.hidden = posOf[i] < 0;
        b.disabled = !jumpAllowed(i);
        b.classList.toggle('visited', !!visited[slides[i].id]);
        if (i === cur) b.setAttribute('aria-current', 'page'); else b.removeAttribute('aria-current');
      });
    }
    if (progress) {
      var seen = seq.filter(function (i) { return visited[slides[i].id]; }).length;
      var pct = Math.round(seen / seq.length * 100);
      progress.setAttribute('aria-valuenow', String(pct));
      progress.firstChild.style.width = pct + '%';
    }
  }
  function updateNav() {
    updateMenu();
    updateNotes();
    if (!nav || !counter) return;
    counter.textContent = (posOf[cur] + 1) + ' / ' + seq.length;
    counter.setAttribute('role', 'img'); counter.setAttribute('aria-label', 'Slide ' + (posOf[cur] + 1) + ' of ' + seq.length);
    var rules = slides[cur].nav || {};
    prevBtn.disabled = posOf[cur] <= 0 || rules.previous_allowed === false;
    // A question must be answered before moving on.
    var gated = kindOf(slides[cur]) === 'Question' && !answers[slides[cur].id];
    var ruled = rules.next === 'Never' || (rules.next === 'AfterTimeline' && !timelineDone[cur]);
    var stuck = (posOf[cur] >= seq.length - 1 && nextButtonTriggers(cur).length === 0) || ruled;
    nextBtn.disabled = gated || (stuck && !(pendingSteps() > 0 && !ruled));
  }

  function reportProgress() {
    if (!lms) return;
    var t = totals();
    var quizDone = questionSlides.length > 0 && t.answered >= questionSlides.length;
    if (questionSlides.length > 0) {
      if (quizDone && (!hasResults || kindOf(slides[cur]) === 'Results')) lms.score(t.pct, t.passed);
    } else if (seq.every(function (i) { return visited[i] || visited[slides[i].id]; })) {
      lms.complete();
    }
    lms.commit();
  }

  // ---- slide transitions: the new slide arrives over, beside or instead of the old one ----------
  var endTransitionNow = null;
  function endTransition() { if (endTransitionNow) { var f = endTransitionNow; endTransitionNow = null; f(); } }
  function runTransition(tr, ms, oldEl, newEl) {
    var kind = tr.kind, edge = tr.from || 'Right', inward = !!tr.inward;
    // Without a canvas to make the mask with, a dissolve is a fade.
    if (kind === 'Dissolve' && !document.createElement('canvas').getContext('2d')) kind = 'Fade';
    // Where things come from, as a move of the whole slide (percent of its size), and as an edge to reveal.
    var vec = { Right: [100, 0], Left: [-100, 0], Bottom: [0, 100], Top: [0, -100] }[edge] || [100, 0];
    var shift = function (k) { return 'translate(' + (vec[0] * k) + '%,' + (vec[1] * k) + '%)'; };
    var clip = { Right: 'inset(0 0 0 100%)', Left: 'inset(0 100% 0 0)', Bottom: 'inset(100% 0 0 0)', Top: 'inset(0 0 100% 0)' }[edge] || 'inset(0 0 0 100%)';
    var vertical = edge === 'Left' || edge === 'Right';
    var band = vertical ? 'inset(0 50% 0 50%)' : 'inset(50% 0 50% 0)';
    var opts = { duration: ms, easing: 'ease-in-out' };
    var anims = [], undo = [];
    function play(el, frames) { var a = el.animate(frames, opts); if (a) anims.push(a); }
    function over(el, z) { el.style.zIndex = z; undo.push(function () { el.style.zIndex = ''; }); }
    oldEl.style.display = 'block'; oldEl.style.pointerEvents = 'none';
    undo.push(function () { oldEl.style.display = 'none'; oldEl.style.pointerEvents = ''; });
    var raf = 0;
    if (kind === 'Push') {
      play(newEl, [{ transform: shift(1) }, { transform: 'none' }]); play(oldEl, [{ transform: 'none' }, { transform: shift(-1) }]);
    } else if (kind === 'Cover') {
      over(newEl, 2); over(oldEl, 1);
      play(newEl, [{ transform: shift(1) }, { transform: 'none' }]);
    } else if (kind === 'Uncover') {
      over(oldEl, 2); over(newEl, 1);
      play(oldEl, [{ transform: 'none' }, { transform: shift(-1) }]);
    } else if (kind === 'Wipe') {
      over(newEl, 2); over(oldEl, 1);
      play(newEl, [{ clipPath: clip }, { clipPath: 'inset(0 0 0 0)' }]);
    } else if (kind === 'Split') {
      if (inward) { over(oldEl, 2); over(newEl, 1); play(oldEl, [{ clipPath: 'inset(0 0 0 0)' }, { clipPath: band }]); }
      else { over(newEl, 2); over(oldEl, 1); play(newEl, [{ clipPath: band }, { clipPath: 'inset(0 0 0 0)' }]); }
    } else if (kind === 'Zoom') {
      if (inward) { over(newEl, 2); over(oldEl, 1); play(newEl, [{ transform: 'scale(0.15)', opacity: 0 }, { transform: 'none', opacity: 1 }]); }
      else { over(oldEl, 2); over(newEl, 1); play(oldEl, [{ transform: 'none', opacity: 1 }, { transform: 'scale(0.15)', opacity: 0 }]); }
    } else if (kind === 'Dissolve') {
      // The new slide shows through a mask of small squares that fill in, in a scattered order.
      over(newEl, 2); over(oldEl, 1);
      var cols = 64, rows = 36, total = cols * rows, order = [];
      for (var q = 0; q < total; q++) order.push(q);
      for (var r = total - 1; r > 0; r--) { var j = Math.floor(Math.random() * (r + 1)), t = order[r]; order[r] = order[j]; order[j] = t; }
      var canvas = document.createElement('canvas'); canvas.width = cols; canvas.height = rows;
      var g = canvas.getContext('2d'), t0 = performance.now(), shown = 0;
      var mask = function (url) { newEl.style.webkitMaskImage = url; newEl.style.maskImage = url; };
      newEl.style.webkitMaskSize = newEl.style.maskSize = '100% 100%';
      undo.push(function () { mask(''); newEl.style.webkitMaskSize = newEl.style.maskSize = ''; });
      var step = function (now) {
        var p = Math.min(1, (now - t0) / ms), n = Math.floor(p * total);
        if (n > shown) { g.fillStyle = '#000'; for (var k = shown; k < n; k++) g.fillRect(order[k] % cols, Math.floor(order[k] / cols), 1, 1); shown = n; mask('url(' + canvas.toDataURL() + ')'); }
        if (p < 1) raf = requestAnimationFrame(step); else endTransition();
      };
      mask('url(' + canvas.toDataURL() + ')');
      raf = requestAnimationFrame(step);
    } else {
      play(newEl, [{ opacity: 0 }, { opacity: 1 }]);
    }
    var done = false;
    endTransitionNow = function () {
      if (done) return; done = true;
      if (raf) cancelAnimationFrame(raf);
      anims.forEach(function (a) { try { a.cancel(); } catch (e) { /* already finished */ } });
      undo.forEach(function (f) { f(); });
    };
    if (anims.length) anims[0].onfinish = function () { endTransition(); };
  }

  function goto(i) {
    markerClosers.forEach(function (f) { f(false); });
    if (i < 0 || i >= slides.length || i === cur || posOf[i] < 0) return;
    if (cur >= 0) slides[cur].layers.forEach(function (layer) { stopLayer(layer.id); });
    var held = document.activeElement, heldInSlide = !!held && held !== document.body && slideEls.some(function (e) { return e.contains(held); });
    var from = cur;
    cur = i;
    slideEls.forEach(function (e, k) { if (k !== i) pauseMedia(e); e.style.display = k === i ? 'block' : 'none'; });
    var slide = slides[i], kind = kindOf(slide);
    var seen = !!visited[slide.id];
    visited[slide.id] = true;
    timelineDone[i] = false;
    if (kind === 'Question') buildQuestion(i);
    else if (kind === 'Results') buildResults(i);
    if (!(seen && slide.nav && slide.nav.revisit === 'Resume')) resetSlide(i);
    steps = { si: i, order: computeSteps(i), done: 0 };
    if (pausing[i] && pausing[i].__user) { delete pausing[i].__user; if (!Object.keys(pausing[i]).length) layers[slide.layers[0].id].paused = false; }
    startLayer(slide.layers[0].id);
    endTransition();
    if (slide.fade_ms > 0 && !reduceMotion && slideEls[i].animate) {
      var tr = slide.transition || {};
      if (from >= 0 && from !== i && tr.kind && tr.kind !== 'Fade') runTransition(tr, slide.fade_ms, slideEls[from], slideEls[i]);
      else slideEls[i].animate([{ opacity: 0 }, { opacity: 1 }], { duration: slide.fade_ms, easing: 'ease-out' });
    }
    controlUpdaters.forEach(function (f) { f(); });
    updateNav();
    var name = slide.name || ('Slide ' + (posOf[i] + 1));
    titleEl.textContent = name;
    // The project's name is the page's one Heading 1; each slide's (visually hidden) title is a Heading 2 under it.
    titleEl.setAttribute('role', 'heading'); titleEl.setAttribute('aria-level', '2'); titleEl.removeAttribute('aria-label');
    announcer.textContent = name + ', slide ' + (posOf[i] + 1) + ' of ' + seq.length;
    if (heldInSlide) focusIt(titleEl);   // what had focus is no longer on screen
    if (lms) lms.set(lms.locationKey, i);
    saveSoon();
    slide.triggers.forEach(function (t) { if (t.event === 'SlideStart') runTrigger(t); });
    reportProgress();
    stage.dataset.slide = String(i);
  }

  // ---- key presses: triggers owned by the slide, a visible layer, or a visible object ---------
  document.addEventListener('keydown', function (ev) {
    if (cur < 0 || /^(INPUT|TEXTAREA|SELECT|BUTTON)$/.test(ev.target && ev.target.tagName)) return;
    slides[cur].triggers.forEach(function (t) {
      if (!t.event || typeof t.event !== 'object' || t.event.KeyPress === undefined) return;
      if (String(t.event.KeyPress).toLowerCase() !== String(ev.key).toLowerCase()) return;
      var o = t.owner;
      if (o.Layer !== undefined) { var l = layers[o.Layer]; if (!l || l.el.style.display === 'none') return; }
      if (o.Object !== undefined) { var e = objects[o.Object]; if (!e || e.node.style.display === 'none' || e.node.parentNode.style.display === 'none') return; }
      runTrigger(t);
    });
  });

  // ---- fit stage to window ----------------------------------------------------------------
  // The sidebar slides in and out while the top bar, control bar and slide glide to their new places. With reduced motion
  // (or no animation support) it simply appears or disappears.
  var sideClosing = false, sideAnimTimer = null;
  function slideSidebar(open) {
    if (reduceMotion || !menu.animate) { menu.hidden = !open; sideClosing = false; fit(); return; }
    var away = sideMode === 'Right' ? '100%' : '-100%';
    document.body.classList.add('side-anim');
    clearTimeout(sideAnimTimer);
    sideAnimTimer = setTimeout(function () { document.body.classList.remove('side-anim'); }, 300);
    if (open) {
      sideClosing = false; menu.hidden = false; fit();
      menu.animate([{ transform: 'translateX(' + away + ')' }, { transform: 'none' }], { duration: 220, easing: 'ease' });
    } else {
      sideClosing = true; fit();
      var a = menu.animate([{ transform: 'none' }, { transform: 'translateX(' + away + ')' }], { duration: 220, easing: 'ease', fill: 'forwards' });
      var done = function () { if (sideClosing) { menu.hidden = true; sideClosing = false; } if (a && a.cancel) a.cancel(); };
      if (a) a.onfinish = done; else done();
    }
  }
  function fit() {
    var navH = nav ? nav.offsetHeight : 0;
    var topH = topbar ? topbar.offsetHeight : 0;
    // A sidebar takes a strip from the left or right; the slide fits what is left.
    var sideW = sidebarOn && menu && !menu.hidden && !sideClosing ? menu.offsetWidth : 0;
    // The sidebar runs the full height; the top bar and the control bar sit beside it, over and under the slide only.
    if (menu && sidebarOn) { menu.style.top = '0px'; menu.style.bottom = '0px'; }
    if (topbar) {
      topbar.style.left = sideMode === 'Left' ? sideW + 'px' : '0px';
      topbar.style.right = sideMode === 'Right' ? sideW + 'px' : '0px';
    }
    if (resPanel) resPanel.style.right = (sideMode === 'Right' ? sideW + 8 : 8) + 'px';
    if (nav) {
      nav.style.left = sideMode === 'Left' ? sideW + 'px' : '0px';
      nav.style.right = sideMode === 'Right' ? sideW + 'px' : '0px';
    }
    var shift = sideMode === 'Left' ? sideW / 2 : sideMode === 'Right' ? -sideW / 2 : 0;
    var s = Math.min((window.innerWidth - sideW) / slides[0].width, (window.innerHeight - navH - topH) / slides[0].height);
    stage.style.transform = 'translate(-50%, -50%) scale(' + s + ')';
    stage.style.left = 'calc(50% + ' + shift + 'px)';
    stage.style.top = 'calc(50% + ' + ((topH - navH) / 2) + 'px)';
  }
  window.addEventListener('resize', fit);

  // ---- start --------------------------------------------------------------------------------
  var startIndex = 0;
  if (lms && lms.init()) {
    var loc = restore();
    if (loc >= 0 && loc < slides.length) startIndex = loc;
    lms.incomplete();
    window.addEventListener('pagehide', function () { saveNow(); lms.finish(); });
    window.addEventListener('beforeunload', function () { saveNow(); lms.finish(); });
  }
  redraw(false);
  if (posOf[startIndex] < 0) startIndex = seq[0];
  textVarEntries.forEach(draw);
  questionSlides.forEach(function (s) { buildQuestion(slides.indexOf(s)); });
  fit();
  goto(startIndex);
  fit();
})();
