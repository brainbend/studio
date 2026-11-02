// BrainBend Studio web lesson player (original implementation): renders a Web Lesson's sections and
// blocks as one scrolling page, gates content behind Continue buttons, plays knowledge checks with
// the same question code as the slide player, tracks progress and reports to a SCORM 1.2 / 2004 LMS.
(function () {
  'use strict';
  var P = window.__BRAINBEND__;
  var L = P.lesson;
  function unwrap(v) { return v === undefined || v === null ? v : Object.values(v)[0]; }
  function el(tag, cls, text) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text !== undefined) e.textContent = text;
    return e;
  }
  function kindOf(b) { return typeof b.kind === 'string' ? b.kind : Object.keys(b.kind)[0]; }
  function dataOf(b) { return typeof b.kind === 'string' ? {} : b.kind[kindOf(b)]; }
  function url(key) { return key && P.assetUrls[key] ? P.assetUrls[key] : ''; }
  var reduceMotion = !!(window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);

  var vars = {};
  P.variables.forEach(function (v) { vars[v.id] = unwrap(v.initial); });
  function setVar(id, value) { if (vars[id] === value) return; vars[id] = value; saveSoon(); }

  /*@LMS@*/
  /*@QUESTIONS@*/

  // ---- state ----------------------------------------------------------------------------------
  var answers = {}, viewed = {}, opened = {};
  var blocks = [];          // { id, si, bi, key, data, node }
  var section = 0;          // the section the learner is in (for bookmarking and the menu)
  var reported = false;

  // ---- text: a small Markdown subset built with DOM calls (never innerHTML) ----------------------
  function safeUrl(u) { return /^(https?:|mailto:)/i.test(u); }
  function newTabNote(a) { a.appendChild(el('span', 'sr-only', ' (opens in a new tab)')); }
  function inline(parent, text) {
    var re = /\*\*([^*]+)\*\*|\*([^*]+)\*|\[([^\]]+)\]\(([^)\s]+)\)/g, last = 0, m;
    while ((m = re.exec(text))) {
      if (m.index > last) parent.appendChild(document.createTextNode(text.slice(last, m.index)));
      if (m[1] !== undefined) parent.appendChild(el('strong', null, m[1]));
      else if (m[2] !== undefined) parent.appendChild(el('em', null, m[2]));
      else if (safeUrl(m[4])) {
        var a = el('a', null, m[3]); a.href = m[4]; a.target = '_blank'; a.rel = 'noopener noreferrer';
        newTabNote(a); parent.appendChild(a);
      } else parent.appendChild(document.createTextNode(m[3]));
      last = re.lastIndex;
    }
    if (last < text.length) parent.appendChild(document.createTextNode(text.slice(last)));
  }
  function md(text) {
    var frag = document.createDocumentFragment();
    String(text || '').split(/\n\s*\n/).forEach(function (para) {
      var lines = para.split('\n').filter(function (l) { return l.trim() !== ''; });
      if (!lines.length) return;
      var bullets = lines.every(function (l) { return /^\s*[-*]\s+/.test(l); });
      var numbers = lines.every(function (l) { return /^\s*\d+[.)]\s+/.test(l); });
      if (bullets || numbers) {
        var list = el(numbers ? 'ol' : 'ul');
        lines.forEach(function (l) { var li = el('li'); inline(li, l.replace(/^\s*(?:[-*]|\d+[.)])\s+/, '')); list.appendChild(li); });
        frag.appendChild(list);
      } else {
        var p = el('p');
        lines.forEach(function (l, i) { if (i) p.appendChild(el('br')); inline(p, l); });
        frag.appendChild(p);
      }
    });
    return frag;
  }

  // ---- blocks -----------------------------------------------------------------------------------
  function figure(img, caption, width) {
    var f = el('figure', 'wl-figure' + (width === 'Medium' ? ' wl-w-medium' : width === 'Small' ? ' wl-w-small' : ''));
    f.appendChild(img);
    if (caption) f.appendChild(el('figcaption', null, caption));
    return f;
  }
  function picture(d, decorative) {
    var img = el('img');
    img.src = url(d.asset); img.alt = decorative ? '' : (d.alt || '');
    if (decorative) img.setAttribute('role', 'presentation');
    if (!d.crop) { img.loading = 'lazy'; return img; }
    // Cropped: the whole picture is stretched so the kept part fills a frame, which hides the rest. The frame
    // takes the shape of the kept part once the picture's size is known.
    var c = d.crop, cw = Math.max(0.01, 1 - c[0] - c[2]), ch = Math.max(0.01, 1 - c[1] - c[3]);
    var wrap = el('div', 'wl-crop');
    img.style.cssText = 'position:absolute;max-width:none;border-radius:0;left:' + (-c[0] / cw * 100) + '%;top:' + (-c[1] / ch * 100) + '%;width:' + (100 / cw) + '%;height:' + (100 / ch) + '%';
    function shape() { if (img.naturalWidth && img.naturalHeight) wrap.style.aspectRatio = (img.naturalWidth * cw) + ' / ' + (img.naturalHeight * ch); }
    img.addEventListener('load', shape); shape();
    wrap.appendChild(img);
    return wrap;
  }
  function tabs(items) {
    var box = el('div', 'wl-tabs'), list = el('div', 'wl-tablist'), panels = [], btns = [];
    list.setAttribute('role', 'tablist');
    var uid = 'tab' + (tabs.count = (tabs.count || 0) + 1);
    function pick(i, focus) {
      btns.forEach(function (b, k) { b.setAttribute('aria-selected', String(k === i)); b.tabIndex = k === i ? 0 : -1; panels[k].hidden = k !== i; });
      if (focus) btns[i].focus();
    }
    items.forEach(function (it, i) {
      var b = el('button', 'wl-tab', it.title); b.type = 'button'; b.setAttribute('role', 'tab');
      var p = el('div', 'wl-tabpanel'); p.setAttribute('role', 'tabpanel'); p.appendChild(md(it.body));
      b.id = uid + '-t' + i; p.id = uid + '-p' + i;
      b.setAttribute('aria-controls', p.id); p.setAttribute('aria-labelledby', b.id); p.tabIndex = 0;
      b.addEventListener('click', function () { pick(i, false); });
      b.addEventListener('keydown', function (ev) {
        var n = items.length;
        if (ev.key === 'ArrowRight') { ev.preventDefault(); pick((i + 1) % n, true); }
        else if (ev.key === 'ArrowLeft') { ev.preventDefault(); pick((i + n - 1) % n, true); }
        else if (ev.key === 'Home') { ev.preventDefault(); pick(0, true); }
        else if (ev.key === 'End') { ev.preventDefault(); pick(n - 1, true); }
      });
      list.appendChild(b); btns.push(b); panels.push(p);
    });
    box.appendChild(list); panels.forEach(function (p) { box.appendChild(p); });
    if (items.length) pick(0, false);
    return box;
  }
  function itemsBlock(d) {
    var style = d.style;
    if (style === 'Tabs') return tabs(d.items);
    var wrap = el('div', 'wl-items wl-' + style.toLowerCase());
    if (style === 'Accordion') {
      d.items.forEach(function (it) {
        var det = el('details', 'wl-acc'); det.appendChild(el('summary', null, it.title));
        var body = el('div', 'wl-acc-body'); body.appendChild(md(it.body)); det.appendChild(body); wrap.appendChild(det);
      });
    } else if (style === 'Flashcards') {
      d.items.forEach(function (it) {
        var c = el('button', 'wl-card'); c.type = 'button'; c.setAttribute('aria-pressed', 'false');
        var front = el('span', 'wl-card-front', it.title), back = el('span', 'wl-card-back'); back.appendChild(md(it.body)); back.hidden = true;
        c.appendChild(front); c.appendChild(back);
        c.addEventListener('click', function () {
          var on = c.getAttribute('aria-pressed') !== 'true';
          c.setAttribute('aria-pressed', String(on)); front.hidden = on; back.hidden = !on;
        });
        wrap.appendChild(c);
      });
    } else {
      var list = el('ol', 'wl-list');
      d.items.forEach(function (it, i) {
        var li = el('li');
        li.appendChild(el(style === 'Process' ? 'span' : 'strong', style === 'Process' ? 'wl-num' : 'wl-date', style === 'Process' ? String(i + 1) : it.title));
        var body = el('div', 'wl-li-body');
        if (style === 'Process') body.appendChild(el('h4', null, it.title));
        body.appendChild(md(it.body)); li.appendChild(body); list.appendChild(li);
      });
      wrap.appendChild(list);
    }
    return wrap;
  }
  function render(b, flat) {
    var key = kindOf(b), d = dataOf(b), n;
    switch (key) {
      case 'Heading':
        // The level asked for sets how big it looks; the tag never skips a level (a screen reader's outline).
        var want = Math.min(4, Math.max(2, (d.level || 2) + 1)), real = Math.min(want, lastHeading + 1);
        lastHeading = real;
        return el('h' + real, 'wl-heading wl-hl-' + want, d.text);
      case 'Text': n = el('div', 'wl-text'); n.appendChild(md(d.text)); return n;
      case 'Image': return d.asset ? figure(picture(d, d.decorative), d.caption, d.width) : el('div');
      case 'ImageText':
        n = el('div', 'wl-imagetext' + (d.image_right ? ' right' : ''));
        n.appendChild(d.asset ? picture(d, false) : el('div'));
        var t = el('div', 'wl-text'); t.appendChild(md(d.text)); n.appendChild(t); return n;
      case 'Video':
        var v = el('video'); v.controls = true; v.preload = 'metadata'; v.src = url(d.asset);
        if (d.captions && url(d.captions)) { var tr = el('track'); tr.kind = 'captions'; tr.src = url(d.captions); tr.label = 'Captions'; tr.srclang = 'en'; tr.default = true; v.appendChild(tr); }
        return figure(v, d.caption);
      case 'Audio':
        var a = el('audio'); a.controls = true; a.preload = 'metadata'; a.src = url(d.asset); a.setAttribute('aria-label', d.caption || 'Audio clip');
        return figure(a, d.caption);
      case 'Button':
        n = el('div', 'wl-buttonrow'); var link = el('a', 'wl-btn', d.label);
        if (safeUrl(d.url)) { link.href = d.url; link.target = '_blank'; link.rel = 'noopener noreferrer'; newTabNote(link); }
        n.appendChild(link); return n;
      case 'Divider': return el('hr', 'wl-divider');
      case 'Spacer': n = el('div', 'wl-spacer'); n.setAttribute('aria-hidden', 'true'); return n;
      case 'Quote':
        n = el('blockquote', 'wl-quote'); n.appendChild(el('p', null, d.text));
        if (d.by) n.appendChild(el('footer', null, d.by)); return n;
      case 'Statement': return el('p', 'wl-statement', d.text);
      case 'Items': return itemsBlock(d);
      case 'KnowledgeCheck':
        n = el('div', 'wl-q');
        n.appendChild(renderQuestion(d, flat, b.id, answers[b.id], function (rec) {
          answers[b.id] = rec; saveSoon(); update();
        }));
        return n;
      case 'Continue':
        n = el('div', 'wl-continue'); var btn = el('button', 'wl-btn', d.label || 'Continue'); btn.type = 'button';
        btn.addEventListener('click', function () { openGate(b.id); });
        n.appendChild(btn); return n;
    }
    return el('div');
  }

  // ---- page: a title page listing the modules, and one full-screen page per module ----------------
  var home = document.getElementById('home'), view = document.getElementById('module');
  var main = document.getElementById('main');
  var menu = document.getElementById('menu'), menuBtn = document.getElementById('menu-btn'), scrim = document.getElementById('scrim');
  var bar = document.getElementById('bar'), barText = document.getElementById('bar-text');
  var done = document.getElementById('done');
  var single = L.sections.length === 1;     // a one-section lesson: no title page, no menu
  var cur = -1;                             // the module on screen, or -1 for the title page
  var menuItems = [], cards = [], secNodes = [], nexts = [];
  var io = 'IntersectionObserver' in window ? new IntersectionObserver(function (entries) {
    entries.forEach(function (e) {
      if (e.isIntersecting && (e.intersectionRatio >= 0.25 || e.intersectionRect.height >= window.innerHeight * 0.25)) markViewed(e.target.dataset.bid);
    });
  }, { threshold: [0, 0.25, 0.5] }) : null;

  // title page
  var cover = L.cover || {};
  function textOrHide(id, t) { var n = document.getElementById(id); n.hidden = !t; if (t) n.textContent = t; }
  document.getElementById('hero-title').textContent = P.name;
  textOrHide('hero-sub', cover.subtitle); textOrHide('hero-desc', cover.description);
  if (cover.image && url(cover.image)) { var hi = document.getElementById('hero-img'); hi.src = url(cover.image); hi.hidden = false; }
  var startBtn = document.getElementById('start');
  var skip = document.querySelector('.skip');
  skip.addEventListener('click', function (ev) {
    ev.preventDefault();
    var t = document.getElementById(cur >= 0 ? 'main' : 'modules');
    if (t) { t.tabIndex = -1; t.focus(); }
  });

  var flat = 0, lastHeading = 1;
  L.sections.forEach(function (s, si) {
    lastHeading = 1;   // the module title is the h1
    var sec = el('section', 'wl-section'); sec.id = 'sec-' + si; sec.hidden = true;
    var head = el('header', 'wl-mhead'), hin = el('div', 'wl-mhead-in');
    if (!single) hin.appendChild(el('p', 'wl-eyebrow', 'Module ' + (si + 1) + ' of ' + L.sections.length));
    var h = el('h1', 'wl-section-title', s.title); h.id = 'sec-title-' + si; h.tabIndex = -1;
    sec.setAttribute('aria-labelledby', h.id); hin.appendChild(h);
    if (s.summary) hin.appendChild(el('p', 'wl-msummary', s.summary));
    var mp = el('div', 'wl-mprog'), track = el('div', 'wl-bar-track'), fill = el('div'), mt = el('span', null, '0%');
    track.appendChild(fill); mp.appendChild(track); mp.appendChild(mt); hin.appendChild(mp);
    head.appendChild(hin); sec.appendChild(head);
    s.blocks.forEach(function (b, bi) {
      var node = render(b, flat++);
      var wrap = el('div', 'wl-block wl-k-' + kindOf(b).toLowerCase()); wrap.dataset.bid = b.id; wrap.appendChild(node);
      sec.appendChild(wrap);
      blocks.push({ id: b.id, si: si, bi: bi, key: kindOf(b), data: dataOf(b), node: wrap });
    });
    // what comes next
    var nx = el('div', 'wl-mnext'), nin = el('div', 'wl-mnext-in'), ntext = el('div');
    var nb = el('button', 'wl-btn'); nb.type = 'button';
    if (si + 1 < L.sections.length) {
      ntext.appendChild(el('span', null, 'Up next')); ntext.appendChild(el('strong', null, L.sections[si + 1].title));
      nb.textContent = 'Next module'; nb.addEventListener('click', function () { go(si + 1); });
    } else {
      ntext.appendChild(el('span', null, single ? 'Finished' : 'You reached the end')); ntext.appendChild(el('strong', null, single ? 'Nothing more in this lesson' : 'Back to the overview'));
      nb.textContent = 'Overview'; nb.addEventListener('click', function () { go(-1); });
      if (single) nin.hidden = true;
    }
    nin.appendChild(ntext); nin.appendChild(nb); nx.appendChild(nin); sec.appendChild(nx);
    main.appendChild(sec);
    secNodes.push({ sec: sec, h: h, fill: fill, text: mt });

    var item = el('li'); var mb = el('button'); mb.type = 'button';
    mb.appendChild(el('span', 'wl-mnum', String(si + 1))); mb.appendChild(el('span', 'wl-mtitle', s.title)); mb.appendChild(el('span', 'wl-mstate'));
    mb.addEventListener('click', function () { go(si); });
    item.appendChild(mb); menu.querySelector('ol').appendChild(item); menuItems.push(mb);

    var li = el('li'), card = el('a', 'wl-mcard'); card.href = '#/m/' + si;
    card.appendChild(el('span', 'wl-mnumber', 'Module ' + (si + 1)));
    card.appendChild(el('h3', null, s.title));
    card.appendChild(el('p', null, s.summary || ''));
    var row = el('div', 'wl-mfoot-row'), chip = el('span', 'wl-chip', 'Not started'), cnt = el('span');
    row.appendChild(chip); row.appendChild(cnt); card.appendChild(row);
    var ctrack = el('div', 'wl-bar-track'), cfill = el('div'); ctrack.appendChild(cfill); card.appendChild(ctrack);
    card.addEventListener('click', function (ev) { ev.preventDefault(); go(si); });
    li.appendChild(card); document.getElementById('cards').appendChild(li);
    cards.push({ chip: chip, cnt: cnt, fill: cfill, card: card });
  });
  if (single || !L.show_menu) { menu.hidden = true; menuBtn.hidden = true; }
  if (single) { document.getElementById('back-btn').hidden = true; }
  function setMenu(open) {
    menu.classList.toggle('open', open); menuBtn.setAttribute('aria-expanded', String(open));
    scrim.hidden = !(open && window.matchMedia && window.matchMedia('(max-width:1000px)').matches);
  }
  menuBtn.addEventListener('click', function () { setMenu(!menu.classList.contains('open')); });
  scrim.addEventListener('click', function () { setMenu(false); });
  document.getElementById('menu-home').addEventListener('click', function () { go(-1); });
  document.getElementById('back-btn').addEventListener('click', function () { go(-1); });
  if (single) document.getElementById('menu-home').hidden = true;
  document.addEventListener('keydown', function (ev) { if (ev.key === 'Escape' && menu.classList.contains('open')) { setMenu(false); menuBtn.focus(); } });

  // ---- routing: #/ is the title page, #/m/N is module N (so the back button works) -----------------
  function go(si) {
    var h = si < 0 ? '#/' : '#/m/' + si;
    if (location.hash !== h) { try { history.pushState(null, '', h); } catch (e) { location.hash = h; } }
    show(si);
  }
  function route() {
    var m = /^#\/m\/(\d+)$/.exec(location.hash), si = m ? parseInt(m[1], 10) : -1;
    if (single) si = 0;
    if (si >= L.sections.length) si = -1;
    show(si);
  }
  function show(si) {
    if (single) si = 0;
    var changed = si !== cur;
    cur = si;
    home.hidden = si >= 0; view.hidden = si < 0;
    secNodes.forEach(function (n, i) { n.sec.hidden = i !== si; });
    setMenu(false);
    if (si >= 0) {
      section = si;
      if (!io) blocks.forEach(function (b) { if (b.si === si && !b.node.hidden) viewed[b.id] = true; });
      saveSoon();
    }
    if (changed) {
      document.documentElement.scrollTop = 0; document.body.scrollTop = 0;
      var target = si >= 0 ? secNodes[si].h : document.getElementById('hero-title');
      target.focus({ preventScroll: true });
    }
    update();
  }
  window.addEventListener('hashchange', route);

  // ---- gates: inside a module, everything after an unopened Continue block stays hidden ------------
  function applyGates() {
    var locked = false, lastSi = -1;
    blocks.forEach(function (b) {
      if (b.si !== lastSi) { lastSi = b.si; locked = false; }
      b.node.hidden = locked;
      if (!locked && io) io.observe(b.node);
      if (b.key === 'Continue') {
        var btn = b.node.querySelector('button');
        if (opened[b.id]) { b.node.hidden = true; }
        else locked = true;
        if (btn) btn.disabled = !!opened[b.id];
      }
    });
  }
  function openGate(id) {
    opened[id] = true;
    var idx = blocks.findIndex(function (b) { return b.id === id; });
    applyGates();
    if (!io) blocks.forEach(function (b) { if (b.si === cur && !b.node.hidden) viewed[b.id] = true; });
    var next = blocks.slice(idx + 1).find(function (b) { return !b.node.hidden; });
    if (next) { next.node.tabIndex = -1; next.node.focus({ preventScroll: true }); next.node.scrollIntoView({ behavior: reduceMotion ? 'auto' : 'smooth', block: 'nearest' }); }
    update(); saveSoon();
  }
  function markViewed(id) {
    if (!id || viewed[id]) return;
    viewed[id] = true;
    var b = blocks.find(function (x) { return x.id === id; });
    if (b) section = b.si;
    update(); saveSoon();
  }

  // ---- scoring, progress and completion ---------------------------------------------------------------
  var checks = blocks.filter(function (b) { return b.key === 'KnowledgeCheck'; });
  function totals() {
    var max = 0, earned = 0;
    checks.forEach(function (b) { max += b.data.points; if (answers[b.id]) earned += answers[b.id].e; });
    var pct = max > 0 ? Math.round(earned / max * 100) : 0;
    return { earned: earned, max: max, pct: pct, answered: checks.filter(function (b) { return answers[b.id]; }).length };
  }
  function status() {
    var t = totals();
    var allViewed = blocks.every(function (b) { return viewed[b.id]; });
    var checksOk = t.answered === checks.length && (t.max === 0 || t.pct >= P.pass_mark);
    var rule = L.completion;
    var complete = rule === 'AllViewed' ? allViewed : rule === 'ChecksPassed' ? checksOk : (allViewed && checksOk);
    return { t: t, allViewed: allViewed, checksOk: checksOk, complete: complete, viewedCount: blocks.filter(function (b) { return viewed[b.id]; }).length };
  }
  function modStats(si) {
    var mine = blocks.filter(function (b) { return b.si === si; });
    var seen = mine.filter(function (b) { return viewed[b.id]; }).length;
    return { total: mine.length, seen: seen, pct: mine.length ? Math.round(seen / mine.length * 100) : 100 };
  }
  function update() {
    var s = status();
    var pct = blocks.length ? Math.round(s.viewedCount / blocks.length * 100) : 100;
    document.getElementById('home-bar').style.width = pct + '%';
    document.getElementById('home-bar-text').textContent = pct + '%';
    document.getElementById('home-bar-box').setAttribute('aria-valuenow', String(pct));
    var stats = L.sections.map(function (_, si) { return modStats(si); });
    stats.forEach(function (m, si) {
      var state = m.pct >= 100 ? 'done' : m.seen > 0 ? 'progress' : 'new';
      var c = cards[si];
      c.chip.textContent = state === 'done' ? '✓ Complete' : state === 'progress' ? 'In progress' : 'Not started';
      c.chip.dataset.state = state;
      c.cnt.textContent = m.seen + ' of ' + m.total + ' viewed';
      c.fill.style.width = m.pct + '%';
      var mb = menuItems[si];
      mb.classList.toggle('done', state === 'done');
      mb.querySelector('.wl-mstate').textContent = state === 'done' ? '✓' : state === 'progress' ? m.pct + '%' : '';
      if (si === cur) mb.setAttribute('aria-current', 'true'); else mb.removeAttribute('aria-current');
      secNodes[si].fill.style.width = m.pct + '%'; secNodes[si].text.textContent = m.pct + '%';
    });
    if (cur >= 0) {
      bar.style.width = stats[cur].pct + '%'; barText.textContent = stats[cur].pct + '%';
      document.getElementById('bar-box').setAttribute('aria-valuenow', String(stats[cur].pct));
    }
    var next = stats.findIndex(function (m, i) { return i === section && m.pct < 100; });
    if (next < 0) next = stats.findIndex(function (m) { return m.pct < 100; });
    var started = s.viewedCount > 0;
    startBtn.textContent = s.complete ? 'Review the lesson' : started ? 'Continue' : 'Start the lesson';
    startBtn.onclick = function () { go(s.complete || next < 0 ? 0 : next); };
    document.getElementById('hero-meta').textContent = L.sections.length + (L.sections.length === 1 ? ' module' : ' modules') + (checks.length ? ' · ' + checks.length + (checks.length === 1 ? ' knowledge check' : ' knowledge checks') : '');
    done.hidden = !s.complete;
    if (s.complete) {
      done.querySelector('.wl-done-score').textContent = s.t.max > 0 ? 'Your score: ' + s.t.pct + '% (' + s.t.earned + ' of ' + s.t.max + ' points).' : '';
      if (lms && !reported) {
        reported = true;
        if (s.t.max > 0) lms.score(s.t.pct, s.t.pct >= P.pass_mark); else lms.complete();
        lms.commit();
      }
    }
  }

  // ---- bookmark and suspend data ------------------------------------------------------------------------
  var saveTimer = null;
  function saveSoon() { if (!lms) return; clearTimeout(saveTimer); saveTimer = setTimeout(saveNow, 500); }
  function saveNow() {
    if (!lms) return;
    var out = {};
    P.variables.forEach(function (v) { if (!v.built_in && vars[v.id] !== unwrap(v.initial)) out[v.id] = vars[v.id]; });
    var data = JSON.stringify({ v: 3, vars: out, ans: answers, view: Object.keys(viewed), open: Object.keys(opened) });
    if (data.length > lms.limit) data = JSON.stringify({ v: 3, vars: out, ans: {}, view: Object.keys(viewed).length === blocks.length ? ['*'] : [], open: Object.keys(opened) });
    if (data.length > lms.limit) return;
    lms.set(lms.suspendKey, data);
    lms.set(lms.locationKey, section);
    lms.commit();
  }
  function restore() {
    var raw = lms.get(lms.suspendKey);
    if (raw) {
      try {
        var d = JSON.parse(raw);
        if (d && d.v === 3) {
          Object.keys(d.vars || {}).forEach(function (id) { if (id in vars && typeof d.vars[id] === typeof vars[id]) vars[id] = d.vars[id]; });
          answers = d.ans || {};
          (d.view || []).forEach(function (k) { if (k === '*') blocks.forEach(function (b) { viewed[b.id] = true; }); else viewed[k] = true; });
          (d.open || []).forEach(function (k) { opened[k] = true; });
        }
      } catch (e) { console.warn('BrainBend Studio: ignoring unreadable suspend data'); }
    }
    var loc = parseInt(lms.get(lms.locationKey), 10);
    return isNaN(loc) ? 0 : loc;
  }

  // ---- start ----------------------------------------------------------------------------------------------
  if (lms && lms.init()) {
    var startSection = restore();
    if (startSection > 0 && startSection < L.sections.length) section = startSection;
    lms.incomplete();
    window.addEventListener('pagehide', function () { saveNow(); lms.finish(); });
    window.addEventListener('beforeunload', function () { saveNow(); lms.finish(); });
  }
  // Answers saved earlier are shown locked: rebuild the checks now that they are known.
  blocks.forEach(function (b, i) {
    if (b.key === 'KnowledgeCheck' && answers[b.id]) {
      var fresh = render({ id: b.id, kind: { KnowledgeCheck: b.data } }, i);
      b.node.replaceChild(fresh, b.node.firstChild);
    }
  });
  applyGates();
  route();
  window.GetPlayer = function () {
    return { GetVar: function (n) { return vars[n]; }, SetVar: function (n, v) { setVar(n, v); } };
  };
})();
