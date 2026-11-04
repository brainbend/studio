const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture, boot, $, $$, click, currentSlide, sleep, visibleLayers } = require('./helpers');

test('authored course: click sets a variable, a conditional trigger shows a layer, JS runs', () => {
  const { w, errors } = boot(fixture('authored'));
  assert.deepEqual(visibleLayers(w), ['block', 'none']);
  const btn = $$(w, '.obj').find((o) => /Click me/.test(o.textContent));
  assert.equal(btn.getAttribute('role'), 'button');
  assert.equal(btn.getAttribute('aria-hidden'), null);
  assert.equal($$(w, '.obj')[1].getAttribute('aria-hidden'), 'true', 'the decorative line is hidden from screen readers');
  const a = $$(w, '.obj')[2].querySelector('a');
  assert.equal(a.textContent, 'docs');
  assert.equal(a.getAttribute('href'), 'https://example.com/docs');
  assert.equal(a.getAttribute('target'), '_blank');
  click(w, btn);
  assert.deepEqual(visibleLayers(w), ['block', 'block']);
  assert.match($$(w, '.obj').find((o) => /Hello/.test(o.textContent)).textContent, /Count=1/);
  click(w, btn);
  assert.match($$(w, '.obj').find((o) => /Hello/.test(o.textContent)).textContent, /Count=2/);
  assert.deepEqual(errors, []);
});

