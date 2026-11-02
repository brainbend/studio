  var lms = (function () {
    var mode = P.lms || 'none';
    if (mode === 'none') return null;
    var name = mode === '1.2' ? 'API' : 'API_1484_11';
    function find(w) {
      for (var i = 0; w && i < 10; i++) {
        try { if (w[name]) return w[name]; } catch (e) { return null; }
        if (!w.parent || w.parent === w) break;
        w = w.parent;
      }
      return null;
    }
    var api = find(window) || (window.opener ? find(window.opener) : null);
    if (!api) { console.warn('BrainBend Studio: no ' + name + ' found; running without an LMS'); return null; }
    var v12 = mode === '1.2', started = Date.now(), open = false;
    var o = {
      init: function () { open = String(v12 ? api.LMSInitialize('') : api.Initialize('')) === 'true'; return open; },
      get: function (k) { return v12 ? api.LMSGetValue(k) : api.GetValue(k); },
      set: function (k, val) { return v12 ? api.LMSSetValue(k, String(val)) : api.SetValue(k, String(val)); },
      commit: function () { return v12 ? api.LMSCommit('') : api.Commit(''); },
      finish: function () {
        if (!open) return; open = false;
        var secs = (Date.now() - started) / 1000;
        if (v12) {
          var h = Math.floor(secs / 3600), m = Math.floor(secs % 3600 / 60), s = (secs % 60).toFixed(2);
          var p2 = function (n) { return (n < 10 ? '0' : '') + n; };
          o.set('cmi.core.session_time', p2(h) + ':' + p2(m) + ':' + (s < 10 ? '0' : '') + s);
          api.LMSCommit(''); api.LMSFinish('');
        } else {
          o.set('cmi.session_time', 'PT' + Math.floor(secs / 3600) + 'H' + Math.floor(secs % 3600 / 60) + 'M' + (secs % 60).toFixed(2) + 'S');
          api.Commit(''); api.Terminate('');
        }
      },
      suspendKey: 'cmi.suspend_data',
      locationKey: v12 ? 'cmi.core.lesson_location' : 'cmi.location',
      complete: function () {
        if (v12) {
          var st = o.get('cmi.core.lesson_status');
          if (st !== 'passed' && st !== 'failed' && st !== 'completed') o.set('cmi.core.lesson_status', 'completed');
          o.set('cmi.core.exit', '');
        } else {
          if (o.get('cmi.completion_status') !== 'completed') o.set('cmi.completion_status', 'completed');
          o.set('cmi.exit', 'normal');
        }
      },
      incomplete: function () {
        if (v12) {
          var st = o.get('cmi.core.lesson_status');
          if (st === 'not attempted' || st === '' || st === 'browsed') o.set('cmi.core.lesson_status', 'incomplete');
          o.set('cmi.core.exit', 'suspend');
        } else {
          var cs = o.get('cmi.completion_status');
          if (cs === 'not attempted' || cs === 'unknown' || cs === '') o.set('cmi.completion_status', 'incomplete');
          o.set('cmi.exit', 'suspend');
        }
      },
      // Score is reported as a percentage (0-100). 1.2 folds pass/fail into lesson_status.
      score: function (pct, passed) {
        if (v12) {
          o.set('cmi.core.score.min', 0); o.set('cmi.core.score.max', 100); o.set('cmi.core.score.raw', pct);
          o.set('cmi.core.lesson_status', passed ? 'passed' : 'failed');
          o.set('cmi.core.exit', '');
        } else {
          o.set('cmi.score.min', 0); o.set('cmi.score.max', 100); o.set('cmi.score.raw', pct);
          o.set('cmi.score.scaled', (pct / 100).toFixed(4));
          o.set('cmi.success_status', passed ? 'passed' : 'failed');
          o.set('cmi.completion_status', 'completed');
          o.set('cmi.exit', 'normal');
        }
      },
      limit: v12 ? 4096 : 64000
    };
    return o;
  })();
