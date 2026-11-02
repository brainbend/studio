  function normText(t, caseSensitive) {
    var x = String(t).trim().replace(/\s+/g, ' ');
    return caseSensitive ? x : x.toLowerCase();
  }
  // A written mark after a field that cannot carry a generated one, so right and wrong are not told by colour alone.
  function markAfter(field, ok) {
    var m = el('span', 'q-mark ' + (ok ? 'right' : 'wrong'), ok ? '\u2713 correct' : '\u2717 incorrect');
    field.parentNode.insertBefore(m, field.nextSibling);
  }
  // A fixed, repeatable shuffle (so reloads and resumes show the same order); never the answer order.
  function shuffled(n, seed) {
    var idx = [], i;
    for (i = 0; i < n; i++) idx.push(i);
    var k = 0;
    for (i = 0; i < seed.length; i++) k = (k * 31 + seed.charCodeAt(i)) >>> 0;
    for (i = n - 1; i > 0; i--) { k = (k * 1103515245 + 12345) >>> 0; var j = k % (i + 1); var t = idx[i]; idx[i] = idx[j]; idx[j] = t; }
    var sorted = idx.every(function (v, p) { return v === p; });
    if (sorted && n > 1) idx.push(idx.shift());
    return idx;
  }

  function choiceStyle(q, si) {
    var form = el('fieldset', 'q-choices');
    var type = q.multiple ? 'checkbox' : 'radio';
    var inputs = q.choices.map(function (c) {
      var row = el('label', 'choice');
      var inp = el('input'); inp.type = type; inp.name = 'q' + si; inp.value = c.id;
      row.appendChild(inp); row.appendChild(el('span', 'choice-text', c.text));
      form.appendChild(row);
      return { inp: inp, row: row, c: c };
    });
    return {
      node: form,
      collect: function () {
        var chosen = inputs.filter(function (x) { return x.inp.checked; }).map(function (x) { return x.c.id; });
        return chosen.length ? chosen : null;
      },
      correct: function (tokens) {
        var want = q.choices.filter(function (c) { return c.correct; }).map(function (c) { return c.id; }).sort().join(',');
        return tokens.slice().sort().join(',') === want;
      },
      lock: function (rec) {
        inputs.forEach(function (x) {
          x.inp.disabled = true;
          x.inp.checked = rec.s.indexOf(x.c.id) >= 0;
          x.row.classList.toggle('right', x.c.correct);
          x.row.classList.toggle('wrong', rec.s.indexOf(x.c.id) >= 0 && !x.c.correct);
        });
      },
      focusFirst: function () { if (inputs[0]) inputs[0].inp.focus(); }
    };
  }

  function textStyle(q, cfg, si) {
    var row = el('label', 'choice');
    var inp = el('input', 'q-text'); inp.type = 'text'; inp.autocomplete = 'off'; inp.spellcheck = false;
    inp.setAttribute('aria-label', 'Your answer');
    row.appendChild(inp);
    return {
      node: row,
      collect: function () { return inp.value.trim() ? [inp.value] : null; },
      correct: function (tokens) {
        return cfg.accepted.some(function (a) { return normText(a, cfg.case_sensitive) === normText(tokens[0], cfg.case_sensitive); });
      },
      lock: function (rec) {
        inp.value = rec.s[0]; inp.disabled = true;
        row.classList.toggle('right', rec.c); row.classList.toggle('wrong', !rec.c);
      },
      focusFirst: function () { inp.focus(); }
    };
  }

  function matchingStyle(q, cfg, si) {
    var wrap = el('div', 'q-match');
    var rights = cfg.pairs.map(function (p) { return p[1]; });
    var order = shuffled(rights.length, 'm' + si);
    var rows = cfg.pairs.map(function (p, i) {
      var row = el('div', 'choice');
      var lab = el('span', 'choice-text', p[0]); lab.id = 'm' + si + '_' + i;
      var sel = el('select', 'q-select'); sel.setAttribute('aria-labelledby', lab.id);
      var none = el('option', null, 'Choose…'); none.value = ''; sel.appendChild(none);
      order.forEach(function (k) { var o = el('option', null, rights[k]); o.value = rights[k]; sel.appendChild(o); });
      row.appendChild(lab); row.appendChild(sel); wrap.appendChild(row);
      return { sel: sel, row: row, want: p[1] };
    });
    return {
      node: wrap,
      collect: function () {
        var v = rows.map(function (r) { return r.sel.value; });
        return v.every(function (x) { return x !== ''; }) ? v : null;
      },
      correct: function (tokens) { return rows.every(function (r, i) { return tokens[i] === r.want; }); },
      lock: function (rec) {
        rows.forEach(function (r, i) {
          r.sel.value = rec.s[i]; r.sel.disabled = true;
          r.row.classList.toggle('right', rec.s[i] === r.want);
          r.row.classList.toggle('wrong', rec.s[i] !== r.want);
        });
      },
      focusFirst: function () { if (rows[0]) rows[0].sel.focus(); }
    };
  }

  function sequenceStyle(q, cfg, si) {
    var list = el('ol', 'q-seq');
    var items = cfg.items;
    var order = shuffled(items.length, 's' + si);   // order[position] = original index
    var locked = false;
    function render(focusPos) {
      list.textContent = '';
      order.forEach(function (orig, pos) {
        var li = el('li', 'choice');
        li.appendChild(el('span', 'choice-text', items[orig]));
        if (!locked) {
          var up = el('button', 'q-move', '▲'); up.type = 'button'; up.setAttribute('aria-label', 'Move "' + items[orig] + '" up');
          var dn = el('button', 'q-move', '▼'); dn.type = 'button'; dn.setAttribute('aria-label', 'Move "' + items[orig] + '" down');
          up.disabled = pos === 0; dn.disabled = pos === order.length - 1;
          up.addEventListener('click', function () { swap(pos, pos - 1, 'up'); });
          dn.addEventListener('click', function () { swap(pos, pos + 1, 'down'); });
          li.appendChild(up); li.appendChild(dn);
          li.dataset.pos = String(pos);
        }
        list.appendChild(li);
      });
      if (focusPos !== undefined && !locked) {
        var b = list.children[focusPos.pos].querySelectorAll('button')[focusPos.which === 'up' ? 0 : 1];
        if (b && !b.disabled) b.focus(); else if (b) list.children[focusPos.pos].querySelectorAll('button')[focusPos.which === 'up' ? 1 : 0].focus();
      }
    }
    function swap(a, b, which) {
      var t = order[a]; order[a] = order[b]; order[b] = t;
      render({ pos: b, which: which });
    }
    render();
    return {
      node: list,
      collect: function () { return order.map(String); },
      correct: function (tokens) { return tokens.every(function (t, i) { return t === String(i); }); },
      lock: function (rec) {
        locked = true;
        order = rec.s.map(function (t) { return parseInt(t, 10); });
        render();
        Array.prototype.forEach.call(list.children, function (li, pos) {
          li.classList.toggle('right', order[pos] === pos);
          li.classList.toggle('wrong', order[pos] !== pos);
        });
      },
      focusFirst: function () { var b = list.querySelector('button:not([disabled])'); if (b) b.focus(); }
    };
  }


  // ---- passage questions: `*answer*` marks a blank (or a word to mark); `*a/b*` accepts either ------
  function parsePassage(text) {
    var out = [], plain = '', i = 0;
    while (i < text.length) {
      if (text.charAt(i) === '*') {
        var j = i + 1;
        while (j < text.length && text.charAt(j) !== '*' && text.charAt(j) !== '\n') j++;
        if (j < text.length && text.charAt(j) === '*') {
          var alts = text.slice(i + 1, j).split('/').map(function (a) { return a.trim(); }).filter(function (a) { return a; });
          if (alts.length) {
            if (plain) { out.push({ t: plain }); plain = ''; }
            out.push({ b: alts });
            i = j + 1;
            continue;
          }
        }
      }
      plain += text.charAt(i);
      i++;
    }
    if (plain) out.push({ t: plain });
    return out;
  }
  function matchesAny(alts, value, caseSensitive) {
    return alts.some(function (a) { return normText(a, caseSensitive) === normText(value, caseSensitive); });
  }

  function fillBlanksStyle(q, cfg, si) {
    var p = el('p', 'q-passage');
    var parts = parsePassage(cfg.text), blanks = parts.filter(function (x) { return x.b; });
    var inputs = [], n = 0;
    parts.forEach(function (x) {
      if (x.t !== undefined) { p.appendChild(document.createTextNode(x.t)); return; }
      var inp = el('input', 'q-blank'); inp.type = 'text'; inp.autocomplete = 'off'; inp.spellcheck = false;
      n++;
      inp.setAttribute('aria-label', 'Blank ' + n + ' of ' + blanks.length);
      var longest = x.b.reduce(function (m, a) { return Math.max(m, a.length); }, 4);
      inp.style.width = (Math.min(longest, 24) + 2) + 'ch';
      p.appendChild(inp);
      inputs.push({ inp: inp, alts: x.b });
    });
    return {
      node: p,
      collect: function () {
        var v = inputs.map(function (x) { return x.inp.value; });
        return v.every(function (t) { return t.trim() !== ''; }) ? v : null;
      },
      correct: function (tokens) { return inputs.every(function (x, i) { return matchesAny(x.alts, tokens[i], cfg.case_sensitive); }); },
      lock: function (rec) {
        inputs.forEach(function (x, i) {
          x.inp.value = rec.s[i]; x.inp.disabled = true;
          var ok = matchesAny(x.alts, rec.s[i], cfg.case_sensitive);
          x.inp.classList.toggle('right', ok); x.inp.classList.toggle('wrong', !ok);
          markAfter(x.inp, ok);
        });
      },
      focusFirst: function () { if (inputs[0]) inputs[0].inp.focus(); }
    };
  }

  // Click a word, then click a blank (or drag the word onto it). Click a filled blank to take the word back.
  function dragWordsStyle(q, cfg, si) {
    var parts = parsePassage(cfg.text), blanks = parts.filter(function (x) { return x.b; });
    var words = blanks.map(function (b) { return b.b[0]; }).concat((cfg.distractors || []).filter(function (d) { return d.trim(); }));
    var order = shuffled(words.length, 'd' + si);
    var placed = blanks.map(function () { return -1; });
    var selected = -1, locked = false, verdict = null;
    var wrap = el('div', 'q-drag');
    function place(slot, wi) {
      if (locked || wi < 0) return;
      placed[slot] = wi; selected = -1; render();
    }
    function render() {
      wrap.textContent = '';
      var p = el('p', 'q-passage'), n = 0;
      parts.forEach(function (x) {
        if (x.t !== undefined) { p.appendChild(document.createTextNode(x.t)); return; }
        var slot = n++;
        var b = el('button', 'q-slot' + (verdict ? (verdict[slot] ? ' right' : ' wrong') : ''), placed[slot] >= 0 ? words[placed[slot]] : '      ');
        b.type = 'button';
        b.setAttribute('aria-label', 'Blank ' + (slot + 1) + (placed[slot] >= 0 ? ': ' + words[placed[slot]] + '. Activate to take the word back.' : ': empty'));
        b.disabled = locked;
        b.addEventListener('click', function () {
          if (selected >= 0) place(slot, selected);
          else if (placed[slot] >= 0) { placed[slot] = -1; render(); }
        });
        b.addEventListener('dragover', function (ev) { ev.preventDefault(); });
        b.addEventListener('drop', function (ev) {
          ev.preventDefault();
          var wi = parseInt(ev.dataTransfer && ev.dataTransfer.getData('text/plain'), 10);
          if (!isNaN(wi) && placed.indexOf(wi) < 0) place(slot, wi);
        });
        p.appendChild(b);
      });
      var bank = el('div', 'q-bank'); bank.setAttribute('role', 'group'); bank.setAttribute('aria-label', 'Word bank');
      order.forEach(function (wi) {
        var used = placed.indexOf(wi) >= 0;
        var c = el('button', 'q-chip' + (selected === wi ? ' on' : ''), words[wi]); c.type = 'button';
        c.setAttribute('aria-pressed', String(selected === wi));
        c.disabled = used || locked;
        c.draggable = !used && !locked;
        c.addEventListener('dragstart', function (ev) { if (ev.dataTransfer) ev.dataTransfer.setData('text/plain', String(wi)); });
        c.addEventListener('click', function () { selected = selected === wi ? -1 : wi; render(); });
        bank.appendChild(c);
      });
      wrap.appendChild(p); wrap.appendChild(bank);
    }
    render();
    return {
      node: wrap,
      collect: function () { return placed.every(function (w) { return w >= 0; }) ? placed.map(function (w) { return words[w]; }) : null; },
      correct: function (tokens) { return blanks.every(function (b, i) { return matchesAny(b.b, tokens[i], false); }); },
      lock: function (rec) {
        locked = true; selected = -1;
        var taken = [];
        placed = rec.s.map(function (t) {
          for (var i = 0; i < words.length; i++) if (words[i] === t && taken.indexOf(i) < 0) { taken.push(i); return i; }
          return -1;
        });
        verdict = blanks.map(function (b, i) { return matchesAny(b.b, rec.s[i], false); });
        render();
      },
      focusFirst: function () { var c = wrap.querySelector('.q-chip:not([disabled])'); if (c) c.focus(); }
    };
  }

  function markWordsStyle(q, cfg, si) {
    var p = el('p', 'q-passage');
    var words = [];
    function addWords(text, correct) {
      text.split(/(\s+)/).forEach(function (piece) {
        if (!piece) return;
        if (/^\s+$/.test(piece)) { p.appendChild(document.createTextNode(piece)); return; }
        var b = el('button', 'q-word', piece); b.type = 'button'; b.setAttribute('aria-pressed', 'false');
        var w = { b: b, correct: correct, on: false, i: words.length };
        b.addEventListener('click', function () { if (b.disabled) return; w.on = !w.on; b.setAttribute('aria-pressed', String(w.on)); b.classList.toggle('on', w.on); });
        words.push(w); p.appendChild(b);
      });
    }
    parsePassage(cfg.text).forEach(function (x) { if (x.t !== undefined) addWords(x.t, false); else addWords(x.b[0], true); });
    return {
      node: p,
      collect: function () {
        var on = words.filter(function (w) { return w.on; }).map(function (w) { return String(w.i); });
        return on.length ? on : null;
      },
      correct: function (tokens) {
        return words.every(function (w) { return w.correct === (tokens.indexOf(String(w.i)) >= 0); });
      },
      lock: function (rec) {
        words.forEach(function (w) {
          var on = rec.s.indexOf(String(w.i)) >= 0;
          w.on = on; w.b.disabled = true; w.b.setAttribute('aria-pressed', String(on));
          w.b.classList.toggle('on', on);
          w.b.classList.toggle('right', on && w.correct);
          w.b.classList.toggle('wrong', on && !w.correct);
          w.b.classList.toggle('missed', !on && w.correct);
        });
      },
      focusFirst: function () { if (words[0]) words[0].b.focus(); }
    };
  }

  function numericStyle(q, cfg, si) {
    var row = el('label', 'choice');
    var inp = el('input', 'q-text'); inp.type = 'text'; inp.autocomplete = 'off'; inp.spellcheck = false;
    inp.setAttribute('inputmode', 'decimal'); inp.setAttribute('aria-label', 'Your answer (a number)');
    row.appendChild(inp);
    function num(t) { return Number(String(t).trim().replace(',', '.')); }
    return {
      node: row,
      collect: function () { return inp.value.trim() !== '' && !isNaN(num(inp.value)) ? [inp.value] : null; },
      correct: function (tokens) { return Math.abs(num(tokens[0]) - cfg.answer) <= cfg.tolerance + 1e-9; },
      lock: function (rec) {
        inp.value = rec.s[0]; inp.disabled = true;
        row.classList.toggle('right', rec.c); row.classList.toggle('wrong', !rec.c);
      },
      focusFirst: function () { inp.focus(); }
    };
  }

  // Click an item, then click the group it belongs to (or drag it there). Click a sorted item to take it back.
  function categorizeStyle(q, cfg, si) {
    var items = [];
    cfg.categories.forEach(function (c, ci) { c[1].forEach(function (t) { items.push({ text: t, cat: ci }); }); });
    var order = shuffled(items.length, 'c' + si);
    var placed = items.map(function () { return -1; });
    var selected = -1, locked = false, verdict = null;
    var wrap = el('div', 'q-sort');
    function put(cat, ii) {
      if (locked || ii < 0) return;
      placed[ii] = cat; selected = -1; render();
    }
    function chip(ii, onclick) {
      var c = el('button', 'q-chip' + (selected === ii ? ' on' : '') + (verdict ? (verdict[ii] ? ' right' : ' wrong') : ''), items[ii].text);
      c.type = 'button'; c.disabled = locked && !verdict;
      if (!verdict) c.setAttribute('aria-pressed', String(selected === ii));
      c.draggable = !locked;
      c.addEventListener('dragstart', function (ev) { if (ev.dataTransfer) ev.dataTransfer.setData('text/plain', String(ii)); });
      c.addEventListener('click', function () { if (!locked) onclick(); });
      if (locked) c.disabled = true;
      return c;
    }
    function render() {
      wrap.textContent = '';
      var pool = el('div', 'q-bank'); pool.setAttribute('role', 'group'); pool.setAttribute('aria-label', 'Items to sort');
      order.forEach(function (ii) {
        if (placed[ii] >= 0) return;
        pool.appendChild(chip(ii, function () { selected = selected === ii ? -1 : ii; render(); }));
      });
      if (!pool.children.length) pool.appendChild(el('span', 'q-note', 'Every item is sorted.'));
      wrap.appendChild(pool);
      var cats = el('div', 'q-cats');
      cfg.categories.forEach(function (c, ci) {
        var box = el('div', 'q-cat'); box.setAttribute('role', 'group'); box.setAttribute('aria-label', c[0]);
        var head = el('button', 'q-cat-head', c[0]); head.type = 'button'; head.disabled = locked;
        head.setAttribute('aria-label', 'Put the chosen item in ' + c[0]);
        head.addEventListener('click', function () { if (selected >= 0) put(ci, selected); });
        box.appendChild(head);
        order.forEach(function (ii) {
          if (placed[ii] !== ci) return;
          box.appendChild(chip(ii, function () { placed[ii] = -1; render(); }));
        });
        box.addEventListener('dragover', function (ev) { ev.preventDefault(); });
        box.addEventListener('drop', function (ev) {
          ev.preventDefault();
          var ii = parseInt(ev.dataTransfer && ev.dataTransfer.getData('text/plain'), 10);
          if (!isNaN(ii)) put(ci, ii);
        });
        cats.appendChild(box);
      });
      wrap.appendChild(cats);
    }
    render();
    return {
      node: wrap,
      collect: function () { return placed.every(function (c) { return c >= 0; }) ? placed.map(String) : null; },
      correct: function (tokens) { return items.every(function (it, i) { return tokens[i] === String(it.cat); }); },
      lock: function (rec) {
        locked = true; selected = -1;
        placed = rec.s.map(function (t) { return parseInt(t, 10); });
        verdict = items.map(function (it, i) { return placed[i] === it.cat; });
        render();
      },
      focusFirst: function () { var c = wrap.querySelector('.q-chip:not([disabled])'); if (c) c.focus(); }
    };
  }


  // Click the picture (or move the marker with the arrow keys); correct when the point is inside a region.
  function hotspotStyle(q, cfg, si) {
    var box = el('div', 'q-hot');
    var wrap = el('div', 'q-hot-pic'); wrap.tabIndex = 0;
    wrap.setAttribute('role', 'group');
    wrap.setAttribute('aria-label', (cfg.alt ? cfg.alt + '. ' : '') + 'Click the picture, or use the arrow keys to move the marker, then press Submit.');
    var img = el('img'); img.alt = ''; img.draggable = false;
    if (cfg.asset && P.assetUrls[cfg.asset]) img.src = P.assetUrls[cfg.asset];
    var pin = el('span', 'q-pin'); pin.style.display = 'none';
    var said = el('span', 'sr-only'); said.setAttribute('role', 'status');
    wrap.appendChild(img); wrap.appendChild(pin);
    box.appendChild(wrap); box.appendChild(said);
    var at = null, locked = false;
    function inside(pt) { return cfg.regions.some(function (r) { return hotInside(r, pt[0], pt[1]); }); }
    function show() {
      pin.style.display = at ? 'block' : 'none';
      if (at) {
        pin.style.left = (Math.round(at[0] * 1000) / 10) + '%'; pin.style.top = (Math.round(at[1] * 1000) / 10) + '%';
        said.textContent = 'Marker at ' + Math.round(at[0] * 100) + ' percent across and ' + Math.round(at[1] * 100) + ' percent down.';
      }
    }
    wrap.addEventListener('click', function (ev) {
      if (locked) return;
      var r = img.getBoundingClientRect();
      if (!r.width || !r.height) return;
      at = [Math.min(1, Math.max(0, (ev.clientX - r.left) / r.width)), Math.min(1, Math.max(0, (ev.clientY - r.top) / r.height))];
      show();
    });
    wrap.addEventListener('keydown', function (ev) {
      if (locked) return;
      var d = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[ev.key];
      if (!d) return;
      ev.preventDefault();
      var step = ev.shiftKey ? 0.1 : 0.02;
      at = at || [0.5, 0.5];
      at = [Math.min(1, Math.max(0, at[0] + d[0] * step)), Math.min(1, Math.max(0, at[1] + d[1] * step))];
      show();
    });
    return {
      node: box,
      collect: function () { return at ? [String(at[0]), String(at[1])] : null; },
      correct: function (tokens) { return inside([Number(tokens[0]), Number(tokens[1])]); },
      lock: function (rec) {
        locked = true;
        at = [Number(rec.s[0]), Number(rec.s[1])];
        show();
        pin.classList.add(rec.c ? 'right' : 'wrong');
        cfg.regions.forEach(function (r) {
          var a = el('span', 'q-area');
          a.style.left = (r.x * 100) + '%'; a.style.top = (r.y * 100) + '%'; a.style.width = (r.w * 100) + '%'; a.style.height = (r.h * 100) + '%';
          if (r.ellipse) a.style.borderRadius = '50%';
          wrap.appendChild(a);
        });
      },
      focusFirst: function () { wrap.focus(); }
    };
  }
  function hotInside(r, px, py) {
    if (r.ellipse) {
      var rx = r.w / 2, ry = r.h / 2;
      if (rx <= 0 || ry <= 0) return false;
      var dx = (px - r.x - rx) / rx, dy = (py - r.y - ry) / ry;
      return dx * dx + dy * dy <= 1;
    }
    return px >= r.x && px <= r.x + r.w && py >= r.y && py <= r.y + r.h;
  }


  // ---- survey and free text (not graded), dictation, drop-down passages, labelling a picture ----------
  function likertStyle(q, cfg, si) {
    var table = el('table', 'q-likert');
    var head = el('tr'); var corner = el('th', null, 'Statement'); corner.scope = 'col'; corner.className = 'sr-only-cell'; head.appendChild(corner);
    cfg.scale.forEach(function (c) { var th = el('th', null, c); th.scope = 'col'; head.appendChild(th); });
    table.appendChild(head);
    var rows = cfg.statements.map(function (t, r) {
      var tr = el('tr'); var th = el('th', 'q-statement', t); th.scope = 'row'; tr.appendChild(th);
      var radios = cfg.scale.map(function (c, k) {
        var td = el('td'); var inp = el('input'); inp.type = 'radio'; inp.name = 'l' + si + '_' + r; inp.value = String(k);
        inp.setAttribute('aria-label', t + ': ' + c);
        td.appendChild(inp); tr.appendChild(td); return inp;
      });
      table.appendChild(tr);
      return radios;
    });
    return {
      node: table,
      collect: function () {
        var v = rows.map(function (rs) { var on = rs.filter(function (x) { return x.checked; })[0]; return on ? on.value : null; });
        return v.every(function (x) { return x !== null; }) ? v : null;
      },
      correct: function () { return true; },
      lock: function (rec) { rows.forEach(function (rs, r) { rs.forEach(function (x) { x.disabled = true; x.checked = x.value === rec.s[r]; }); }); },
      focusFirst: function () { if (rows[0] && rows[0][0]) rows[0][0].focus(); }
    };
  }

  function wordCount(t) { var m = String(t).trim().match(/\S+/g); return m ? m.length : 0; }
  function reflectionStyle(q, cfg, si) {
    var ta = el('textarea', 'q-essay'); ta.rows = 6; ta.setAttribute('aria-label', 'Your answer');
    return {
      node: ta,
      collect: function () { return wordCount(ta.value) >= Math.max(1, cfg.min_words || 0) ? [ta.value] : null; },
      correct: function () { return true; },
      lock: function (rec) { ta.value = rec.s[0]; ta.disabled = true; },
      focusFirst: function () { ta.focus(); }
    };
  }

  function dictNorm(t) {
    return String(t).toLowerCase().replace(/[^\p{L}\p{N}\s]/gu, '').replace(/\s+/g, ' ').trim();
  }
  function dictationStyle(q, cfg, si) {
    var box = el('div', 'q-dict');
    var audio = el('audio'); audio.controls = true; audio.preload = 'metadata';
    audio.setAttribute('aria-label', 'Audio clip');
    if (cfg.asset && P.assetUrls[cfg.asset]) audio.src = P.assetUrls[cfg.asset];
    var row = el('label', 'choice');
    var inp = el('input', 'q-text'); inp.type = 'text'; inp.autocomplete = 'off'; inp.spellcheck = false;
    inp.setAttribute('aria-label', 'Type what you hear');
    row.appendChild(inp);
    box.appendChild(audio);
    var limit = cfg.max_plays | 0;
    if (limit > 0) {
      var left = el('div', 'q-plays'); left.setAttribute('aria-live', 'polite');
      var plays = 0, started = false, spent = false;
      var show = function () { left.textContent = spent ? 'No plays left.' : 'Plays left: ' + (limit - plays); };
      audio.addEventListener('play', function () {
        if (spent) { audio.pause(); return; }
        if (!started) { started = true; plays++; show(); }
      });
      audio.addEventListener('ended', function () {
        started = false;
        if (plays >= limit) { spent = true; audio.controls = false; audio.removeAttribute('controls'); show(); }
      });
      show();
      box.appendChild(left);
    }
    box.appendChild(row);
    return {
      node: box,
      collect: function () { return inp.value.trim() ? [inp.value] : null; },
      correct: function (tokens) { return dictNorm(tokens[0]) === dictNorm(cfg.text); },
      lock: function (rec) { inp.value = rec.s[0]; inp.disabled = true; row.classList.toggle('right', rec.c); row.classList.toggle('wrong', !rec.c); },
      focusFirst: function () { inp.focus(); }
    };
  }

  function dropdownsStyle(q, cfg, si) {
    var p = el('p', 'q-passage');
    var selects = [], n = 0;
    parsePassage(cfg.text).forEach(function (x) {
      if (x.t !== undefined) { p.appendChild(document.createTextNode(x.t)); return; }
      n++;
      var sel = el('select', 'q-select q-inline'); sel.setAttribute('aria-label', 'Blank ' + n);
      var none = el('option', null, 'Choose…'); none.value = ''; sel.appendChild(none);
      shuffled(x.b.length, 'dd' + si + '_' + n).forEach(function (k) { var o = el('option', null, x.b[k]); o.value = x.b[k]; sel.appendChild(o); });
      p.appendChild(sel);
      selects.push({ sel: sel, right: x.b[0] });
    });
    return {
      node: p,
      collect: function () { var v = selects.map(function (x) { return x.sel.value; }); return v.every(function (t) { return t !== ''; }) ? v : null; },
      correct: function (tokens) { return selects.every(function (x, i) { return tokens[i] === x.right; }); },
      lock: function (rec) {
        selects.forEach(function (x, i) {
          x.sel.value = rec.s[i]; x.sel.disabled = true;
          var ok = rec.s[i] === x.right;
          x.sel.classList.toggle('right', ok); x.sel.classList.toggle('wrong', !ok);
          markAfter(x.sel, ok);
        });
      },
      focusFirst: function () { if (selects[0]) selects[0].sel.focus(); }
    };
  }

  // Pick a label, then the area it belongs to (or drag it there). Click a filled area to take the label back.
  function labelPictureStyle(q, cfg, si) {
    var labels = cfg.zones.map(function (z) { return z.label; }).concat((cfg.distractors || []).filter(function (d) { return d.trim(); }));
    var order = shuffled(labels.length, 'lp' + si);
    var placed = cfg.zones.map(function () { return -1; });
    var selected = -1, locked = false, verdict = null;
    var box = el('div', 'q-label');
    function put(zone, li) { if (locked || li < 0) return; placed[zone] = li; selected = -1; render(); }
    function render() {
      box.textContent = '';
      var pic = el('div', 'q-hot-pic');
      var img = el('img'); img.alt = cfg.alt || ''; img.draggable = false;
      if (cfg.asset && P.assetUrls[cfg.asset]) img.src = P.assetUrls[cfg.asset];
      pic.appendChild(img);
      cfg.zones.forEach(function (z, i) {
        var r = z.region;
        var b = el('button', 'q-zone' + (placed[i] >= 0 ? ' filled' : '') + (verdict ? (verdict[i] ? ' right' : ' wrong') : ''), placed[i] >= 0 ? labels[placed[i]] : String(i + 1));
        b.type = 'button'; b.disabled = locked;
        b.style.left = (r.x * 100) + '%'; b.style.top = (r.y * 100) + '%'; b.style.width = (r.w * 100) + '%'; b.style.height = (r.h * 100) + '%';
        if (r.ellipse) b.style.borderRadius = '50%';
        b.setAttribute('aria-label', 'Area ' + (i + 1) + (placed[i] >= 0 ? ': ' + labels[placed[i]] + '. Activate to take the label back.' : ': empty'));
        b.addEventListener('click', function () {
          if (selected >= 0) put(i, selected);
          else if (placed[i] >= 0 && !locked) { placed[i] = -1; render(); }
        });
        b.addEventListener('dragover', function (ev) { ev.preventDefault(); });
        b.addEventListener('drop', function (ev) {
          ev.preventDefault();
          var li = parseInt(ev.dataTransfer && ev.dataTransfer.getData('text/plain'), 10);
          if (!isNaN(li) && placed.indexOf(li) < 0) put(i, li);
        });
        pic.appendChild(b);
      });
      var bank = el('div', 'q-bank'); bank.setAttribute('role', 'group'); bank.setAttribute('aria-label', 'Labels');
      order.forEach(function (li) {
        var used = placed.indexOf(li) >= 0;
        var c = el('button', 'q-chip' + (selected === li ? ' on' : ''), labels[li]); c.type = 'button';
        c.setAttribute('aria-pressed', String(selected === li));
        c.disabled = used || locked; c.draggable = !used && !locked;
        c.addEventListener('dragstart', function (ev) { if (ev.dataTransfer) ev.dataTransfer.setData('text/plain', String(li)); });
        c.addEventListener('click', function () { selected = selected === li ? -1 : li; render(); });
        bank.appendChild(c);
      });
      box.appendChild(pic); box.appendChild(bank);
    }
    render();
    return {
      node: box,
      collect: function () { return placed.every(function (l) { return l >= 0; }) ? placed.map(function (l) { return labels[l]; }) : null; },
      correct: function (tokens) { return cfg.zones.every(function (z, i) { return tokens[i] === z.label; }); },
      lock: function (rec) {
        locked = true; selected = -1;
        var taken = [];
        placed = rec.s.map(function (t) {
          for (var i = 0; i < labels.length; i++) if (labels[i] === t && taken.indexOf(i) < 0) { taken.push(i); return i; }
          return -1;
        });
        verdict = cfg.zones.map(function (z, i) { return rec.s[i] === z.label; });
        render();
      },
      focusFirst: function () { var c = box.querySelector('.q-chip:not([disabled])'); if (c) c.focus(); }
    };
  }


  // Build one question's card. `id` keys the stored answer, `saved` is that answer (or undefined),
  // and `onAnswer(rec)` hears each new submission. Both players use this.
  function renderQuestion(q, si, id, saved, onAnswer) {
    var box = el('div', 'q');
    var prompt = el('h2', 'q-prompt', q.prompt);
    prompt.id = 'qp' + si;
    box.appendChild(prompt);
    var styleKey = typeof q.style === 'string' || !q.style ? 'Choice' : Object.keys(q.style)[0];
    var cfg = styleKey === 'Choice' ? null : q.style[styleKey];
    var ui = styleKey === 'TextEntry' ? textStyle(q, cfg, si)
      : styleKey === 'Matching' ? matchingStyle(q, cfg, si)
      : styleKey === 'Sequence' ? sequenceStyle(q, cfg, si)
      : styleKey === 'FillBlanks' ? fillBlanksStyle(q, cfg, si)
      : styleKey === 'DragWords' ? dragWordsStyle(q, cfg, si)
      : styleKey === 'MarkWords' ? markWordsStyle(q, cfg, si)
      : styleKey === 'Numeric' ? numericStyle(q, cfg, si)
      : styleKey === 'Categorize' ? categorizeStyle(q, cfg, si)
      : styleKey === 'Hotspot' ? hotspotStyle(q, cfg, si)
      : styleKey === 'Likert' ? likertStyle(q, cfg, si)
      : styleKey === 'Reflection' ? reflectionStyle(q, cfg, si)
      : styleKey === 'Dictation' ? dictationStyle(q, cfg, si)
      : styleKey === 'Dropdowns' ? dropdownsStyle(q, cfg, si)
      : styleKey === 'LabelPicture' ? labelPictureStyle(q, cfg, si)
      : choiceStyle(q, si);
    // A group named by the prompt; a single field (or the label around it) is described by it instead.
    if (/^(LABEL|INPUT|TEXTAREA|SELECT)$/.test(ui.node.tagName)) {
      var field = ui.node.matches('input,textarea,select') ? ui.node : ui.node.querySelector('input,textarea,select');
      if (field) field.setAttribute('aria-describedby', prompt.id);
    } else {
      ui.node.setAttribute('role', 'group');
      ui.node.setAttribute('aria-labelledby', prompt.id);
    }
    box.appendChild(ui.node);
    var feedback = el('div', 'q-feedback'); feedback.setAttribute('role', 'status'); feedback.setAttribute('aria-live', 'polite');
    var submit = el('button', 'q-submit', 'Submit'); submit.type = 'button';
    box.appendChild(submit); box.appendChild(feedback);

    function lock(rec) {
      ui.lock(rec);
      submit.style.display = 'none';
      var msg = rec.c ? q.feedback_correct : q.feedback_incorrect;
      if (!rec.c && styleKey === 'TextEntry' && cfg.accepted.length) msg += ' Accepted: ' + cfg.accepted.join(' / ');
      if (!rec.c && styleKey === 'Dictation') msg += ' It said: ' + cfg.text;
      if (!rec.c && styleKey === 'Numeric') msg += ' Answer: ' + cfg.answer + (cfg.tolerance ? ' (within ' + cfg.tolerance + ')' : '');
      if (!rec.c && styleKey === 'FillBlanks') msg += ' Answers: ' + parsePassage(cfg.text).filter(function (x) { return x.b; }).map(function (x) { return x.b.join(' / '); }).join(', ');
      feedback.textContent = msg;
      feedback.className = 'q-feedback ' + (rec.c ? 'ok' : 'bad');
    }
    submit.addEventListener('click', function () {
      var tokens = ui.collect();
      if (!tokens) {
        feedback.textContent = styleKey === 'Sequence' ? 'Put the items in order first.'
          : styleKey === 'Matching' ? 'Match every item first.'
          : styleKey === 'FillBlanks' || styleKey === 'DragWords' ? 'Fill in every blank first.'
          : styleKey === 'MarkWords' ? 'Mark at least one word first.'
          : styleKey === 'Numeric' ? 'Type a number first.'
          : styleKey === 'Categorize' ? 'Sort every item first.'
          : styleKey === 'Hotspot' ? 'Click the picture first.'
          : styleKey === 'Likert' ? 'Rate every statement first.'
          : styleKey === 'Reflection' ? 'Write at least ' + Math.max(1, cfg.min_words || 0) + (Math.max(1, cfg.min_words || 0) === 1 ? ' word' : ' words') + ' first.'
          : styleKey === 'Dictation' ? 'Type what you hear first.'
          : styleKey === 'Dropdowns' ? 'Choose an answer for every blank first.'
          : styleKey === 'LabelPicture' ? 'Put a label on every area first.'
          : 'Choose an answer first.';
        feedback.className = 'q-feedback';
        return;
      }
      var ok = ui.correct(tokens);
      var rec = { c: ok, e: ok ? q.points : 0, s: tokens };
      if (styleKey === 'Likert' && cfg.total_variable && cfg.total_variable in vars) {
        var rev = cfg.reversed || [], n = cfg.scale.length;
        setVar(cfg.total_variable, tokens.reduce(function (sum, k, i) { return sum + (rev[i] ? n - Number(k) : Number(k) + 1); }, 0));
      }
      lock(rec);
      onAnswer(rec);
      feedback.tabIndex = -1; feedback.focus({ preventScroll: true });
    });
    if (saved) lock(saved);
    return box;
  }