test('keyboard activation works on click-trigger objects', () => {
  const { w } = boot(fixture('authored'));
  const btn = $$(w, '.obj').find((o) => /Click me/.test(o.textContent));
  btn.dispatchEvent(new w.KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
  assert.deepEqual(visibleLayers(w), ['block', 'block']);
});

test('quiz: navigation is gated, answers are scored, results and retry work', () => {
  const { w, errors } = boot(fixture('quiz'));
  const next = () => $$(w, '#nav button').find((b) => b.textContent === 'Next');
  assert.equal(currentSlide(w), 0);
  click(w, $$(w, '.obj').find((o) => /Start quiz/.test(o.textContent)));
  assert.equal(currentSlide(w), 1);
  assert.equal(next().disabled, true, 'Next is disabled until the question is answered');

  const q1 = $$(w, '.slide')[1];
  click(w, q1.querySelector('.q-submit'));
  assert.match(q1.querySelector('.q-feedback').textContent, /Choose an answer/);
  q1.querySelector('input[value=a]').checked = true; // correct
  click(w, q1.querySelector('.q-submit'));
  assert.equal(q1.querySelector('.q-feedback').textContent, 'Correct!');
  assert.equal(next().disabled, false);
  click(w, next());

  const q2 = $$(w, '.slide')[2];
  q2.querySelector('input[value=a]').checked = true; // only one of two correct answers
  click(w, q2.querySelector('.q-submit'));
  assert.equal(q2.querySelector('.q-feedback').textContent, 'That is not correct.');
  click(w, next());

  const res = $$(w, '.slide')[3];
  assert.equal(res.querySelector('.r-score').textContent, '25%'); // 1 of 4 points
  assert.match(res.querySelector('.r-verdict').textContent, /did not pass/);

  click(w, res.querySelector('.q-submit')); // Retry
  assert.equal(currentSlide(w), 0);
  assert.equal($$(w, '.slide')[1].querySelector('input').disabled, false);
  assert.deepEqual(errors, []);
});

for (const mode of ['1.2', '2004']) {
  const dir = () => fixture(mode === '1.2' ? 'quiz12' : 'quiz2004');
  const key = (k12, k2004) => (mode === '1.2' ? k12 : k2004);

  test(`SCORM ${mode}: initializes, reports score and pass/fail, bookmarks`, async () => {
    const { w, store, log, errors } = boot(dir(), mode);
    assert.ok(log.some((l) => l === `${key('cmi.core.lesson_status', 'cmi.completion_status')}=incomplete`));
    click(w, $$(w, '.obj').find((o) => /Start quiz/.test(o.textContent)));
    const next = () => $$(w, '#nav button').find((b) => b.textContent === 'Next');
    const q1 = $$(w, '.slide')[1];
    q1.querySelector('input[value=a]').checked = true; click(w, q1.querySelector('.q-submit')); click(w, next());
    const q2 = $$(w, '.slide')[2];
    q2.querySelector('input[value=a]').checked = true; q2.querySelector('input[value=b]').checked = true;
    click(w, q2.querySelector('.q-submit')); click(w, next());
    assert.equal(store[key('cmi.core.score.raw', 'cmi.score.raw')], '100');
    assert.equal(store[key('cmi.core.lesson_status', 'cmi.success_status')], 'passed');
    if (mode === '2004') assert.equal(store['cmi.completion_status'], 'completed');
    assert.equal(store[key('cmi.core.lesson_location', 'cmi.location')], '3');
    await sleep(700);
    assert.match(store['cmi.suspend_data'], /"v":2/);
    assert.deepEqual(errors, []);
  });

  test(`SCORM ${mode}: resumes at the saved slide with answers locked`, async () => {
    const first = boot(dir(), mode);
    click(first.w, $$(first.w, '.obj').find((o) => /Start quiz/.test(o.textContent)));
    const q1 = $$(first.w, '.slide')[1];
    q1.querySelector('input[value=b]').checked = true; click(first.w, q1.querySelector('.q-submit'));
    click(first.w, $$(first.w, '#nav button').find((b) => b.textContent === 'Next'));
    await sleep(700);
    const init = {
      'cmi.suspend_data': first.store['cmi.suspend_data'],
      [key('cmi.core.lesson_location', 'cmi.location')]: '2',
    };
    const again = boot(dir(), mode, init);
    assert.equal(currentSlide(again.w), 2);
    assert.equal($$(again.w, '.slide')[1].querySelector('input').disabled, true);
  });
}

test('failing score is reported as failed', () => {
  const { w, store } = boot(fixture('quiz12'), '1.2');
  click(w, $$(w, '.obj').find((o) => /Start quiz/.test(o.textContent)));
  const next = () => $$(w, '#nav button').find((b) => b.textContent === 'Next');
  const q1 = $$(w, '.slide')[1];
  q1.querySelector('input[value=b]').checked = true; click(w, q1.querySelector('.q-submit')); click(w, next());
  const q2 = $$(w, '.slide')[2];
  q2.querySelector('input[value=c]').checked = true; click(w, q2.querySelector('.q-submit')); click(w, next());
  assert.equal(store['cmi.core.lesson_status'], 'failed');
  assert.equal(store['cmi.core.score.raw'], '0');
});

test('entrance animation plays when a layer is shown, and is skipped for reduced motion', () => {
  const { w } = boot(fixture('authored'));
  assert.equal(w.__animations.length, 0, 'the popup layer is not visible yet');
  click(w, $$(w, '.obj').find((o) => /Click me/.test(o.textContent)));
  assert.equal(w.__animations.length, 1);
  const a = w.__animations[0];
  assert.match(a.text, /Hello/);
  assert.equal(a.options.duration, 300);
  assert.equal(a.options.delay, 100);
  assert.equal(a.keyframes[0].opacity, 0);

  // matchMedia is absent in jsdom; emulate a reduced-motion preference before the player starts.
  const { JSDOM } = require('jsdom');
  const fs = require('node:fs');
  const dom = new JSDOM(fs.readFileSync(fixture('authored') + '/index.html', 'utf8'), {
    runScripts: 'dangerously',
    beforeParse(win) {
      win.__animations = [];
      win.Element.prototype.animate = function () { win.__animations.push(1); };
      win.matchMedia = () => ({ matches: true });
    },
  });
  click(dom.window, [...dom.window.document.querySelectorAll('.obj')].find((o) => /Click me/.test(o.textContent)));
  assert.equal(dom.window.__animations.length, 0);
});

test('hover and key-press triggers fire', () => {
  const { w } = boot(fixture('authored'));
  const btn = $$(w, '.obj').find((o) => /Click me/.test(o.textContent));
  btn.dispatchEvent(new w.MouseEvent('mouseenter'));
  assert.equal(w.GetPlayer().GetVar('Count'), 41);
  assert.deepEqual(visibleLayers(w), ['block', 'none']);
  w.document.dispatchEvent(new w.KeyboardEvent('keydown', { key: 'x' }));
  assert.deepEqual(visibleLayers(w), ['block', 'none'], 'other keys do nothing');
  w.document.dispatchEvent(new w.KeyboardEvent('keydown', { key: 'K' }));
  assert.deepEqual(visibleLayers(w), ['block', 'block'], 'matching is case-insensitive');
});

test('timeline: objects appear and leave on schedule, the end event fires, auto-advance moves on', async () => {
  const { w, errors } = boot(fixture('timeline'));
  const obj = (name) => $$(w, '.obj').find((o) => o.textContent === name);
  const shown = (name) => obj(name).style.visibility !== 'hidden';
  // t ~ 0: "late" has not appeared yet; the others are on screen.
  assert.equal(shown('early'), true);
  assert.equal(shown('late'), false);
  assert.equal(shown('always'), true);
  await sleep(220); // t ~ 220: "early" is gone (100 ms), "late" is on (from 150 ms)
  assert.equal(shown('early'), false);
  assert.equal(shown('late'), true);
  assert.equal(currentSlide(w), 0);
  await sleep(120); // t ~ 340: slide A's timeline (300 ms) ended and it advanced by itself
  assert.equal(currentSlide(w), 1);
  assert.equal(w.GetPlayer().GetVar('Done'), false, "B's timeline (100 ms, started at ~300) has not ended yet");
  await sleep(200); // t ~ 540: slide B's timeline ended and fired its trigger
  assert.equal(w.GetPlayer().GetVar('Done'), true);
  assert.equal(currentSlide(w), 1, 'B does not auto-advance');
  assert.deepEqual(errors, []);
});

test('timeline timers are cleared when the slide is left', async () => {
  const { w } = boot(fixture('timeline'));
  click(w, $$(w, '#nav button').find((b) => b.textContent === 'Next')); // Next, before A's timeline ends
  assert.equal(currentSlide(w), 1);
  await sleep(450);
  assert.equal(currentSlide(w), 1, 'a stale A timer must not push the course forward');
  assert.equal(w.GetPlayer().GetVar('Done'), true);
});

// ---- other question styles --------------------------------------------------------------------
const slideEl = (w, i) => $$(w, '.slide')[i];
const submit = (w, i) => click(w, slideEl(w, i).querySelector('.q-submit'));
const feedback = (w, i) => slideEl(w, i).querySelector('.q-feedback').textContent;
const nextBtn = (w) => $$(w, '#nav button').find((b) => b.textContent === 'Next');

function sortSequence(w, i, wanted) {
  // Bubble the shuffled list into `wanted` order by pressing the move buttons, as a learner would.
  for (let guard = 0; guard < 50; guard++) {
    const rows = [...slideEl(w, i).querySelectorAll('.q-seq li')];
    const names = rows.map((r) => r.querySelector('.choice-text').textContent);
    const bad = names.findIndex((n, k) => n !== wanted[k]);
    if (bad < 0) return;
    const from = names.indexOf(wanted[bad]);
    rows[from].querySelectorAll('button')[0].click(); // move up
  }
  throw new Error('could not sort');
}

test('question styles: typed, matching, sequencing and true/false all score correctly', () => {
  const { w, errors } = boot(fixture('quiz2'));
  assert.equal(currentSlide(w), 0);
  // Fill in the blank: needs text, ignores case and spacing, accepts either listed answer.
  submit(w, 0);
  assert.match(feedback(w, 0), /Choose an answer/);
  slideEl(w, 0).querySelector('input.q-text').value = '  pARIS ';
  submit(w, 0);
  assert.equal(feedback(w, 0), 'Correct!');
  assert.equal(slideEl(w, 0).querySelector('input.q-text').disabled, true);
  click(w, nextBtn(w));

  // Matching: every item must be matched before submitting.
  const sels = [...slideEl(w, 1).querySelectorAll('select')];
  assert.equal(sels.length, 3);
  sels[0].value = 'Meows';
  submit(w, 1);
  assert.match(feedback(w, 1), /Match every item/);
  sels[1].value = 'Barks'; sels[2].value = 'Moos';
  submit(w, 1);
  assert.equal(feedback(w, 1), 'Correct!');
  click(w, nextBtn(w));

  // Sequencing: starts shuffled (never already correct), then the learner orders it.
  const startNames = [...slideEl(w, 2).querySelectorAll('.q-seq .choice-text')].map((n) => n.textContent);
  assert.notDeepEqual(startNames, ['First', 'Second', 'Third']);
  sortSequence(w, 2, ['First', 'Second', 'Third']);
  submit(w, 2);
  assert.equal(feedback(w, 2), 'Correct!');
  assert.equal(slideEl(w, 2).querySelectorAll('.q-move').length, 0, 'controls are removed once answered');
  click(w, nextBtn(w));

  // True/false is a two-choice question.
  slideEl(w, 3).querySelector('input[value=t]').checked = true;
  submit(w, 3);
  click(w, nextBtn(w));
  const res = slideEl(w, 4);
  assert.equal(res.querySelector('.r-score').textContent, '100%');
  assert.match(res.querySelector('.r-verdict').textContent, /passed/);
  assert.deepEqual(errors, []);
});

test('question styles: wrong answers are marked and accepted answers are shown', () => {
  const { w } = boot(fixture('quiz2'));
  slideEl(w, 0).querySelector('input.q-text').value = 'London';
  submit(w, 0);
  assert.match(feedback(w, 0), /not correct\. Accepted: Paris \/ Paris, France/);
  click(w, nextBtn(w));
  const sels = [...slideEl(w, 1).querySelectorAll('select')];
  sels[0].value = 'Barks'; sels[1].value = 'Meows'; sels[2].value = 'Moos'; // one right, two wrong
  submit(w, 1);
  assert.match(feedback(w, 1), /not correct/);
  assert.equal(slideEl(w, 1).querySelectorAll('.choice.wrong').length, 2);
  click(w, nextBtn(w));
  submit(w, 2); // shuffled order submitted as is
  assert.match(feedback(w, 2), /not correct/);
  click(w, nextBtn(w));
  slideEl(w, 3).querySelector('input[value=f]').checked = true;
  submit(w, 3);
  click(w, nextBtn(w));
  assert.equal(slideEl(w, 4).querySelector('.r-score').textContent, '0%');
});

test('slide menu: lists slides, blocks skipping past unanswered questions, marks progress', () => {
  const { w, errors } = boot(fixture('quiz2'));
  const menu = w.document.getElementById('menu');
  assert.ok(menu && menu.hidden, 'the menu starts closed');
  const menuBtn = $$(w, '#nav button')[0];
  assert.equal(menuBtn.textContent, 'Menu');
  click(w, menuBtn);
  assert.equal(menu.hidden, false);
  assert.equal(menuBtn.getAttribute('aria-expanded'), 'true');
  const items = () => $$(w, '#menu button');
  assert.equal(items().length, 5);
  assert.equal(items()[0].getAttribute('aria-current'), 'page');
  assert.equal(items()[0].disabled, false);
  assert.equal(items()[1].disabled, true, 'cannot move past the unanswered first question');
  assert.equal(items()[2].disabled, true);
  assert.equal(items()[4].disabled, true);
  // Answer question 1, and the slides up to the next unanswered question open up.
  menu.hidden = true;
  slideEl(w, 0).querySelector('input.q-text').value = 'Paris';
  submit(w, 0);
  const bar = () => w.document.querySelector('#progress');
  assert.equal(items()[1].disabled, false, 'question 1 is answered, so question 2 can be reached');
  assert.equal(items()[2].disabled, true, 'but not past question 2, which is unanswered');
  assert.equal(bar().getAttribute('aria-valuenow'), '20', 'one of five slides visited');
  click(w, items()[1]);
  assert.equal(currentSlide(w), 1);
  assert.equal(menu.hidden, true, 'picking a slide closes the menu');
  assert.equal(bar().getAttribute('aria-valuenow'), '40');
  assert.ok(items()[0].classList.contains('visited'));
  // Escape closes the menu and returns focus to the button.
  click(w, menuBtn);
  w.document.dispatchEvent(new w.KeyboardEvent('keydown', { key: 'Escape' }));
  assert.equal(menu.hidden, true);
  assert.deepEqual(errors, []);
});

test('lines are drawn as SVG strokes, rising or falling', () => {
  const { w, errors } = boot(fixture('authored'));
  const line = $$(w, '.obj svg line')[0];
  assert.ok(line, 'a line object renders an svg line');
  assert.equal(line.getAttribute('stroke-width'), '6');
  assert.equal(line.getAttribute('y1'), '100'); // rising: starts at the bottom
  assert.equal(line.getAttribute('y2'), '0');
  assert.deepEqual(errors, []);
});

test('slides follow the master: gradient, colour and picture backgrounds, and master shapes underneath', () => {
  const { w, errors } = boot(fixture('master'));
  const slides = $$(w, '.slide');
  assert.match(slides[0].style.background, /linear-gradient\(90deg/);
  assert.match(slides[1].style.background, /rgba?\(0, ?128, ?0/);
  assert.match(slides[2].style.backgroundImage, /url\(.*assets\/a0\.png/);
  assert.equal(slides[2].style.backgroundSize, 'cover');
  for (const s of slides) assert.match(s.textContent, /MASTERLOGO/);
  assert.deepEqual(errors, []);
});

test('more actions and events: show/hide object, adjust variable, mouse leave, timeline reaches, open page', async () => {
  const { w, errors } = boot(fixture('actions'));
  const opened = [];
  w.open = (url) => opened.push(url);
  const btn = $$(w, '.obj').find((o) => o.textContent === 'Go');
  const target = $$(w, '.obj').find((o) => o.textContent === 'Target');
  const get = (v) => w.GetPlayer().GetVar(v);
  assert.equal(target.style.display, 'none', 'hidden at start');
  assert.equal(get('N'), 10);
  click(w, btn);
  assert.equal(target.style.display, '', 'show object');
  assert.equal(get('N'), 15, 'add');
  assert.equal(get('S'), 'ab', 'append');
  btn.dispatchEvent(new w.MouseEvent('mouseleave'));
  assert.equal(get('N'), 30, 'multiply on mouse leave');
  btn.dispatchEvent(new w.MouseEvent('dblclick'));
  assert.equal(target.style.display, 'none', 'hide object');
  assert.deepEqual(opened, ['https://example.com/']);
  await sleep(200);
  assert.equal(get('N'), 29, 'timeline reaches 0.12 s: subtract 1');
  // A cue point in the video fires once as playback passes it, and again after seeking back.
  const video = w.document.querySelector('video');
  const at = (s) => { Object.defineProperty(video, 'currentTime', { value: s, configurable: true }); video.dispatchEvent(new w.Event('timeupdate')); };
  at(1); assert.equal(get('N'), 29, 'not yet');
  at(1.6); assert.equal(get('N'), 129, 'cue point reached');
  at(1.9); assert.equal(get('N'), 129, 'only once');
  at(0.2); at(1.6); assert.equal(get('N'), 229, 'again after seeking back');
  assert.deepEqual(errors, []);
});

test('question styles: blanks, drag the words, mark the words, numbers and sorting score correctly', () => {
  const { w, errors } = boot(fixture('quiz3'));
  const slide = (i) => slideEl(w, i);
  const chip = (i, text) => [...slide(i).querySelectorAll('.q-chip')].find((c) => c.textContent === text);

  // Fill in the blanks: every blank is needed; alternatives and case are accepted.
  const blanks = [...slide(0).querySelectorAll('input.q-blank')];
  assert.equal(blanks.length, 2);
  blanks[0].value = 'FELINE';
  submit(w, 0);
  assert.match(feedback(w, 0), /Fill in every blank/);
  blanks[1].value = 'dog';
  submit(w, 0);
  assert.equal(feedback(w, 0), 'Correct!');
  assert.equal(blanks[0].disabled, true);
  click(w, nextBtn(w));

  // Drag the words: pick a word, then a blank; a spare word is in the bank; a wrong word is marked.
  assert.equal(slide(1).querySelectorAll('.q-slot').length, 3);
  assert.equal(slide(1).querySelectorAll('.q-chip').length, 4, 'three answers and one spare word');
  submit(w, 1);
  assert.match(feedback(w, 1), /Fill in every blank/);
  const place = (word, slotIndex) => { click(w, chip(1, word)); click(w, slide(1).querySelectorAll('.q-slot')[slotIndex]); };
  place('metal', 0);
  place('water', 1);
  place('air', 2);
  assert.equal(slide(1).querySelector('.q-slot').textContent, 'metal');
  click(w, slide(1).querySelector('.q-slot')); // take the word back
  assert.equal(chip(1, 'metal').disabled, false);
  place('sunlight', 0);
  submit(w, 1);
  assert.equal(feedback(w, 1), 'Correct!');
  click(w, nextBtn(w));

  // Mark the words: needs a mark; exactly the starred words must be marked.
  submit(w, 2);
  assert.match(feedback(w, 2), /Mark at least one word/);
  const word = (t) => [...slide(2).querySelectorAll('.q-word')].find((b) => b.textContent === t);
  ['dog', 'cat'].forEach((t) => click(w, word(t)));
  submit(w, 2);
  assert.match(feedback(w, 2), /not correct/, 'a missing word is wrong');
  assert.equal(slide(2).querySelectorAll('.q-word.missed').length, 1);
  click(w, nextBtn(w));

  // Number: tolerance and a comma decimal are accepted; text is not a number.
  const num = slide(3).querySelector('input.q-text');
  num.value = 'pi';
  submit(w, 3);
  assert.match(feedback(w, 3), /Type a number/);
  num.value = '3,15';
  submit(w, 3);
  assert.equal(feedback(w, 3), 'Correct!');
  click(w, nextBtn(w));

  // Sorting: pick an item, then its group. Everything must be sorted.
  submit(w, 4);
  assert.match(feedback(w, 4), /Sort every item/);
  const into = (item, group) => { click(w, chip(4, item)); click(w, [...slide(4).querySelectorAll('.q-cat-head')].find((h) => h.textContent === group)); };
  into('Apple', 'Fruit'); into('Banana', 'Fruit'); into('Carrot', 'Vegetable'); into('Spinach', 'Fruit');
  submit(w, 4);
  assert.match(feedback(w, 4), /not correct/);
  assert.equal(slide(4).querySelectorAll('.q-chip.wrong').length, 1);
  click(w, nextBtn(w));

  // Find the hotspot: a click is placed on the picture; inside either area is correct (one is an oval).
  submit(w, 5);
  assert.match(feedback(w, 5), /Click the picture first/);
  const pic = slide(5).querySelector('.q-hot-pic'), im = slide(5).querySelector('img');
  im.getBoundingClientRect = () => ({ left: 100, top: 100, width: 400, height: 200 });
  const tap = (x, y) => pic.dispatchEvent(new w.MouseEvent('click', { bubbles: true, clientX: 100 + x * 400, clientY: 100 + y * 200 }));
  tap(0.95, 0.15);
  assert.equal(slide(5).querySelector('.q-pin').style.display, 'block');
  tap(0.64, 0.64); // the corner of the oval's box is outside the oval
  submit(w, 5);
  assert.match(feedback(w, 5), /not correct/);
  click(w, nextBtn(w));
  assert.equal(slide(6).querySelector('.r-score').textContent, '50%', 'three of six right');
  assert.deepEqual(errors, []);
});

test('slide navigation rules: next after the timeline, no way back, resume or start over, no next', async () => {
  const { w, errors } = boot(fixture('nav'));
  const menuItem = (i) => $$(w, '#menu button')[i];
  const box = (id) => $$(w, '.obj').find((o) => o.textContent === `Box ${id}`);
  const reveal = (id) => click(w, $$(w, '.obj').filter((o) => o.textContent === 'Reveal')[id === 'b' ? 1 : 2]);
  // A: Next waits for its 150 ms timeline.
  assert.equal(nextBtn(w).disabled, true, 'Next is off until the timeline ends');
  await sleep(220);
  assert.equal(nextBtn(w).disabled, false);
  click(w, nextBtn(w));
  // B: no way back; it resumes where it was left.
  assert.equal(currentSlide(w), 1);
  assert.equal($$(w, '#nav button').find((b) => b.textContent === 'Previous').disabled, true, 'Previous is off');
  assert.equal(box('b').style.display, 'none');
  reveal('b');
  assert.equal(box('b').style.display, '');
  click(w, menuItem(0));
  click(w, menuItem(1));
  assert.equal(box('b').style.display, '', 'Resume keeps the revealed box');
  // C: starts over each time; Next never works.
  click(w, menuItem(2));
  assert.equal(nextBtn(w).disabled, true);
  reveal('c');
  assert.equal(box('c').style.display, '');
  click(w, menuItem(0));
  click(w, menuItem(2));
  assert.equal(box('c').style.display, 'none', 'Reset hides it again');
  assert.deepEqual(errors, []);
});

test('hotspot: the arrow keys move the marker and a point in an area is correct', () => {
  const { w, errors } = boot(fixture('quiz3'));
  const menu = () => $$(w, '#menu button');
  click(w, menu()[5]);
  const slide = slideEl(w, 5);
  const pic = slide.querySelector('.q-hot-pic');
  const key = (k, shift) => pic.dispatchEvent(new w.KeyboardEvent('keydown', { key: k, shiftKey: !!shift, bubbles: true }));
  key('ArrowLeft', true); key('ArrowLeft', true); key('ArrowLeft', true); key('ArrowUp', true); key('ArrowUp', true); key('ArrowUp', true);
  // From the centre (0.5, 0.5) three big steps left and up land at (0.2, 0.2): inside the first area.
  assert.equal(slide.querySelector('.q-pin').style.left, '20%');
  submit(w, 5);
  assert.equal(feedback(w, 5), 'Correct!');
  assert.equal(slide.querySelectorAll('.q-area').length, 2, 'the correct areas are shown afterwards');
  assert.deepEqual(errors, []);
});

test('question bank: each attempt shows a random draw, counts and scores only those, and retry redraws', () => {
  const { w, errors } = boot(fixture('bank'));
  const slides = () => $$(w, '#menu li').filter((li) => !li.hidden).map((li) => li.textContent);
  const inPlay = slides();
  assert.equal(inPlay.length, 4, 'intro, two drawn questions, results');
  assert.deepEqual(inPlay.filter((n) => /^q/.test(n)).slice().sort(), inPlay.filter((n) => /^q/.test(n)), 'not shuffled: original order');
  assert.match($$(w, '#nav .nav-count')[0].textContent, /1 \/ 4/);
  click(w, nextBtn(w));
  const first = inPlay[1];
  assert.match(slideEl(w, ['q1', 'q2', 'q3', 'q4'].indexOf(first) + 1).textContent, /Question 1 of 2/);
  // Answer both drawn questions correctly, then the score is out of two.
  for (let n = 0; n < 2; n++) {
    const i = currentSlide(w);
    slideEl(w, i).querySelector('input[value=a]').checked = true;
    submit(w, i);
    click(w, nextBtn(w));
  }
  assert.equal(slideEl(w, 5).querySelector('.r-score').textContent, '100%');
  assert.match(slideEl(w, 5).querySelector('.r-detail').textContent, /2 of 2 points/);
  // Retry draws again (over many tries a different pair shows up) and always shows exactly two questions.
  const seen = new Set();
  for (let t = 0; t < 30; t++) {
    click(w, $$(w, '.q-submit').find((b) => b.textContent === 'Retry'));
    const now = slides().filter((n) => /^q/.test(n));
    assert.equal(now.length, 2);
    seen.add(now.join());
    // Walk to the results again.
    while (currentSlide(w) !== 5) {
      const i = currentSlide(w);
      if (/Question/.test(slideEl(w, i).textContent) && slideEl(w, i).querySelector('input[value=a]')) { slideEl(w, i).querySelector('input[value=a]').checked = true; submit(w, i); }
      click(w, nextBtn(w));
    }
  }
  assert.ok(seen.size > 1, 'the draw changes between attempts');
  assert.deepEqual(errors, []);
});

test('question bank: a resumed attempt keeps its draw', () => {
  const a = boot(fixture('bank12'), '1.2');
  const drawn = a.w.document.querySelectorAll('#menu li:not([hidden])');
  const names = [...drawn].map((li) => li.textContent);
  a.w.dispatchEvent(new a.w.Event('pagehide'));
  const saved = a.store['cmi.suspend_data'];
  assert.ok(saved && JSON.parse(saved).dr, 'the draw is in the suspend data');
  const b = boot(fixture('bank12'), '1.2', { 'cmi.suspend_data': saved, 'cmi.core.lesson_location': '0' });
  const again = [...b.w.document.querySelectorAll('#menu li:not([hidden])')].map((li) => li.textContent);
  assert.deepEqual(again, names);
});

test('question styles: survey, free text, dictation, drop-downs and labelling', () => {
  const { w, errors } = boot(fixture('quiz4'));
  const slide = (i) => slideEl(w, i);
  // Survey: every statement needs a rating; it is never marked wrong and earns nothing.
  submit(w, 0);
  assert.match(feedback(w, 0), /Rate every statement/);
  const rows = [...slide(0).querySelectorAll('tr')].slice(1);
  assert.equal(rows.length, 2);
  rows.forEach((tr, i) => { const r = tr.querySelectorAll('input')[i + 1]; r.checked = true; });
  submit(w, 0);
  assert.equal(feedback(w, 0), 'Thank you for your answer.');
  assert.equal(slide(0).querySelectorAll('input:disabled').length, 10);
  // Scored: points 2 and (reversed) 5 - 2 = 3 add up to 5.
  assert.equal(w.GetPlayer().GetVar('Score'), 5);
  click(w, nextBtn(w));

  // Free text: needs three words.
  const essay = slide(1).querySelector('textarea');
  essay.value = 'two words';
  submit(w, 1);
  assert.match(feedback(w, 1), /Write at least 3 words/);
  essay.value = 'now three words';
  submit(w, 1);
  assert.equal(feedback(w, 1), 'Thank you for your answer.');
  assert.equal(essay.disabled, true);
  click(w, nextBtn(w));

  // Dictation: case and punctuation are ignored.
  assert.ok(slide(2).querySelector('audio'));
  // The clip may be played twice; the second ending uses up the plays.
  const clip = slide(2).querySelector('audio');
  const plays = () => slide(2).querySelector('.q-plays').textContent;
  assert.equal(plays(), 'Plays left: 2');
  clip.dispatchEvent(new w.Event('play')); assert.equal(plays(), 'Plays left: 1');
  clip.dispatchEvent(new w.Event('play')); assert.equal(plays(), 'Plays left: 1'); // resuming is not a new play
  clip.dispatchEvent(new w.Event('ended'));
  clip.dispatchEvent(new w.Event('play')); assert.equal(plays(), 'Plays left: 0');
  clip.dispatchEvent(new w.Event('ended'));
  assert.equal(plays(), 'No plays left.');
  assert.equal(clip.hasAttribute('controls'), false);
  slide(2).querySelector('input.q-text').value = '  SAFETY   first ';
  submit(w, 2);
  assert.equal(feedback(w, 2), 'Correct!');
  click(w, nextBtn(w));

  // Drop-down passage: the first listed option of each blank is correct.
  const sels = [...slide(3).querySelectorAll('select')];
  assert.equal(sels.length, 2);
  assert.equal(sels[0].querySelectorAll('option').length, 4, 'a prompt and three choices');
  sels[0].value = 'sun';
  submit(w, 3);
  assert.match(feedback(w, 3), /Choose an answer for every blank/);
  sels[1].value = 'galaxy';
  submit(w, 3);
  assert.match(feedback(w, 3), /not correct/);
  assert.equal(slide(3).querySelectorAll('select.right').length, 1);
  click(w, nextBtn(w));

  // Labelling: pick a label, then an area. The spare label is left over.
  const chips = () => [...slide(4).querySelectorAll('.q-chip')];
  assert.equal(chips().length, 3);
  const put = (label, area) => { click(w, chips().find((c) => c.textContent === label)); click(w, slide(4).querySelectorAll('.q-zone')[area]); };
  submit(w, 4);
  assert.match(feedback(w, 4), /Put a label on every area/);
  put('Wheel', 0);
  put('Engine', 1);
  submit(w, 4);
  assert.match(feedback(w, 4), /not correct/);
  assert.equal(slide(4).querySelectorAll('.q-zone.wrong').length, 2);
  click(w, nextBtn(w));
  // Only the three graded questions count: dictation right, drop-downs wrong, labels wrong.
  assert.equal(slide(5).querySelector('.r-score').textContent, '33%');
  assert.match(slide(5).querySelector('.r-detail').textContent, /1 of 3 points/);
  assert.deepEqual(errors, []);
});

test('emphasis and motion paths play after the object appears, and a trigger can play them again', async () => {
  const { w, errors } = boot(fixture('anim'));
  const by = (text) => w.__animations.filter((a) => a.text === text);
  // Emphasis: spin twice, after a short delay.
  const spin = by('spin');
  assert.equal(spin.length, 1);
  assert.equal(spin[0].options.iterations, 2);
  assert.equal(spin[0].options.delay, 50);
  assert.equal(JSON.stringify(spin[0].keyframes.map((k) => k.rotate)), '["0deg","360deg"]');
  // Motion path: a straight line right, moving by its own `translate` property and staying at the end.
  const mover = by('mover');
  assert.equal(mover.length, 1);
  assert.equal(JSON.stringify(mover[0].keyframes.map((k) => k.translate)), '["0px 0px","300px 0px"]');
  assert.equal(mover[0].options.fill, 'forwards');
  assert.equal(JSON.stringify(mover[0].keyframes.map((k) => k.offset)), '[0,1]');
  // A curved path is sampled into many points that end where the path ends.
  const curved = by('curved')[0];
  assert.ok(curved.keyframes.length > 10);
  assert.equal(curved.keyframes[curved.keyframes.length - 1].translate, '300px 0px');
  assert.ok(curved.keyframes.some((k) => parseFloat(k.translate.split(' ')[1]) < -100), 'it rises along the way');
  // An object that appears later starts its emphasis then; repeat 0 means for as long as the slide shows.
  assert.equal(by('later').length, 0);
  await sleep(200);
  assert.equal(by('later').length, 1);
  assert.equal(by('later')[0].options.iterations, Infinity);
  // The trigger plays the object's animations again, right away.
  click(w, $$(w, '.obj').find((o) => o.textContent === 'Again'));
  assert.equal(by('spin').length, 2);
  assert.equal(by('spin')[1].options.delay, 0);
  assert.deepEqual(errors, []);
});

test('click steps: each click plays the effects numbered for it before the slide moves on', async () => {
  const { w, errors } = boot(fixture('clicks'));
  const obj = (t) => $$(w, '.obj').find((o) => o.textContent === t);
  const hidden = (t) => obj(t).style.visibility === 'hidden';
  const spins = () => w.__animations.filter((a) => a.text === 'two' && a.keyframes.some && a.keyframes.some((k) => k.rotate)).length;
  // Before any click: the click-entrance objects wait; the automatic one and the leaving one are there.
  assert.equal(hidden('one'), true);
  assert.equal(hidden('two'), true);
  assert.equal(hidden('three'), false);
  assert.equal(hidden('auto'), false);
  assert.equal(nextBtn(w).disabled, false);
  // Click 1: Next plays the step (entrances together) and stays on the slide.
  click(w, nextBtn(w));
  assert.equal(currentSlide(w), 0);
  assert.equal(hidden('one') || hidden('two'), false);
  assert.equal(spins(), 0);
  // Click 2: a click on the slide background plays the emphasis.
  click(w, w.document.getElementById('stage'));
  assert.equal(spins(), 1);
  assert.equal(currentSlide(w), 0);
  // Click 3: the space bar plays the exit, then the object is gone.
  w.document.dispatchEvent(new w.KeyboardEvent('keydown', { key: ' ', bubbles: true }));
  await sleep(160);
  assert.equal(hidden('three'), true);
  // No steps are left: Next now moves to the next slide.
  click(w, nextBtn(w));
  assert.equal(currentSlide(w), 1);
  // Coming back starts the steps over.
  click(w, $$(w, '#nav button').find((b) => b.textContent === 'Previous'));
  assert.equal(currentSlide(w), 0);
  assert.equal(hidden('one'), true);
  assert.equal(hidden('three'), false);
  assert.deepEqual(errors, []);
});

// ---- Web Lesson player -------------------------------------------------------------------------------
const weblesson = () => fixture('weblesson');
const wlClick = (w, sel) => click(w, w.document.querySelector(sel));
const block = (w, id) => w.document.querySelector(`.wl-block[data-bid="${id}"]`);
const openModule = (w, i) => click(w, w.document.querySelectorAll('.wl-mcard')[i]);

test('web lesson: title page lists the modules; opening one shows it full screen', () => {
  const { w, errors } = boot(weblesson());
  const d = w.document;
  assert.equal(d.getElementById('hero-title').textContent, 'Safety lesson');
  assert.equal(d.getElementById('hero-sub').textContent, 'Stay safe at work');
  assert.match(d.getElementById('hero-desc').textContent, /habits that keep you/);
  assert.equal(d.getElementById('home').hidden, false);
  assert.equal(d.getElementById('module').hidden, true);
  assert.deepEqual($$(w, '.wl-mcard h3').map((h) => h.textContent), ['Basics', 'Check']);
  assert.deepEqual($$(w, '.wl-mcard p').map((p) => p.textContent), ['Why safety matters and the core ideas.', 'Test what you learned.']);
  assert.deepEqual($$(w, '.wl-mcard .wl-chip').map((c) => c.textContent), ['Not started', 'Not started']);
  assert.equal(d.getElementById('start').textContent, 'Start the lesson');
  openModule(w, 0);
  assert.equal(d.getElementById('home').hidden, true);
  assert.equal(d.getElementById('module').hidden, false);
  assert.equal(d.getElementById('sec-0').hidden, false);
  assert.equal(d.getElementById('sec-1').hidden, true);
  assert.equal(w.location.hash, '#/m/0');
  assert.equal(d.querySelector('#sec-0 .wl-eyebrow').textContent, 'Module 1 of 2');
  // The module has been seen, so its card and menu entry show progress; the back button returns.
  click(w, d.getElementById('back-btn'));
  assert.equal(d.getElementById('home').hidden, false);
  assert.equal(w.location.hash, '#/');
  assert.equal($$(w, '.wl-mcard .wl-chip')[0].dataset.state, 'progress', 'the gate holds back the rest of the module');
  assert.equal($$(w, '.wl-mcard .wl-chip')[1].dataset.state, 'new');
  assert.equal(d.getElementById('start').textContent, 'Continue');
  assert.deepEqual(errors, []);
});

test('web lesson: cover, menu, safe text rendering and buttons', () => {
  const { w, errors } = boot(weblesson());
  assert.deepEqual($$(w, '#menu .wl-mtitle').map((b) => b.textContent), ['Basics', 'Check']);
  assert.equal($$(w, 'h1.wl-section-title').length, 2);
  const text = block(w, 't1');
  assert.equal(text.querySelector('strong').textContent, 'this');
  assert.equal(text.querySelector('em').textContent, 'that');
  assert.deepEqual([...text.querySelectorAll('li')].map((l) => l.textContent), ['first point', 'second point']);
  const links = [...text.querySelectorAll('a')];
  assert.equal(links.length, 1, 'only the https link becomes a link');
  assert.equal(links[0].getAttribute('href'), 'https://example.com/safe');
  assert.equal(links[0].getAttribute('rel'), 'noopener noreferrer');
  assert.equal(text.querySelector('img'), null, 'markup in text is shown as text, not run');
  assert.match(text.textContent, /<img src=x onerror=alert\(2\)>/);
  const img = block(w, 'i1').querySelector('img');
  assert.equal(img.getAttribute('alt'), 'A tiny picture');
  assert.equal(block(w, 'i1').querySelector('figcaption').textContent, 'Figure one');
  assert.equal(block(w, 'btn').querySelector('a').getAttribute('href'), 'https://example.com/more');
  assert.deepEqual(errors, []);
});

test('web lesson: a Continue block hides what follows until it is pressed', () => {
  const { w, errors } = boot(weblesson());
  assert.equal(block(w, 'btn').hidden, false);
  assert.equal(block(w, 'gate').hidden, false);
  assert.equal(block(w, 'st').hidden, true);
  assert.equal(block(w, 'q1').hidden, false, 'a gate only holds back the rest of its own module');
  wlClick(w, '.wl-block[data-bid="gate"] button');
  assert.equal(block(w, 'st').hidden, false);
  assert.equal(block(w, 'gate').hidden, true);
  assert.deepEqual(errors, []);
});

test('web lesson: accordion, tabs, process and flashcards', () => {
  const { w } = boot(weblesson());
  wlClick(w, '.wl-block[data-bid="gate"] button');
  assert.equal(block(w, 'acc').querySelectorAll('details').length, 2);
  const tabs = [...block(w, 'tabs').querySelectorAll('[role=tab]')];
  const panels = [...block(w, 'tabs').querySelectorAll('[role=tabpanel]')];
  assert.equal(tabs[0].getAttribute('aria-selected'), 'true');
  assert.equal(panels[1].hidden, true);
  click(w, tabs[1]);
  assert.equal(tabs[1].getAttribute('aria-selected'), 'true');
  assert.equal(panels[0].hidden, true);
  assert.equal(panels[1].hidden, false);
  assert.deepEqual([...block(w, 'proc').querySelectorAll('.wl-num')].map((n) => n.textContent), ['1', '2']);
  const card = block(w, 'flash').querySelector('button');
  assert.equal(card.getAttribute('aria-pressed'), 'false');
  click(w, card);
  assert.equal(card.getAttribute('aria-pressed'), 'true');
  assert.equal(card.querySelector('.wl-card-back').hidden, false);
});

test('web lesson: knowledge check scoring and completion (both rules)', () => {
  const wrong = boot(weblesson());
  wlClick(wrong.w, '.wl-block[data-bid="gate"] button');
  openModule(wrong.w, 0);
  openModule(wrong.w, 1);
  const q = block(wrong.w, 'q1');
  // Pick the second (wrong) choice and submit.
  click(wrong.w, q.querySelectorAll('.choice input')[1]);
  click(wrong.w, q.querySelector('.q-submit'));
  assert.match(q.querySelector('.q-feedback').textContent, /not correct/);
  assert.equal(wrong.w.document.getElementById('done').hidden, true, 'a failed check is not complete');

  const right = boot(weblesson());
  openModule(right.w, 0);
  wlClick(right.w, '.wl-block[data-bid="gate"] button');
  openModule(right.w, 1);
  const q2 = block(right.w, 'q1');
  click(right.w, q2.querySelectorAll('.choice input')[0]);
  click(right.w, q2.querySelector('.q-submit'));
  assert.equal(q2.querySelector('.q-feedback').textContent, 'Correct!');
  assert.equal(right.w.document.getElementById('done').hidden, false);
  assert.match(right.w.document.querySelector('.wl-done-score').textContent, /100%/);
  assert.equal(right.w.document.getElementById('home-bar-text').textContent, '100%');
  assert.deepEqual($$(right.w, '.wl-mcard .wl-chip').map((c) => c.dataset.state), ['done', 'done']);
});

test('web lesson in SCORM 1.2: incomplete until done, then passed with a score; resumes where it left off', () => {
  const first = boot(fixture('weblesson12'), '1.2');
  assert.equal(first.store['cmi.core.lesson_status'], 'incomplete');
  openModule(first.w, 0);
  wlClick(first.w, '.wl-block[data-bid="gate"] button');
  openModule(first.w, 1);
  const q = block(first.w, 'q1');
  click(first.w, q.querySelectorAll('.choice input')[0]);
  click(first.w, q.querySelector('.q-submit'));
  assert.equal(first.store['cmi.core.lesson_status'], 'passed');
  assert.equal(first.store['cmi.core.score.raw'], '100');
  first.w.dispatchEvent(new first.w.Event('pagehide'));
  assert.ok(first.store['cmi.suspend_data'], 'progress is saved');
  // A new session with the saved data shows the gate open and the answer locked.
  const second = boot(fixture('weblesson12'), '1.2', { 'cmi.suspend_data': first.store['cmi.suspend_data'], 'cmi.core.lesson_location': first.store['cmi.core.lesson_location'] });
  assert.equal(block(second.w, 'st').hidden, false);
  assert.equal(block(second.w, 'q1').querySelector('.q-submit').style.display, 'none');
  assert.equal(second.w.document.getElementById('done').hidden, false);
});

// ---- sliders, dials and pictures that change with a variable ------------------------------------------
const controls = () => {
  const { w, errors } = boot(fixture('controls'));
  const [slider, dial, pic, box1, box2, submit, spot] = $$(w, '.obj');
  const layer = slider.parentNode;
  layer.getBoundingClientRect = () => ({ left: 0, top: 0, width: 1920, height: 1080 });
  slider.getBoundingClientRect = () => ({ left: 100, top: 400, width: 800, height: 80 });
  const press = (node, x, y, type = 'pointerdown') => node.dispatchEvent(Object.assign(new w.Event(type, { bubbles: true, cancelable: true }), { clientX: x, clientY: y, pointerId: 1, pointerType: 'mouse', button: 0 }));
  const key = (node, k) => node.dispatchEvent(new w.KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true }));
  const get = (n) => w.GetPlayer().GetVar(n);
  return { w, errors, slider, dial, pic, box1, box2, submit, spot, press, key, get, thumb: () => slider.lastElementChild };
};

test('slider: the thumb sits where its variable says, and keys and drags set the variable', () => {
  const c = controls();
  assert.equal(c.slider.getAttribute('role'), 'slider');
  assert.equal(c.slider.getAttribute('aria-valuenow'), '5');
  assert.equal(c.slider.getAttribute('aria-label'), 'Level');
  // 5 of 0..10 along a track 800 wide with a 40 wide thumb (half the 80 tall frame).
  assert.equal(c.thumb().style.left, '380px');
  c.key(c.slider, 'ArrowRight');
  assert.equal(c.get('Level'), 6);
  assert.equal(c.thumb().style.left, `${0.6 * 760}px`);
  c.key(c.slider, 'Home');
  assert.equal(c.get('Level'), 0);
  assert.equal(c.thumb().style.left, '0px');
  // Dragging: the pointer at the middle of where the thumb would be for 8 (20 + 0.8 * 760 from the left edge).
  c.press(c.slider, 100 + 20 + 0.8 * 760, 440);
  assert.equal(c.get('Level'), 8);
  c.press(c.slider, 100 + 20 + 0.3 * 760 + 5, 440, 'pointermove');
  assert.equal(c.get('Level'), 3, 'dragging follows the pointer, in whole steps');
  c.press(c.slider, 5000, 440, 'pointerup');
  c.press(c.slider, 100 + 20 + 0.9 * 760, 440, 'pointermove');
  assert.equal(c.get('Level'), 3, 'after the pointer is released it no longer drags');
  assert.deepEqual(c.errors, []);
});

test('slider: changing its variable fires triggers, and a picture state can swap the picture and its size', () => {
  const c = controls();
  const img = () => c.pic.querySelector('img');
  const first = img().getAttribute('src');
  assert.equal(c.pic.style.width, '100px');
  c.press(c.slider, 100 + 20 + 0.7 * 760, 440);
  assert.equal(c.get('Level'), 7);
  assert.equal(c.pic.style.width, '60px', 'state B is smaller');
  assert.equal(c.pic.style.height, '30px');
  assert.equal(c.pic.style.left, '520px', 'it resizes about its centre: 500 + (100 - 60) / 2');
  assert.equal(c.pic.style.top, '110px');
  assert.notEqual(img().getAttribute('src'), first, 'and shows the other picture');
  c.press(c.slider, 100 + 20 + 0.3 * 760, 440);
  assert.equal(c.pic.style.width, '100px', 'back to state A');
  assert.equal(img().getAttribute('src'), first);
  assert.equal(c.pic.getAttribute('role'), null, 'picture states are not button states');
  assert.deepEqual(c.errors, []);
});

test('dial: turning sets the variable in steps and the object turns through its sweep', () => {
  const c = controls();
  // 5 of 0..10 over a 180 degree sweep is upright.
  assert.equal(c.dial.getAttribute('role'), 'slider');
  assert.equal(c.dial.style.rotate, '0deg');
  // The dial's centre is (250, 150). A pointer due east is a quarter turn clockwise: the end of the range.
  c.press(c.dial, 350, 150);
  assert.equal(c.get('Turn'), 10);
  assert.equal(c.dial.style.rotate, '90deg');
  // Due west is the other end; straight up is the middle.
  c.press(c.dial, 150, 150, 'pointermove');
  assert.equal(c.get('Turn'), 0);
  assert.equal(c.dial.style.rotate, '-90deg');
  c.press(c.dial, 250, 50, 'pointermove');
  assert.equal(c.get('Turn'), 5);
  // Between: 45 degrees clockwise is three quarters of the way through the sweep.
  c.press(c.dial, 250 + 100, 150 - 100, 'pointermove');
  assert.equal(c.get('Turn'), 8, '7.5 snaps to the nearest step');
  // Past the end of a half-turn sweep it stays at the end.
  c.press(c.dial, 250, 250, 'pointermove');
  assert.ok([0, 10].includes(c.get('Turn')), 'straight down is outside the sweep');
  c.key(c.dial, 'Home');
  assert.equal(c.get('Turn'), 0);
  c.key(c.dial, 'ArrowRight');
  assert.equal(c.get('Turn'), 1);
  assert.equal(c.dial.style.rotate, `${0.1 * 180 - 90}deg`);
  assert.deepEqual(c.errors, []);
});

test('check box: clicking and Space tick it, Disabled ignores clicks, and watchers run when it enters a state', () => {
  const c = controls();
  const click = (n) => n.dispatchEvent(new c.w.MouseEvent('click', { bubbles: true }));
  const tick = (b) => b.querySelector('svg').style.display !== 'none';
  assert.equal(c.box1.getAttribute('role'), 'checkbox');
  assert.equal(c.box1.getAttribute('aria-checked'), 'false');
  assert.equal(tick(c.box1), false);
  const grey = c.submit.style.background;
  assert.match(grey, /150, 150, 150/, 'Submit starts disabled');
  click(c.box1);
  assert.equal(c.box1.getAttribute('aria-checked'), 'true');
  assert.equal(tick(c.box1), true);
  assert.equal(c.box2.getAttribute('aria-checked'), 'false', 'the other box is untouched');
  assert.match(c.submit.style.background, /3, 83, 158/, 'box1 entering Selected switched Submit from Disabled to Normal');
  c.key(c.box1, ' ');
  assert.equal(c.box1.getAttribute('aria-checked'), 'false', 'Space toggles back');
  c.key(c.box2, 'Enter');
  assert.equal(c.box2.getAttribute('aria-checked'), 'true');
  assert.deepEqual(c.errors, []);
});

test('hotspot: an outlined hotspot is clipped to its outline', () => {
  const c = controls();
  assert.equal(c.spot.style.clipPath.replace(/\s/g, ''), 'polygon(0.00%0.00%,100.00%0.00%,0.00%100.00%)');
  assert.equal(c.slider.style.clipPath, '', 'other objects are not clipped');
});

// ---- timeline seeking and the course actions ------------------------------------------------------
test('seek timeline: jumping the slide timeline forward or back shows and hides what its time says', async () => {
  const { w, errors } = boot(fixture('course12'));
  const obj = (t) => $$(w, '.obj').find((o) => o.textContent === t);
  const late = obj('Late');
  assert.equal(late.style.visibility, 'hidden', 'starts at 0.4 s');
  click(w, obj('Seek600'));
  assert.equal(late.style.visibility, 'visible', 'seeking past its start shows it at once');
  click(w, obj('Seek0'));
  assert.equal(late.style.visibility, 'hidden', 'seeking to the start hides it again');
  await sleep(500);
  assert.equal(late.style.visibility, 'visible', 'and it appears on its own when its time comes');
  assert.deepEqual(errors, []);
});

test('course actions: complete reports to the LMS, restart resets everything, exit ends the session', async () => {
  const c = boot(fixture('course12'), '1.2');
  const { w, store, log } = c;
  const obj = (t) => $$(w, '.obj').find((o) => o.textContent === t);
  const get = (v) => w.GetPlayer().GetVar(v);
  assert.notEqual(store['cmi.core.lesson_status'], 'completed');
  click(w, obj('Done'));
  assert.equal(store['cmi.core.lesson_status'], 'completed');
  click(w, obj('Bump')); click(w, obj('Bump'));
  assert.equal(get('N'), 2);
  click(w, obj('Next'));
  assert.equal(w.document.querySelector('#stage').dataset.slide, '1');
  click(w, obj('Restart'));
  assert.equal(get('N'), 0, 'variables are back at their start values');
  assert.equal(w.document.querySelector('#stage').dataset.slide, '0', 'the first slide shows');
  let closed = 0;
  w.close = () => { closed++; };
  click(w, obj('Leave'));
  assert.equal(closed, 1);
  assert.ok(log.some((l) => l.startsWith('cmi.core.session_time')), 'the LMS session was finished');
  await sleep(150);
  assert.match(w.document.body.textContent, /You can close this window/);
  assert.deepEqual(c.errors, []);
});

// ---- Accessibility ------------------------------------------------------------------------------------
const axe = require('axe-core');
const fs = require('node:fs');
const path = require('node:path');
const audit = async (w) => {
  w.eval(axe.source);
  // jsdom does no layout, so colour contrast is checked from the stylesheets elsewhere.
  const r = await w.axe.run(w.document, { rules: { 'color-contrast': { enabled: false } } });
  return Array.from(r.violations, (v) => `${v.id}: ${v.nodes[0].html.slice(0, 120)}`);
};

test('accessibility: every published course passes the automated audit', async () => {
  const dir = path.join(__dirname, '.fixtures');
  const names = fs.readdirSync(dir).filter((n) => fs.statSync(path.join(dir, n)).isDirectory());
  assert.ok(names.length >= 15);
  for (const name of names) {
    const { w } = boot(path.join(dir, name));
    assert.deepEqual(await audit(w), [], name);
  }
  // A web lesson module page (the title page is covered above).
  const { w } = boot(weblesson());
  openModule(w, 0);
  assert.deepEqual(await audit(w), [], 'web lesson module');
});

const a11y = () => fixture('a11y');
const obj = (w, t) => $$(w, '.obj').find((o) => o.textContent === t || o.getAttribute('aria-label') === t);

test('accessibility: landmarks, a heading and a spoken announcement name each slide', () => {
  const { w } = boot(a11y());
  const main = $(w, 'main#stage');
  assert.equal(main.getAttribute('aria-label'), 'Access');
  assert.equal($(w, 'nav#nav').getAttribute('aria-label'), 'Course navigation');
  assert.equal($(w, '#slide-title').textContent, 'First topic');
  assert.equal($(w, '#slide-title').getAttribute('role'), 'heading', 'the hidden slide title is a level 1 heading');
  assert.equal($(w, '#slide-title').getAttribute('aria-level'), '1');
  assert.equal($(w, '#announce').textContent, 'First topic, slide 1 of 2');
  assert.equal($(w, '#announce').getAttribute('aria-live'), 'polite');
  assert.equal($(w, '.nav-count').getAttribute('aria-label'), 'Slide 1 of 2');
  click(w, nextBtn(w));
  assert.equal($(w, '#slide-title').textContent, 'Second topic');
  assert.equal($(w, '#announce').textContent, 'Second topic, slide 2 of 2');
  assert.equal($(w, '.nav-count').getAttribute('aria-label'), 'Slide 2 of 2');
  assert.equal($(w, '.layer[data-layer=pop]').getAttribute('aria-label'), 'More detail', 'a layer is a named group');
  assert.equal($(w, '.layer[data-layer=pop]').getAttribute('role'), 'group');
});

test('accessibility: focus is not left on something that just left the screen', () => {
  const { w } = boot(a11y());
  click(w, nextBtn(w));
  const inSlide = obj(w, 'Next topic');
  inSlide.focus();
  assert.equal(w.document.activeElement, inSlide);
  click(w, inSlide);                       // jumps back to the first slide, hiding this one
  assert.equal(currentSlide(w), 0);
  assert.equal(w.document.activeElement, $(w, '#slide-title'));
  // The player's own Next button keeps focus, so pressing it again and again works.
  nextBtn(w).focus();
  click(w, nextBtn(w));
  assert.equal(w.document.activeElement, nextBtn(w));
});

test('accessibility: opening a layer moves focus into it and closing it gives focus back', async () => {
  const { w } = boot(a11y());
  const open = obj(w, 'Open');
  open.focus();
  click(w, open);
  assert.equal($(w, '.layer[data-layer=pop]').style.display, 'block');
  assert.equal(w.document.activeElement, obj(w, 'Close'), 'focus moved into the layer');
  click(w, obj(w, 'Close'));
  assert.equal($(w, '.layer[data-layer=pop]').style.display, 'none');
  assert.equal(w.document.activeElement, open, 'focus returned to what opened it');
});

test('accessibility: hover content can be reached from the keyboard', () => {
  const { w } = boot(a11y());
  const tip = obj(w, 'Hover over me');
  assert.equal(tip.tabIndex, 0, 'a hover-only object can take focus');
  const get = () => w.GetPlayer().GetVar('Hovered');
  assert.equal(get(), 0);
  tip.focus();
  assert.equal(get(), 1, 'focus does what hover does');
  tip.dispatchEvent(new w.MouseEvent('mouseenter'));
  assert.equal(get(), 1);
  tip.blur();
  assert.equal(get(), 1, 'the pointer is still over it');
  tip.dispatchEvent(new w.MouseEvent('mouseleave'));
  assert.equal(get(), 2, 'leaving both ways does what mouse-leave does');
});

test('accessibility: shapes with alt text are images, disabled buttons say so, focus shows a ring', () => {
  const { w } = boot(a11y());
  const badge = obj(w, 'Gold star badge');
  assert.equal(badge.getAttribute('role'), 'img');
  const locked = obj(w, 'Locked');
  assert.equal(locked.getAttribute('aria-disabled'), 'true');
  click(w, locked);
  assert.equal(w.GetPlayer().GetVar('Hovered'), 0, 'a disabled button does nothing');
  const open = obj(w, 'Open');
  assert.equal(open.style.boxShadow, 'none');
  open.focus();
  assert.match(open.style.boxShadow, /#fff|255, 255, 255/, 'a white band is drawn inside the dark focus outline');
  open.blur();
  assert.equal(open.style.boxShadow, 'none');
});

test('accessibility: a key a control handled does not also play a click step', () => {
  const { w } = boot(fixture('clicks'));
  const hidden = (t) => $$(w, '.obj').find((o) => o.textContent === t).style.visibility === 'hidden';
  assert.equal(hidden('one'), true);
  const ev = new w.KeyboardEvent('keydown', { key: ' ', bubbles: true, cancelable: true });
  w.document.addEventListener('keydown', (e) => e.preventDefault(), { once: true, capture: true });
  w.document.dispatchEvent(ev);
  assert.equal(hidden('one'), true, 'the space bar was already used');
  w.document.dispatchEvent(new w.KeyboardEvent('keydown', { key: ' ', bubbles: true, cancelable: true }));
  assert.equal(hidden('one'), false, 'an unused space bar plays the step');
});

test('accessibility: questions - results are written as well as coloured, and answering moves focus to the feedback', () => {
  const { w } = boot(fixture('quiz2'));
  // The first question in this fixture is typed; answer it wrong, then see the written mark and the focus.
  const input = $(w, '.q-text');
  input.value = 'definitely wrong';
  click(w, $(w, '.q-submit'));
  assert.equal(w.document.activeElement, $(w, '.q-feedback'), 'focus lands on the result, not on a button that vanished');
  assert.equal($(w, '.q-feedback').getAttribute('role'), 'status');
  assert.equal($(w, '.choice').classList.contains('wrong'), true);
  assert.equal(input.getAttribute('aria-describedby'), $(w, '.q-prompt').id, 'the prompt describes a lone field');
});

test('accessibility: web lesson skip link, tab wiring and links that open a new tab', () => {
  const { w } = boot(weblesson());
  const d = w.document;
  const skip = $(w, '.skip');
  click(w, skip);
  assert.equal(w.location.hash, '', 'the skip link does not change the route');
  assert.equal(d.getElementById('home').hidden, false, 'and so does not throw the learner back to the title page');
  assert.equal(d.activeElement, d.getElementById('modules'));
  openModule(w, 0);
  click(w, skip);
  assert.equal(d.getElementById('module').hidden, false);
  assert.equal(d.activeElement, d.getElementById('main'));
  for (const a of $$(w, 'a[target=_blank]')) assert.match(a.textContent, /opens in a new tab/);
  for (const tab of $$(w, '[role=tab]')) {
    const panel = d.getElementById(tab.getAttribute('aria-controls'));
    assert.ok(panel, 'tab controls a panel');
    assert.equal(panel.getAttribute('aria-labelledby'), tab.id);
  }
});

test('entrance and exit effects: spin turns while growing, wheel sweeps, split opens from or closes to a line', async () => {
  const { w, errors } = boot(fixture('effects'));
  const enter = (id) => w.__animations.filter((a) => a.text === id)[0];
  const spin = (id, deg) => {
    const k = enter(id).keyframes;
    assert.equal(k[0].opacity, 0);
    assert.equal(k[0].transform, `scale(0.1) rotate(${-deg}deg)`, id);
    assert.equal(k[1].transform, 'none');
    assert.equal(enter(id).options.easing, 'linear');
  };
  spin('spinq', 90); spin('spinh', 180); spin('spinf', 360);
  const wheel = enter('wheel').keyframes;
  assert.ok(wheel.length > 12, 'stepped finely');
  assert.match(wheel[0].maskImage, /conic-gradient\(#000 0deg 0\.0deg, transparent 0\.0deg 360deg\)/, 'starts fully hidden');
  assert.match(wheel[wheel.length - 1].maskImage, /360\.0deg/, 'ends fully shown');
  assert.equal(wheel[5].webkitMaskImage, wheel[5].maskImage);
  assert.deepEqual(Array.from(enter('svo').keyframes, (k) => k.clipPath), ['inset(0 50% 0 50%)', 'inset(0 0 0 0)'], 'vertical split opens from the middle line outwards');
  assert.deepEqual(Array.from(enter('sho').keyframes, (k) => k.clipPath), ['inset(50% 0 50% 0)', 'inset(0 0 0 0)']);
  const vin = enter('svi').keyframes;
  assert.match(vin[0].maskImage, /linear-gradient\(90deg, #000 0%, #000 0\.0%, transparent 0\.0%, transparent 100\.0%, #000 100\.0%/, 'in: starts hidden');
  assert.match(vin[vin.length - 1].maskImage, /#000 50\.0%, transparent 50\.0%, transparent 50\.0%/, 'in: ends with the halves meeting');
  assert.match(enter('shi').keyframes[3].maskImage, /^linear-gradient\(180deg/);
  // Leaving plays the same effect backwards (and spins on, away from home).
  await sleep(700);
  const leave = (id) => w.__animations.filter((a) => a.text === id)[1];
  assert.equal(leave('spinh').keyframes[1].transform, 'scale(0.1) rotate(180deg)');
  assert.equal(leave('spinh').options.fill, 'forwards');
  const wl = leave('wheel').keyframes;
  assert.match(wl[0].maskImage, /360\.0deg/, 'the wheel starts full and sweeps away');
  assert.deepEqual(Array.from(leave('svo').keyframes, (k) => k.clipPath), ['inset(0 0 0 0)', 'inset(0 50% 0 50%)']);
  assert.deepEqual(errors, []);
});

test('media actions: pause keeps the place, stop pauses and winds back to the start', () => {
  const { w, errors } = boot(fixture('actions'));
  w.open = () => {};
  const video = $(w, 'video');
  const calls = [];
  video.pause = () => calls.push('pause');
  Object.defineProperty(video, 'currentTime', { configurable: true, get: () => 12, set: (v) => calls.push(`seek ${v}`) });
  const btn = $$(w, '.obj').find((o) => o.textContent === 'Go');
  btn.dispatchEvent(new w.MouseEvent('dblclick'));
  assert.deepEqual(calls, ['pause'], 'pause leaves the playhead alone');
  click(w, btn);                                   // shows the hidden "Target" object, which carries the stop trigger
  calls.length = 0;
  click(w, $$(w, '.obj').find((o) => o.textContent === 'Target'));
  assert.deepEqual(calls, ['pause', 'seek 0'], 'stop pauses, then rewinds');
  assert.deepEqual(errors, []);
});

test('a picture cropped to a shape is clipped to it, and a plain one is not', () => {
  const { w, errors } = boot(fixture('maskpic'));
  const img = (alt) => $$(w, 'img').find((i) => i.alt === alt);
  const oval = img('An oval picture').parentNode.style.clipPath;
  assert.match(oval, /^polygon\(/);
  assert.equal(oval.split(',').length, 64, 'an oval of 64 points');
  assert.match(oval, /50(\.00)?% 0(\.00)?%/, 'it starts at the top middle');
  assert.equal(img('A plain picture').parentNode.style.clipPath, '', 'no shape, no clip');
  assert.equal(img('An oval picture').style.clipPath, '', 'the shape belongs to the frame, not to the stretched picture');
  const round = img('A rounded picture').parentNode.style.clipPath;
  assert.match(round, /^polygon\(/);
  assert.ok(round.split(',').length > 30, 'the rounded corners are made of many points');
  // The crop is still applied beneath the shape.
  assert.match(img('An oval picture').style.width, /125/, 'the cropped picture is stretched to fill the frame');
  assert.deepEqual(errors, []);
});

test('web lesson: a cropped picture fills its frame with only the kept part and takes that shape', () => {
  const { w, errors } = boot(fixture('webcrop'));
  const pic = (alt) => $$(w, 'img').find((i) => i.alt === alt);
  const wrap = (alt) => pic(alt).parentNode;
  assert.ok(wrap('Cropped').classList.contains('wl-crop'), 'a cropped picture sits in a clipping frame');
  assert.ok(!wrap('Whole').classList.contains('wl-crop'), 'an uncropped one does not');
  // 25% off each side: the picture is stretched to twice the frame's width and shifted left by half the frame.
  const s = pic('Cropped').style;
  assert.equal(Math.round(parseFloat(s.width)), 200);
  assert.equal(Math.round(parseFloat(s.left)), -50);
  assert.equal(parseFloat(s.top), 0);
  assert.equal(Math.round(parseFloat(s.height)), 100);
  // Top 10% and bottom 40% off: kept height is half the picture.
  const b = pic('Beside').style;
  assert.equal(Math.round(parseFloat(b.height)), 200);
  assert.equal(Math.round(parseFloat(b.top)), -20);
  // When the picture's own size is known, the frame takes the shape of the kept part: 400 x 300 with half the width kept.
  const img = pic('Cropped');
  Object.defineProperty(img, 'naturalWidth', { value: 400 });
  Object.defineProperty(img, 'naturalHeight', { value: 300 });
  img.dispatchEvent(new w.Event('load'));
  assert.equal(wrap('Cropped').style.aspectRatio, '200 / 300');
  // The picture beside text still takes its place on the right.
  assert.ok($(w, '.wl-imagetext').classList.contains('right'));
  assert.deepEqual(errors, []);
});

test('rich text: an empty paragraph and a trailing line break each keep their line, and links are real links', () => {
  const { w, errors } = boot(fixture('richtext'));
  const paras = $$(w, '.obj .label > div');
  assert.equal(paras.length, 4);
  assert.equal(paras[0].textContent, 'First', 'an ordinary paragraph is just its text');
  assert.equal(paras[1].textContent, '\u200b', 'an empty paragraph holds a line open');
  assert.equal(paras[2].textContent, 'Soft\n\u200b', 'a line break at the end leaves a blank line after it');
  const a = paras[3].querySelector('a');
  assert.equal(a.textContent, 'link');
  assert.equal(a.getAttribute('href'), 'https://example.com/');
  assert.deepEqual(errors, []);
});

test('rich text: strikethrough, highlight, letter spacing and paragraph indent reach the page', () => {
  const { w, errors } = boot(fixture('richtext'));
  const paras = $$(w, '.obj .label > div');
  assert.equal(paras[0].style.paddingLeft, '80px', 'two indent steps of 40px');
  assert.equal(paras[1].style.paddingLeft, '', 'no indent, no padding');
  const gone = paras[3].lastElementChild;
  assert.equal(gone.textContent, ' gone');
  assert.match(gone.style.textDecoration, /line-through/);
  assert.equal(gone.style.letterSpacing, '2px');
  assert.match(gone.style.backgroundColor, /255, 240, 0/);
  assert.deepEqual(errors, []);
});

test('list items hang: the marker sits in the margin and wrapped lines line up with the text', () => {
  const { w, errors } = boot(fixture('richtext'));
  const item = $$(w, '.obj .label > div')[3];
  assert.equal(item.style.textIndent, '-1.3em', 'the first line starts one hang to the left of the rest');
  assert.match(item.style.paddingLeft, /1\.3em/, 'every line is set in by the hang');
  const marker = item.firstElementChild;
  assert.equal(marker.textContent, '• ');
  assert.equal(marker.style.display, 'inline-block');
  assert.equal(marker.style.minWidth, '1.3em', 'the text after the marker starts exactly at the hang');
  assert.equal($$(w, '.obj .label > div')[0].style.textIndent, '', 'an ordinary paragraph does not hang');
  assert.deepEqual(errors, []);
});

test('headings are published as real heading elements, and a Heading 1 replaces the hidden slide title as the slide heading', () => {
  const { w, errors } = boot(fixture('richtext'));
  const h2 = $(w, 'h2.label');
  assert.ok(h2, 'a Heading 2 text object is an h2');
  assert.ok(h2.closest('.obj'), 'inside its object');
  const h1 = $(w, 'h1.label');
  assert.equal(h1.textContent, 'Main heading');
  const title = $(w, '#slide-title');
  assert.equal(title.getAttribute('role'), 'group', 'the hidden slide title steps aside for an author-made Heading 1');
  assert.equal(title.getAttribute('aria-label'), 'A');
  assert.equal($$(w, '.obj .label').filter((l) => l.tagName === 'DIV').length > 0, true, 'ordinary text stays a block');
  assert.deepEqual(errors, []);
});

test('a focus order puts the objects in that order for the keyboard and screen readers, and stacking does not change', () => {
  const { w, errors } = boot(fixture('focusorder'));
  const objs = $$(w, '.layer[data-layer] > .obj');
  assert.deepEqual(objs.map((o) => o.textContent), ['THIRD', 'FIRST', 'SECOND'], 'listed ids first, then the rest in stacking order; unknown ids are ignored');
  const z = Object.fromEntries(objs.map((o) => [o.textContent, o.style.zIndex]));
  assert.deepEqual(z, { FIRST: '1', SECOND: '2', THIRD: '3' }, 'the front-to-back stacking is the one the author drew');
  assert.deepEqual(errors, []);
});

test('a web object is a titled frame for an http(s) address only', () => {
  const { w, errors } = boot(fixture('webobject'));
  const frames = $$(w, 'iframe');
  assert.equal(frames.length, 1, 'a javascript: address and an empty one make no frame');
  const f = frames[0];
  assert.equal(f.getAttribute('src'), 'https://example.com/glossary');
  assert.equal(f.getAttribute('title'), 'Course glossary', 'named for screen readers');
  assert.equal(f.getAttribute('scrolling'), 'no');
  assert.match(f.getAttribute('sandbox'), /allow-scripts/);
  assert.equal(f.getAttribute('referrerpolicy'), 'no-referrer');
  assert.ok(f.closest('.obj').classList.contains('media'), 'its label does not cover the page');
  assert.deepEqual(errors, []);
});

test('slide number text shows each slide its own place in the lesson', () => {
  const { w, errors } = boot(fixture('slidenum'));
  const texts = $$(w, '.slide .obj .label').map((l) => l.textContent);
  assert.deepEqual(texts, ['Slide 1 of 3', 'Slide 2 of 3', 'Slide 3 of 3']);
  assert.deepEqual(errors, []);
});

test('radio buttons: choosing one clears the others of its group, and clicking the chosen one changes nothing', () => {
  const { w, errors } = boot(fixture('inputs'));
  const radio = (name) => $$(w, '[role=radio]').find((n) => n.textContent.includes(name));
  const checked = () => $$(w, '[role=radio]').filter((n) => n.getAttribute('aria-checked') === 'true').map((n) => n.textContent.trim());
  assert.deepEqual(checked().sort(), ['one', 'other'], 'one chosen per group to start with');
  radio('two').click();
  assert.deepEqual(checked().sort(), ['other', 'two'], 'two replaces one; the other group is untouched');
  radio('two').click();
  assert.deepEqual(checked().sort(), ['other', 'two'], 'a radio button cannot be unticked by clicking it');
  radio('three').dispatchEvent(new w.KeyboardEvent('keydown', { key: ' ', bubbles: true }));
  assert.deepEqual(checked().sort(), ['other', 'three'], 'the space bar chooses too');
  assert.equal(radio('one').getAttribute('role'), 'radio');
  assert.deepEqual(errors, []);
});

test('entry fields keep what is typed in their variable, and show a variable changed elsewhere', () => {
  const { w, errors } = boot(fixture('inputs'));
  const [name, age] = $$(w, 'input');
  assert.equal(name.type, 'text'); assert.equal(name.value, 'hi', 'starts with the variable');
  assert.equal(age.type, 'number'); assert.equal(age.value, '7'); assert.equal(age.max, '120');
  assert.equal(name.getAttribute('aria-label'), 'Your name', 'labelled from the alt text');
  const hello = () => $$(w, '.obj .label').map((l) => l.textContent).find((t) => t.startsWith('Hello'));
  assert.equal(hello(), 'Hello hi, age 7');
  name.value = 'Sam'; name.dispatchEvent(new w.Event('input', { bubbles: true }));
  age.value = '42'; age.dispatchEvent(new w.Event('input', { bubbles: true }));
  assert.equal(hello(), 'Hello Sam, age 42', 'typing sets the variables');
  age.value = ''; age.dispatchEvent(new w.Event('input', { bubbles: true }));
  assert.equal(hello(), 'Hello Sam, age 42', 'an empty number field leaves the number alone');
  assert.ok(name.closest('.obj').classList.contains('media'), 'the label does not cover the field');
  assert.deepEqual(errors, []);
});

test('markers: the title shows on hover, a click opens the description card, and clicking elsewhere or Escape closes it', () => {
  const { w, errors } = boot(fixture('markers'));
  const marker = (name) => $$(w, '.obj').find((o) => o.getAttribute('aria-label') === name);
  const tv = marker('Testing');
  assert.equal(tv.getAttribute('role'), 'button');
  assert.equal(tv.getAttribute('aria-expanded'), 'false');
  const tip = $$(w, '.marker-tip').find((c) => c.textContent === 'Testing');
  const pop = $(w, '#marker-tv');
  assert.equal(tip.style.display, 'none'); assert.equal(pop.style.display, 'none');
  tv.dispatchEvent(new w.MouseEvent('mouseenter'));
  assert.equal(tip.style.display, 'block', 'hovering shows the title');
  tv.dispatchEvent(new w.MouseEvent('mouseleave'));
  assert.equal(tip.style.display, 'none');
  tv.click();
  assert.equal(pop.style.display, 'block', 'a click opens the card');
  assert.equal(tv.getAttribute('aria-expanded'), 'true');
  assert.equal(pop.getAttribute('role'), 'region');
  assert.match(pop.textContent, /Testing[\s\S]*Testing testing testing/, 'the title, then the description');
  w.document.body.click();
  assert.equal(pop.style.display, 'none', 'clicking elsewhere closes it');
  tv.click();
  assert.equal(pop.style.display, 'block');
  pop.click();
  assert.equal(pop.style.display, 'block', 'clicking the card itself keeps it open');
  w.document.dispatchEvent(new w.KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  assert.equal(pop.style.display, 'none', 'Escape closes it');
  assert.deepEqual(errors, []);
});

test('marker cards sit above the marker when they fit and below when they would leave the slide; a title-only marker opens nothing', () => {
  const { w, errors } = boot(fixture('markers'));
  const marker = (name) => $$(w, '.obj').find((o) => o.getAttribute('aria-label') === name);
  marker('Testing').click();
  const pop = $(w, '#marker-tv');
  assert.equal(pop.style.top, '242px', 'marker at y 600, card 340 high, 18 gap');
  assert.equal(pop.style.left, '640px', 'centred on the marker: 960 - 320');
  w.document.body.click();
  const top = marker('Near the top');
  top.click();
  const below = $(w, '#marker-top');
  assert.equal(below.style.top, '158px', 'no room above (y 20): below, 20 + 120 + 18');
  assert.equal(below.style.left, '1272px', 'kept on the slide: 1920 - 640 - 8');
  w.document.body.click();
  const only = marker('Just a title');
  assert.equal(only.getAttribute('aria-expanded'), null, 'nothing to expand');
  only.click();
  assert.equal($$(w, '.marker-pop').filter((c) => c.style.display === 'block').length, 0);
  assert.deepEqual(errors, []);
});

test('a freeform shape\'s shadow follows its outline, not a box behind it, and a marker animates its cards', () => {
  const { w, errors } = boot(fixture('markers'));
  const tv = $$(w, '.obj').find((o) => o.getAttribute('aria-label') === 'Testing');
  assert.equal(tv.style.borderRadius, '50%', 'a marker is round (so its focus ring is)');
  tv.dispatchEvent(new w.MouseEvent('mouseenter'));
  assert.equal(tv.style.scale, '1.1', 'it swells under the pointer');
  tv.dispatchEvent(new w.MouseEvent('mouseleave'));
  assert.equal(tv.style.scale, '');
  tv.dispatchEvent(new w.MouseEvent('mouseenter'));
  const tip = $$(w, '.marker-tip').find((c) => c.textContent === 'Testing');
  assert.ok(tip.classList.contains('mk-show'), 'the title card plays its entrance animation');
  assert.match(w.document.documentElement.innerHTML, /@keyframes mk-in/, 'and the page defines it');
  assert.deepEqual(errors, []);
});

test('an equation is hidden outlines plus MathML for screen readers, and only a short list of MathML gets through', () => {
  const { w, errors } = boot(fixture('equation'));
  const [half, bad, nomath] = $$(w, '.obj');
  assert.equal(half.getAttribute('role'), 'math');
  assert.equal(half.querySelector('svg').getAttribute('aria-hidden'), 'true', 'the drawing is not read');
  const m = half.querySelector('math');
  assert.ok(m && m.namespaceURI === 'http://www.w3.org/1998/Math/MathML', 'real MathML');
  assert.equal(m.querySelector('mfrac').children.length, 2);
  assert.equal(half.getAttribute('aria-label'), null, 'the MathML is the label when there is no alt text');
  assert.equal(bad.getAttribute('aria-label'), 'The letter x', 'alt text, when given, is the name');
  assert.equal(bad.querySelector('script'), null, 'no script from the MathML');
  assert.equal(bad.querySelector('[onclick]'), null, 'no event attributes');
  assert.equal(bad.querySelector('[href]'), null, 'no links');
  assert.equal(bad.querySelector('mi').getAttribute('mathvariant'), 'italic', 'real attributes stay');
  assert.equal(w.pwned, undefined);
  assert.equal(nomath.getAttribute('aria-label'), String.raw`\SI{5}{m}`, 'no MathML: the LaTeX is the name');
  assert.deepEqual(errors, []);
});

test('a vertical or horizontal line with no width or height still has an svg that can draw', () => {
  const { w, errors } = boot(fixture('richtext'));
  const svg = $(w, '.obj svg');
  assert.equal(svg.getAttribute('width'), '1', 'a width of 100% of nothing would draw nothing');
  assert.equal(svg.getAttribute('height'), '100%');
  assert.equal($(w, '.obj svg line').getAttribute('stroke-width'), '2');
  assert.deepEqual(errors, []);
});

test('the Hidden state keeps an object off the slide until another state is set, and setting Hidden again hides it', () => {
  const { w, errors } = boot(fixture('hiddenstate'));
  const box = $$(w, '.obj').find((o) => o.textContent === 'Box');
  const tip = $$(w, '.obj').find((o) => o.textContent === 'Tip');
  assert.equal(tip.style.display, 'none', 'it starts hidden');
  box.dispatchEvent(new w.MouseEvent('mouseenter'));
  assert.equal(tip.style.display, '', 'hovering shows it');
  box.dispatchEvent(new w.MouseEvent('mouseleave'));
  assert.equal(tip.style.display, 'none', 'leaving puts it back');
  assert.deepEqual(errors, []);
});

test('an outline can be dotted or dashed', () => {
  const { w, errors } = boot(fixture('richtext'));
  const box = $$(w, '.obj').find((o) => o.style.border && /dotted/.test(o.style.border));
  assert.ok(box, 'a dotted outline becomes a dotted CSS border');
  assert.match(box.style.border, /^3px dotted/);
  const line = $(w, '.obj svg line');
  assert.equal(line.getAttribute('stroke-dasharray'), '', 'a solid line has no dash pattern');
  assert.deepEqual(errors, []);
});

test('layers: a layer that does not block lets clicks through to the base layer, and one that hides others replaces the last', () => {
  const { w, errors } = boot(fixture('hiddenstate'));
  const layer = (id) => $(w, `.layer[data-layer=${id}]`);
  const box = $$(w, '.obj').find((o) => o.textContent === 'Box');
  assert.equal(layer('l1').style.pointerEvents, 'none', 'a non-modal layer is click-through');
  assert.equal(layer('l2').style.pointerEvents, '', 'a modal one blocks');
  click(w, box);
  assert.equal(layer('l1').style.display, 'block');
  // The tip is hidden until hovered; show it, then click it: layer 2 replaces layer 1.
  box.dispatchEvent(new w.MouseEvent('mouseenter'));
  const tip = $$(w, '.obj').find((o) => o.textContent === 'Tip');
  click(w, tip);
  assert.equal(layer('l2').style.display, 'block');
  assert.equal(layer('l1').style.display, 'none', 'showing layer 2 hid layer 1');
  assert.deepEqual(errors, []);
});

// A mouse press as a browser sends it: the click only follows if the element that was pressed is still in the page
// when the button is released, so anything that rebuilds the label under the pointer loses the click.
function press(w, el) {
  const leaf = () => el.querySelector('span') || el;
  // The pointer moves here from wherever it was.
  if (w.__pointerOn && w.__pointerOn !== el) w.__pointerOn.dispatchEvent(new w.MouseEvent('mouseleave'));
  w.__pointerOn = el;
  el.dispatchEvent(new w.MouseEvent('mouseenter'));
  const down = leaf();
  down.dispatchEvent(new w.MouseEvent('mousedown', { bubbles: true }));
  const up = leaf();
  up.dispatchEvent(new w.MouseEvent('mouseup', { bubbles: true }));
  if (!down.isConnected) return false;
  up.dispatchEvent(new w.MouseEvent('click', { bubbles: true }));
  return true;
}

test('tabs: a real press on a two-line tab shows its layer, one at a time, and hides the base objects it replaces', () => {
  const { w, errors } = boot(fixture('tabs'));
  const obj = (text) => $$(w, '.obj').find((o) => o.textContent === text);
  const tab = (n) => $$(w, '.obj').find((o) => o.textContent.startsWith(`Label ${n}`));
  const layer = (id) => $(w, `.layer[data-layer="${id}"]`);
  const hiddenByLayer = (text) => obj(text).classList.contains('hidden-by-layer');
  assert.equal(layer('l1').style.display, 'none');
  assert.equal(hiddenByLayer('Base body'), false);
  assert.ok(press(w, tab(1)), 'the press on tab 1 produced a click (its label was not rebuilt under the pointer)');
  assert.deepEqual(['l1', 'l2', 'l3'].map((id) => layer(id).style.display), ['block', 'none', 'none']);
  // The tabs are a button set: the pressed one is Selected (dark), the others stay Normal.
  const look = (n) => (tab(n).style.background.match(/\d+, \d+, \d+/) || [''])[0];
  assert.deepEqual([1, 2, 3].map(look), ['30, 40, 50', '70, 90, 110', '70, 90, 110'], 'tab 1 is Selected');
  assert.equal(hiddenByLayer('Base body'), true, 'the base body text is replaced by the layer\'s');
  assert.equal(hiddenByLayer('Base character'), true);
  assert.equal(hiddenByLayer('Title'), false, 'objects the layer does not list stay');
  assert.equal(tab(2).classList.contains('hidden-by-layer'), false, 'the tabs stay');
  assert.ok(press(w, tab(2)));
  assert.deepEqual(['l1', 'l2', 'l3'].map((id) => layer(id).style.display), ['none', 'block', 'none'], 'tab 2 swaps the layers');
  assert.deepEqual([1, 2, 3].map(look), ['70, 90, 110', '30, 40, 50', '70, 90, 110'], 'selecting tab 2 releases tab 1');
  assert.equal(hiddenByLayer('Base body'), true, 'still replaced');
  assert.ok(press(w, tab(3)));
  assert.deepEqual(['l1', 'l2', 'l3'].map((id) => layer(id).style.display), ['none', 'none', 'block']);
  assert.deepEqual(errors, []);
});

test('a button with a Visited state shows it after the first press, and Hover still applies while the pointer is on it', () => {
  const { w, errors } = boot(fixture('tabs'));
  click(w, $$(w, '#nav button').find((b) => b.textContent === 'Next'));
  const v = $$(w, '.obj').find((o) => o.textContent === 'Visit me');
  const look = () => (v.style.background.match(/\d+, \d+, \d+/) || [''])[0];
  assert.equal(look(), '10, 20, 30', 'Normal at first');
  assert.ok(press(w, v));
  assert.equal(look(), '60, 70, 80', 'pressed and still under the pointer: Hover');
  v.dispatchEvent(new w.MouseEvent('mouseleave'));
  assert.equal(look(), '100, 5, 5', 'the pointer left: Visited');
  v.dispatchEvent(new w.MouseEvent('mouseenter'));
  assert.equal(look(), '60, 70, 80', 'Hover applies to a visited button');
  assert.deepEqual(errors, []);
});

test('layers that do not hide the others stack; the Previous button runs its slide trigger instead of moving back', () => {
  const { w, errors } = boot(fixture('tabs'));
  const next = $$(w, '#nav button').find((b) => b.textContent === 'Next');
  const prev = $$(w, '#nav button').find((b) => b.textContent === 'Previous');
  click(w, next);
  assert.equal(currentSlide(w), 1);
  const btn = (text) => $$(w, '.obj').find((o) => o.textContent === text);
  const layer = (id) => $(w, `.layer[data-layer="${id}"]`);
  assert.ok(press(w, btn('Show one')));
  assert.ok(press(w, btn('Show two')));
  assert.deepEqual(['k1', 'k2'].map((id) => layer(id).style.display), ['block', 'block'], 'both stay');
  // A click on the slide itself is not the Previous button.
  click(w, $(w, '#stage'));
  assert.equal(currentSlide(w), 1, 'clicking the slide does not go back');
  click(w, prev);
  assert.equal(currentSlide(w), 1, 'the slide\'s Previous trigger replaced the move back');
  assert.deepEqual(errors, []);
});

test('a layer that pauses the base layer stops its timeline while it shows, and the timeline runs on when it hides', async () => {
  const { w, errors } = boot(fixture('pause'));
  const obj = (t) => $$(w, '.obj').find((o) => o.textContent === t);
  const shown = (t) => obj(t).style.visibility !== 'hidden';
  assert.equal(shown('Later'), false, 'appears 400 ms in');
  click(w, obj('Open'));
  await sleep(550); // the base timeline would have reached 400 ms by now
  assert.equal(shown('Later'), false, 'paused: still not there');
  click(w, obj('Close'));
  assert.equal(shown('Later'), false, 'the layer hid and the timeline resumed from ~0 ms');
  await sleep(500);
  assert.equal(shown('Later'), true, 'it runs on and the object appears 400 ms after the resume');
  assert.deepEqual(errors, []);
});

// jsdom does no layout, so say how big the sidebar and the top bar are.
const sidebarSizes = (w) => {
  Object.defineProperty(w.HTMLElement.prototype, 'offsetWidth', { configurable: true, get() { return this.id === 'menu' ? 280 : 0; } });
  Object.defineProperty(w.HTMLElement.prototype, 'offsetHeight', { configurable: true, get() { return this.id === 'topbar' ? 44 : 0; } });
};

test('player layout: the banner tops the sidebar, the logo sits small beside the title, and the top bar starts beside an open sidebar', () => {
  const { w, errors } = boot(fixture('playerui'), null, {}, sidebarSizes);
  const menu = $(w, '#menu');
  // The banner is the first thing in the sidebar, above the tabs, and is decoration.
  const banner = $(w, '#menu img.banner');
  assert.ok(banner && banner.src.length > 0, 'the banner shows the author\'s picture');
  assert.equal(banner.getAttribute('alt'), '', 'screen readers skip it');
  assert.equal(menu.firstElementChild, banner, 'it is above the navigation');
  assert.equal($(w, '#menu img.logo'), null, 'the logo is no longer in the sidebar');
  // The logo is small, in the top bar, before the title.
  const logo = $(w, '#topbar img.logo');
  assert.ok(logo && logo.src.length > 0, 'the logo is in the top bar');
  assert.ok(logo.compareDocumentPosition($(w, '#topbar .tb-title')) & w.Node.DOCUMENT_POSITION_FOLLOWING, 'next to the title, before it');
  // The sidebar runs the full height; the top bar starts where it ends, and the slide fits what is left.
  assert.equal(menu.style.top, '0px', 'the sidebar starts at the very top');
  assert.equal($(w, '#topbar').style.left, '280px', 'the top bar moves right of the open sidebar');
  assert.equal($(w, '#topbar').style.right, '0px');
  assert.equal($(w, '#resources').style.right, '8px');
  // Collapsing the sidebar gives the top bar the whole width; opening it moves it back.
  click(w, $(w, '#topbar .tb-toggle'));
  assert.equal($(w, '#topbar').style.left, '0px', 'collapsed: the top bar spans the window');
  click(w, $(w, '#topbar .tb-toggle'));
  assert.equal($(w, '#topbar').style.left, '280px');
  assert.deepEqual(errors, []);
});

test('button styles: a state can be a gradient, so a button keeps its look when hovered', () => {
  const { w, errors } = boot(fixture('buttonstyle'));
  const btn = $$(w, '.obj')[0];
  assert.match(btn.style.background, /linear-gradient\(180deg,rgba\(110,179,215,1\) 0%,rgba\(47,78,162,1\) 100%\)/, 'the resting face is the gradient');
  assert.match(btn.style.boxShadow, /^0px 4px 3\.5px 0px /, 'with its drop shadow');
  btn.dispatchEvent(new w.MouseEvent('mouseenter'));
  assert.match(btn.style.background, /linear-gradient\(180deg,rgba\(121,198,225,1\) 0%,rgba\(35,59,152,1\) 100%\)/, 'hover is a different gradient, not a flat colour');
  btn.dispatchEvent(new w.MouseEvent('mouseleave'));
  assert.match(btn.style.background, /rgba\(110,179,215,1\)/, 'and it goes back');
  assert.deepEqual(errors, []);
});

test('button effects: a glow is a halo, soft edges fade along the outline, a reflection mirrors it, and none of it touches a plain button', () => {
  const { w, errors } = boot(fixture('buttoneffects'));
  const order = ['glow', 'soft', 'reflect', 'plain', 'star', 'starglow', 'oval'];   // the example's objects, in order
  const node = (name) => $$(w, '.obj')[order.indexOf(name)];
  assert.match(node('glow').style.boxShadow, /^0 0 24px 6px /, 'a glow is a blurred shadow with no offset');
  // A softened box draws its fill on a smaller, blurred layer behind its text, so rounded corners stay rounded.
  const soft = node('soft'); const back = soft.querySelector('.soft-back');
  assert.ok(back, 'the fill moves to a backing layer');
  assert.equal(back.style.filter, 'blur(6px)');
  assert.equal(back.style.inset, '12px');
  assert.equal(back.style.borderRadius, '0px', 'its corners follow the box (a 10 px radius shrunk by the 12 px inset)');
  assert.match(back.style.background, /rgb\(3, 83, 158\)/, 'the fill is the box\'s own');
  assert.ok(soft.style.background === 'none' || soft.style.background === '', 'and the box itself has none');
  assert.equal(soft.style.boxShadow, 'none');
  assert.equal(soft.textContent, 'soft', 'the text is not part of the fade');
  assert.equal(node('oval').querySelector('.soft-back').style.borderRadius, '50%', 'an oval fades as an oval');
  // A freeform shape is blurred whole.
  const starSvg = node('star').querySelector('svg');
  assert.equal(starSvg.style.filter, 'blur(6px)');
  assert.equal(starSvg.style.overflow, 'visible', 'so its tips are not cut off');
  // A freeform shape's glow follows its outline; its reflection mirrors the element.
  assert.match(node('starglow').querySelector('svg').style.filter, /drop-shadow\(0 0 12px /);
  assert.match(node('reflect').style.webkitBoxReflect, /^below 6px linear-gradient\(to bottom,transparent,transparent 40%,rgba\(255,255,255,0\.5\)\)$/, 'a reflection shows 60% of the height, strongest at the object');
  const plain = node('plain');
  assert.ok(!plain.style.webkitMaskImage && !plain.style.webkitBoxReflect && !plain.querySelector('.soft-back'), 'a plain button has none of it');
  assert.ok(!/24px/.test(plain.style.boxShadow));
  assert.deepEqual(errors, []);
});

// ---- Play / pause, seekbar and replay ------------------------------------------------------------
const transport = (w) => ({
  play: $(w, '.ctl-play'), replay: $(w, '.ctl-replay'), seek: $(w, 'input.ctl-seek'), time: $(w, '.ctl-time'),
  shown: (name) => $$(w, '.obj').find((o) => o.textContent === name).style.visibility !== 'hidden',
});
const seekTo = (w, t, ms) => { t.seek.value = String(ms); t.seek.dispatchEvent(new w.Event('input', { bubbles: true })); t.seek.dispatchEvent(new w.Event('change', { bubbles: true })); };

test('player controls: play/pause, a seekbar and replay appear when asked for, and follow the slide\'s timeline', async () => {
  const { w, errors } = boot(fixture('transport'));
  const t = transport(w);
  assert.ok(t.play && t.replay && t.seek && t.time, 'all three are in the control bar');
  assert.equal(t.seek.getAttribute('aria-label'), 'Seek');
  assert.equal(t.seek.max, '3000', 'the bar is as long as the slide\'s timeline');
  assert.equal(t.play.getAttribute('aria-label'), 'Pause', 'the slide is playing, so the button offers Pause');
  assert.equal(t.seek.disabled, false);
  await sleep(1150);
  assert.ok(Number(t.seek.value) >= 1000 && Number(t.seek.value) < 1500, 'the bar follows the timeline: ' + t.seek.value);
  assert.match(t.time.textContent, /^0:01 \/ 0:03$/);
  assert.match(t.seek.getAttribute('aria-valuetext'), /^0:01 of 0:03$/);
  assert.equal(t.shown('early'), false, 'and the objects follow it too');
  assert.deepEqual(errors, []);
});

test('player controls: pause holds the timeline and play runs it on from there', async () => {
  const { w, errors } = boot(fixture('transport'));
  const t = transport(w);
  await sleep(200);
  click(w, t.play);
  assert.equal(t.play.getAttribute('aria-label'), 'Play', 'paused: the button now offers Play');
  const held = t.seek.value;
  await sleep(1500);
  assert.equal(t.seek.value, held, 'the bar does not move while paused');
  assert.equal(t.shown('late'), false, 'and "late" (due at 1.5 s) has not appeared');
  click(w, t.play);
  assert.equal(t.play.getAttribute('aria-label'), 'Pause');
  await sleep(300);
  assert.ok(Number(t.seek.value) > Number(held), 'play runs on from where it stopped');
  assert.ok(Number(t.seek.value) < 1000, 'not from where the clock would have been');
  assert.deepEqual(errors, []);
});

test('player controls: the seekbar jumps to a moment, even while paused', async () => {
  const { w, errors } = boot(fixture('transport'));
  const t = transport(w);
  seekTo(w, t, 2000);
  assert.equal(t.shown('late'), true, 'at 2 s "late" is on');
  assert.equal(t.shown('early'), false, 'and "early" (gone at 1 s) is off');
  await sleep(150);
  assert.ok(Number(t.seek.value) >= 2000, 'it plays on from there');
  click(w, t.play);
  seekTo(w, t, 500);
  assert.equal(t.shown('early'), true, 'back at 0.5 s "early" is on again');
  assert.equal(t.shown('late'), false);
  assert.equal(t.play.getAttribute('aria-label'), 'Play', 'seeking while paused stays paused');
  const at = t.seek.value;
  await sleep(300);
  assert.equal(t.seek.value, at);
  // Dragging only moves the readout until it is let go.
  t.seek.value = '1000'; t.seek.dispatchEvent(new w.Event('input', { bubbles: true }));
  assert.equal(t.time.textContent, '0:01 / 0:03');
  assert.equal(t.shown('early'), true, 'nothing moves until the drag ends');
  assert.deepEqual(errors, []);
});

test('player controls: replay starts the slide again, and slides with nothing to play grey the controls out', async () => {
  const { w, errors } = boot(fixture('transport'));
  const t = transport(w);
  seekTo(w, t, 2500);
  assert.equal(t.shown('late'), true);
  click(w, t.replay);
  assert.equal(t.shown('late'), false, 'replay is back at the start');
  assert.equal(t.shown('early'), true);
  assert.ok(Number(t.seek.value) < 500);
  assert.equal(t.play.getAttribute('aria-label'), 'Pause', 'and playing');
  // The second slide has no timeline and no media.
  click(w, $$(w, '#nav button').find((b) => b.textContent === 'Next'));
  assert.equal(currentSlide(w), 1);
  assert.equal(t.play.disabled && t.replay.disabled && t.seek.disabled, true, 'nothing to play: all three are greyed out');
  click(w, $$(w, '#nav button').find((b) => b.textContent === 'Previous'));
  assert.equal(t.play.disabled || t.replay.disabled || t.seek.disabled, false, 'and back on the timed slide they work again');
  assert.deepEqual(errors, []);
});

test('player controls: a slide with a video is driven through the video', () => {
  const { w, errors } = boot(fixture('transport'), null, {}, (win) => {
    win.HTMLMediaElement.prototype.play = function () { this.__paused = false; return Promise.resolve(); };
    win.HTMLMediaElement.prototype.pause = function () { this.__paused = true; };
    Object.defineProperty(win.HTMLMediaElement.prototype, 'paused', { configurable: true, get() { return this.__paused !== false; } });
    Object.defineProperty(win.HTMLMediaElement.prototype, 'duration', { configurable: true, get() { return 90; } });
  });
  const t = transport(w);
  click(w, $$(w, '#nav button').find((b) => b.textContent === 'Next'));
  const slide = $$(w, '.slide')[1];
  const v = w.document.createElement('video'); slide.querySelector('.layer').appendChild(v);
  w.document.dispatchEvent(new w.Event('loadedmetadata'));
  assert.equal(t.seek.max, '90000', 'the bar is as long as the video');
  assert.equal(t.play.disabled, false);
  assert.equal(t.play.getAttribute('aria-label'), 'Play', 'it is not playing yet');
  click(w, t.play);
  assert.equal(v.paused, false);
  assert.equal(t.play.getAttribute('aria-label'), 'Pause');
  seekTo(w, t, 30000);
  assert.equal(v.currentTime, 30);
  click(w, t.replay);
  assert.equal(v.currentTime, 0);
  assert.deepEqual(errors, []);
  w.close();   // the fake video plays forever, and the bar keeps following it
});

test('player controls: icons, icon-only buttons keep their names, and the controls are off unless asked for', () => {
  const { w } = boot(fixture('transport-icons'));
  const t = transport(w);
  assert.ok(t.play.querySelector('svg'), 'an icon');
  assert.equal(t.play.textContent, '');
  assert.equal(t.play.getAttribute('aria-label'), 'Pause');
  assert.equal(t.replay.getAttribute('aria-label'), 'Replay');
  const { w: plain } = boot(fixture('playerui-icons'));
  assert.equal($(plain, '.ctl-play') || $(plain, '.ctl-replay') || $(plain, '.ctl-seek'), null, 'not shown unless chosen');
});

test('player layout: the sidebar slides in and out, the top bar moves at once, and the menu hides when the slide ends', () => {
  const anims = [];
  const { w, errors } = boot(fixture('playerui'), null, {}, (win) => {
    sidebarSizes(win);
  });
  const menu = $(w, '#menu'); const bar = $(w, '#topbar');
  menu.animate = function (frames) { const a = { frames, cancel() {} }; anims.push(a); return a; };
  click(w, $(w, '#topbar .tb-toggle'));
  assert.equal(anims.length, 1, 'closing starts a slide');
  assert.equal(anims[0].frames[1].transform, 'translateX(-100%)', 'it slides off to the left');
  assert.equal(menu.hidden, false, 'it stays on screen while it slides');
  assert.equal(bar.style.left, '0px', 'the top bar starts moving at once');
  assert.ok(w.document.body.classList.contains('side-anim'), 'the layout glides rather than jumps');
  anims[0].onfinish();
  assert.equal(menu.hidden, true, 'and is gone when the slide ends');
  click(w, $(w, '#topbar .tb-toggle'));
  assert.equal(menu.hidden, false);
  assert.equal(anims[1].frames[0].transform, 'translateX(-100%)', 'opening slides in from the left');
  assert.equal(bar.style.left, '280px');
  assert.deepEqual(errors, []);
});

test('player layout: with a sidebar on the right the top bar stops short of it', () => {
  const { w, errors } = boot(fixture('playerui-right'), null, {}, sidebarSizes);
  assert.equal($(w, '#topbar').style.right, '280px');
  assert.equal($(w, '#topbar').style.left, '0px');
  assert.equal($(w, '#resources').style.right, '288px', 'the Resources popup stays under its button');
  assert.ok($(w, '#menu img.banner'), 'the banner tops the right-hand sidebar too');
  assert.equal($(w, '#menu').style.top, '0px');
  assert.deepEqual(errors, []);
});

test('player layout: with no sidebar the banner is not shown, but the small logo still is', () => {
  const { w, errors } = boot(fixture('playerui-icons'));
  assert.equal($(w, '#menu.sidebar'), null, 'no sidebar in this fixture (the menu is only a slide-over drawer)');
  assert.equal($(w, 'img.banner'), null, 'a banner belongs to the sidebar, so it is hidden without one');
  assert.ok($(w, '#topbar img.logo'), 'the logo is in the top bar');
  assert.equal($(w, '#topbar').style.left, '0px');
  assert.deepEqual(errors, []);
});

test('player layout: a project with no banner has none, whatever else it has', () => {
  const { w, errors } = boot(fixture('authored'));
  assert.equal($(w, 'img.banner'), null);
  assert.deepEqual(errors, []);
});

test('player sidebar: menu and notes tabs, the title and the logo, a toggle that collapses it, and a right-hand side', () => {
  const { w, errors } = boot(fixture('playerui'));
  const menu = $(w, '#menu');
  assert.ok(menu.classList.contains('sidebar') && menu.classList.contains('side-left'));
  assert.equal(menu.hidden, false, 'the sidebar starts open');
  assert.equal($(w, '#topbar .tb-title').textContent, 'Safety Basics', 'the title falls back to the project name');
  assert.ok($(w, '#menu img.banner').src.length > 0, 'the banner sits above the navigation');
  assert.ok($(w, '#topbar img.logo').src.length > 0, 'the logo sits beside the title');
  const tab = (name) => $$(w, '#menu .sb-tabs button').find((b) => b.textContent === name);
  assert.deepEqual($$(w, '#menu .sb-tabs button[role=tab]').map((b) => b.textContent), ['Menu', 'Glossary', 'Notes']);
  assert.equal($$(w, '#menu button[data-index]').length, 3, 'one menu entry per slide');
  // Notes: the current slide's, and a hint when it has none.
  click(w, tab('Notes'));
  assert.match($(w, '#menu .notes').textContent, /Greet the learners/);
  click(w, $$(w, '#nav button').find((b) => b.textContent === 'Next'));
  assert.equal($(w, '#menu .notes').textContent, 'No notes for this slide.');
  click(w, $$(w, '#nav button').find((b) => b.textContent === 'Next'));
  assert.match($(w, '#menu .notes').textContent, /Summarise the three rules/);
  // The glossary has its own tab (alphabetical); resources sit at the right of the top bar, web links only.
  click(w, tab('Glossary'));
  assert.deepEqual($$(w, '#menu dt').map((d) => d.textContent), ['Hazard', 'PPE']);
  const rbtn = $(w, '#topbar .tb-resources');
  assert.equal($(w, '#resources').hidden, true);
  click(w, rbtn);
  assert.deepEqual($$(w, '#resources a').map((a) => [a.textContent, a.getAttribute('href'), a.target]), [['Safety handbook', 'https://example.com/handbook', '_blank']], 'the javascript: link is left out');
  click(w, rbtn);
  assert.equal($(w, '#resources').hidden, true);
  // Search filters the slide list.
  click(w, $(w, '#menu .sb-search'));
  const sin = $(w, '#menu .sb-searchbox input');
  sin.value = 'zzzz'; sin.dispatchEvent(new w.Event('input', { bubbles: true }));
  assert.equal($$(w, '#menu li.nomatch').length, 3, 'nothing matches');
  const first = $$(w, '#menu button[data-index]')[0].textContent;
  sin.value = first.slice(0, 3); sin.dispatchEvent(new w.Event('input', { bubbles: true }));
  assert.ok($$(w, '#menu li:not(.nomatch) button[data-index]').length >= 1);
  click(w, $(w, '#menu .sb-search'));
  assert.equal($$(w, '#menu li.nomatch').length, 0, 'closing the search clears it');
  // The control bar sits beside the sidebar, under the slide only.
  assert.equal($(w, '#nav').style.left, '280px'.replace('280', String($(w, '#menu').offsetWidth)));
  click(w, tab('Menu'));
  click(w, $$(w, '#menu button[data-index]')[0]);
  assert.equal(currentSlide(w), 0, 'the menu still moves between slides');
  // The toggle in the top bar hides and shows the sidebar.
  click(w, $(w, '#topbar .tb-toggle'));
  assert.equal(menu.hidden, true);
  click(w, $(w, '#topbar .tb-toggle'));
  assert.equal(menu.hidden, false);
  assert.deepEqual(errors, []);
  const right = boot(fixture('playerui-right')).w;
  assert.ok($(right, '#menu').classList.contains('side-right'));
});

test('player buttons and controls: text by default, icons with words on request; volume, captions and speed; the theme colours', () => {
  const plain = boot(fixture('playerui')).w;
  assert.deepEqual($$(plain, '#nav button').map((b) => b.textContent), ['Previous', 'Next'], 'text buttons by default');
  assert.equal($(plain, '#nav .nav-controls'), null, 'no controls unless asked for');
  const { w, errors } = boot(fixture('playerui-icons'));
  const next = $$(w, '#nav button').find((b) => b.getAttribute('aria-label') === 'Next');
  assert.ok(next.querySelector('svg') && next.textContent === 'Next', 'icon and words');
  assert.ok($(w, '#menu').hidden && !$(w, '#menu').classList.contains('sidebar'), 'no sidebar was asked for: the slide menu is a drawer, closed');
  const ctl = (id) => $(w, `#nav .ctl-${id}`);
  assert.ok(ctl('volume') && ctl('captions') && ctl('speed'), 'the chosen controls are in the bar');
  click(w, ctl('volume'));
  assert.equal(ctl('volume').getAttribute('aria-pressed'), 'true');
  assert.equal(ctl('volume').getAttribute('aria-label'), 'Unmute');
  click(w, ctl('captions'));
  assert.equal(ctl('captions').getAttribute('aria-pressed'), 'false', 'captions can be turned off');
  assert.equal(ctl('speed').textContent, '1x');
  click(w, ctl('speed'));
  assert.equal(ctl('speed').textContent, '1.25x');
  // A light theme with the author's accent colour reaches the page as custom properties.
  const css = $$(w, 'style').map((s) => s.textContent).join('');
  assert.match(css, /--p-panel:#f4f6f9/);
  assert.match(css, /--p-accent:#e87722/);
  assert.deepEqual(errors, []);
});

test('a video or audio object is not covered by its own label, so the player controls take clicks', () => {
  const { w, errors } = boot(fixture('actions'));
  const video = $(w, 'video');
  const node = video.parentNode;
  assert.ok(node.classList.contains('media'));
  const label = node.querySelector('.label');
  assert.ok(label, 'the object has a label layer over the video');
  assert.equal(w.getComputedStyle(label).pointerEvents, 'none', 'clicks pass through it to the controls');
  assert.deepEqual(errors, []);
});

test('a question slide carries its own shapes, buttons and triggers beside the question', () => {
  const { w, errors } = boot(fixture('qobjs'));
  const obj = (t) => $$(w, '.obj').find((o) => o.textContent === t);
  assert.ok($(w, '.q .q-prompt'), 'the question is drawn');
  const area = $(w, '.q-place');
  assert.ok(area && area.querySelector('.q'), 'the question sits in its own area');
  assert.equal(area.style.left, '100px');
  assert.equal(area.querySelector('.q').style.background, 'transparent', 'the slide shows around it');
  assert.ok($$(w, '.obj').length >= 3, 'the slide objects are built next to it');
  assert.equal(obj('Think about the colour of the sky').style.display, 'none', 'the note starts hidden');
  click(w, obj('Hint'));
  assert.notEqual(obj('Think about the colour of the sky').style.display, 'none', 'the button shows the note');
  assert.ok($(w, '.q'), 'the question was not wiped');
  assert.deepEqual(errors, []);
});

// ---- 360° images ----------------------------------------------------------------------------------------
// jsdom has no WebGL: a recording stand-in lets the viewer's controls, keys and annotations be tested; without one the
// viewer falls back to the flat picture.
function fakeGL(w) {
  const calls = { draw: 0, uniforms: {} };
  const gl = new Proxy({}, {
    get(_, name) {
      if (name === 'calls') return calls;
      if (name === 'drawArrays') return () => { calls.draw++; };
      if (/^[A-Z_0-9]+$/.test(name)) return 1;
      if (name === 'getParameter') return () => 4096;
      if (name === 'getShaderParameter' || name === 'getProgramParameter') return () => true;
      if (name === 'getUniformLocation') return (_p, n) => n;
      if (/^uniform/.test(name)) return (loc, ...v) => { calls.uniforms[loc] = v; };
      return () => ({});
    },
  });
  return gl;
}
// A turning viewer asks for animation frames for ever, which would keep node alive: close those windows at the end.
const glWindows = [];
test.after(() => glWindows.forEach((w) => w.close()));
const withGL = (w) => {
  glWindows.push(w);
  w.__gl = fakeGL(w);
  w.HTMLCanvasElement.prototype.getContext = function () { return w.__gl; };
  // jsdom lays nothing out; say the viewer is on screen.
  w.Element.prototype.getClientRects = function () { return [{}]; };
};
const panoBoot = () => boot(fixture('panorama'), null, {}, withGL);
const panoNode = (w, id) => $$(w, '.obj').find((o) => o.getAttribute('aria-label')?.startsWith(id));

test('a 360° image is one application region named for the scene, with a canvas and real buttons', () => {
  const { w, errors } = panoBoot();
  const tour = panoNode(w, 'A teaching laboratory');
  assert.equal(tour.getAttribute('role'), 'application');
  assert.equal(tour.getAttribute('aria-roledescription'), '360° image');
  assert.match(tour.getAttribute('aria-label'), /^A teaching laboratory with a door and a fume hood\. Drag, or use the arrow keys, to look around; plus and minus zoom\.$/);
  assert.equal(tour.tabIndex, 0, 'the keyboard can reach it');
  assert.equal(tour.querySelector('canvas').getAttribute('aria-hidden'), 'true', 'the drawing is not read');
  const names = [...tour.querySelectorAll('.pano-controls button')].map((b) => b.getAttribute('aria-label'));
  assert.deepEqual(names, ['Pause turning', 'Zoom out', 'Zoom in', 'Back to the starting view', 'Markers and hotspots']);
  assert.ok(w.__gl.calls.draw >= 0);
  assert.deepEqual(errors, []);
});

test('annotations in view are buttons over the picture; those out of view are reached from the list', async () => {
  const { w, errors } = panoBoot();
  const tour = panoNode(w, 'A teaching laboratory');
  await sleep(60);
  const pins = [...tour.querySelectorAll(':scope > button[aria-haspopup]')];
  assert.equal(pins.length, 3);
  assert.equal(pins[0].getAttribute('aria-label'), 'Door');
  assert.equal(pins[0].style.display, 'flex', 'the door is ahead of the viewer');
  assert.equal(pins[1].style.display, 'none', 'the fume hood is behind the viewer');
  assert.equal(pins[2].getAttribute('aria-label'), 'Bench', 'the hotspot has a keyboard stop');
  // The door opens a card with its text; the text is not parsed as HTML.
  click(w, pins[0]);
  const card = $(w, '.pano-card');
  assert.equal(card.style.display, 'block');
  assert.equal(card.getAttribute('role'), 'region');
  assert.equal(card.getAttribute('aria-label'), 'Door');
  assert.equal(card.querySelector('.pano-card-title').textContent, 'Door');
  assert.match(card.querySelector('.pano-card-body').textContent, /The way out\. <img src=x/);
  assert.equal(card.querySelector('img'), null);
  assert.equal(pins[0].getAttribute('aria-expanded'), 'true');
  // Escape closes it and returns to the annotation.
  tour.dispatchEvent(new w.KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  assert.equal(card.style.display, 'none');
  assert.equal(pins[0].getAttribute('aria-expanded'), 'false');
  // Clicking elsewhere also closes it.
  click(w, pins[0]);
  click(w, w.document.body);
  assert.equal(card.style.display, 'none');
  // The list reaches the one behind the viewer: the view turns to it and its card opens.
  const list = $(w, '.pano-list');
  assert.equal(list.style.display, 'none');
  click(w, tour.querySelector('.pano-controls button[aria-label="Markers and hotspots"]'));
  assert.equal(list.style.display, 'block');
  const items = [...list.querySelectorAll('button')].map((b) => b.textContent);
  assert.deepEqual(items, ['1  Door', '2  Fume hood', '3  Marker 3', '4  Lamp', '5  Bench']);
  click(w, list.querySelectorAll('button')[1]);
  assert.equal(list.style.display, 'none');
  await sleep(900);
  assert.equal(pins[1].style.display, 'flex', 'the view has turned to the fume hood');
  assert.equal(card.getAttribute('aria-label'), 'Fume hood');
  assert.equal(card.style.display, 'block');
  assert.equal(w.pwned, undefined);
  assert.deepEqual(errors, []);
});

test('an annotation set to be a button runs its triggers and opens no card', async () => {
  const { w, errors } = panoBoot();
  const tour = panoNode(w, 'A teaching laboratory');
  await sleep(60);
  const lamp = [...tour.querySelectorAll(':scope > button')].find((b) => b.getAttribute('aria-label') === 'Lamp');
  assert.equal(lamp.style.display, 'flex');
  assert.equal(lamp.getAttribute('aria-haspopup'), null, 'it does not promise a card');
  click(w, lamp);
  assert.equal(w.lampPressed, 1);
  const card = tour.querySelector('.pano-card');
  assert.ok(!card || card.style.display === 'none', 'no card');
  // The list runs it too.
  click(w, tour.querySelector('.pano-controls button[aria-label="Markers and hotspots"]'));
  click(w, [...$(w, '.pano-list').querySelectorAll('button')][3]);
  await sleep(700); // the view turns to it first
  assert.equal(w.lampPressed, 2);
  assert.deepEqual(errors, []);
});

test('a marker has its colour and icon, a hotspot is a shaded area, and each pulses until it is visited', async () => {
  const { w, errors } = panoBoot();
  const tour = panoNode(w, 'A teaching laboratory');
  await sleep(60);
  const door = [...tour.querySelectorAll(':scope > button')].find((b) => b.getAttribute('aria-label') === 'Door');
  assert.match(door.style.background, /rgb\(79, 129, 189\)/, 'the default marker colour');
  assert.match(door.querySelector('svg path').getAttribute('d'), /^M0\.25 0\.25L0\.75 0\.25/, 'its icon');
  assert.ok(door.classList.contains('pano-pulse'), 'unvisited pulses');
  const poly = tour.querySelector('svg polygon');
  assert.equal(poly.getAttribute('points').split(' ').length, 4);
  assert.equal(poly.getAttribute('fill'), 'rgb(200,40,40)');
  assert.equal(tour.querySelector('.pano-progress').textContent, 'Visited 0 of 5');
  click(w, door);
  assert.ok(!door.classList.contains('pano-pulse'), 'visiting stops the pulse');
  assert.equal(tour.querySelector('.pano-progress').textContent, 'Visited 1 of 5');
  // The shaded area takes the press and opens its card.
  click(w, poly);
  assert.equal($(w, '.pano-card').getAttribute('aria-label'), 'Bench');
  assert.equal(tour.querySelector('.pano-progress').textContent, 'Visited 2 of 5');
  assert.equal(w.lampPressed, undefined);
  assert.deepEqual(errors, []);
});

test('a guided tour shows one item at a time with Previous and Next, a light square card, and a hint for a hidden hotspot', async () => {
  const { w, errors } = panoBoot();
  const tour = panoNode(w, 'A guided tour of the lab');
  await sleep(700);
  const bar = tour.querySelector('.pano-tour');
  const [prev, next] = [...bar.querySelectorAll('button')];
  assert.equal(prev.disabled, true);
  assert.equal(next.disabled, false);
  assert.equal(tour.querySelector('.pano-progress').textContent, 'Step 1 of 3');
  let card = $(w, '.pano-card', tour);
  assert.equal(card.getAttribute('aria-label'), 'Start');
  assert.match(card.style.borderRadius, /^0(px)?$/, 'square corners');
  assert.match(card.style.background, /rgb\(255, 255, 255\)|#fff/, 'a light card');
  click(w, next);
  await sleep(700);
  assert.equal(tour.querySelector('.pano-progress').textContent, 'Step 2 of 3');
  assert.equal(tour.querySelector('.pano-card').getAttribute('aria-label'), 'Shelf');
  assert.equal(bar.querySelector('.pano-hint').textContent, 'Hint: Behind you, on the right');
  const first = [...tour.querySelectorAll(':scope > button')].find((b) => b.getAttribute('aria-label') === 'Start');
  assert.equal(first.style.display, 'none', 'only the current item is shown');
  click(w, next);
  assert.equal(next.disabled, true, 'the last step');
  assert.equal(bar.querySelector('.pano-hint').style.display, 'none');
  assert.deepEqual(errors, []);
});

test('arrow keys look around, plus and minus zoom, and any of them stops the automatic turning', async () => {
  const { w, errors } = panoBoot();
  const tour = panoNode(w, 'A teaching laboratory');
  await sleep(40);
  const pause = tour.querySelector('.pano-controls button');
  assert.equal(pause.getAttribute('aria-label'), 'Pause turning');
  assert.equal(pause.getAttribute('aria-pressed'), 'false');
  const key = (k) => tour.dispatchEvent(new w.KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true }));
  tour._panoReady = true; // jsdom never loads the picture
  const draws = w.__gl.calls.draw;
  key('ArrowRight');
  assert.equal(pause.getAttribute('aria-label'), 'Turn automatically', 'taking over stops the turning');
  assert.equal(pause.getAttribute('aria-pressed'), 'true');
  await sleep(60);
  assert.ok(w.__gl.calls.draw > draws, 'the view was drawn again');
  const fov0 = w.__gl.calls.uniforms.t[0];
  key('+'); await sleep(40);
  assert.ok(w.__gl.calls.uniforms.t[0] < fov0, 'zooming in narrows the view');
  key('-'); key('-'); await sleep(40);
  assert.ok(w.__gl.calls.uniforms.t[0] > fov0, 'and out widens it');
  // The pause button is a toggle.
  click(w, pause);
  assert.equal(pause.getAttribute('aria-label'), 'Pause turning');
  click(w, pause);
  assert.equal(pause.getAttribute('aria-label'), 'Turn automatically');
  assert.deepEqual(errors, []);
});

test('a 360° image without automatic turning or zoom has only the buttons it needs', () => {
  const { w, errors } = panoBoot();
  const plain = panoNode(w, 'Plain');
  assert.doesNotMatch(plain.getAttribute('aria-label'), /zoom/);
  assert.deepEqual([...plain.querySelectorAll('.pano-controls button')].map((b) => b.getAttribute('aria-label')), ['Back to the starting view']);
  assert.equal(plain.querySelector('.pano-list'), null, 'no annotations, no list');
  assert.deepEqual(errors, []);
});

test('without WebGL a 360° image shows the picture scrolling sideways, and annotations still open from the list', () => {
  const { w, errors } = boot(fixture('panorama'));
  const tour = panoNode(w, 'A teaching laboratory');
  assert.equal(tour.getAttribute('role'), 'group');
  assert.equal(tour.getAttribute('aria-roledescription'), null);
  const flat = tour.querySelector('.pano-flat');
  assert.ok(flat && flat.querySelector('img').getAttribute('src').startsWith('assets/'), 'the flat picture');
  assert.equal(flat.getAttribute('role'), 'region');
  assert.match(flat.getAttribute('aria-label'), /scrolls sideways/);
  assert.equal(tour.querySelector('canvas').style.display, 'none');
  assert.deepEqual([...tour.querySelectorAll('.pano-controls button')].filter((b) => b.style.display !== 'none').map((b) => b.getAttribute('aria-label')), ['Markers and hotspots']);
  click(w, tour.querySelector('.pano-controls button[aria-label="Markers and hotspots"]'));
  click(w, tour.querySelectorAll('.pano-list button')[0]);
  assert.equal($(w, '.pano-card').getAttribute('aria-label'), 'Door');
  assert.deepEqual(errors, []);
});

test('accessibility: a 360° image passes the automated audit with and without WebGL', async () => {
  for (const before of [withGL, null]) {
    const { w } = boot(fixture('panorama'), null, {}, before);
    assert.deepEqual(await audit(w), [], before ? 'with WebGL' : 'flat');
  }
});
