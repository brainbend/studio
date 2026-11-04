// Generates exports with the Rust examples into tests/player/.fixtures (git-ignored).
const { execFileSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../..');
const out = path.join(__dirname, '.fixtures');
fs.rmSync(out, { recursive: true, force: true });
fs.mkdirSync(out, { recursive: true });

function cargo(example, args) {
  execFileSync('cargo', ['run', '-q', '-p', 'brainbend-publish-html', '--example', example, '--', ...args], { cwd: root, stdio: 'inherit' });
}
function unzip(zip, dir) {
  fs.mkdirSync(dir, { recursive: true });
  execFileSync('unzip', ['-q', '-o', zip, '-d', dir]);
}

cargo('authored', [path.join(out, 'authored')]);
cargo('quiz', [path.join(out, 'quiz')]);
cargo('master', [path.join(out, 'master')]);
cargo('timeline', [path.join(out, 'timeline')]);
cargo('actions', [path.join(out, 'actions')]);
cargo('qobjs', [path.join(out, 'qobjs')]);
cargo('nav', [path.join(out, 'nav')]);
cargo('anim', [path.join(out, 'anim')]);
cargo('a11y', [path.join(out, 'a11y')]);
cargo('effects', [path.join(out, 'effects')]);
cargo('maskpic', [path.join(out, 'maskpic')]);
cargo('webcrop', [path.join(out, 'webcrop')]);
cargo('richtext', [path.join(out, 'richtext')]);
cargo('focusorder', [path.join(out, 'focusorder')]);
cargo('webobject', [path.join(out, 'webobject')]);
cargo('slidenum', [path.join(out, 'slidenum')]);
cargo('inputs', [path.join(out, 'inputs')]);
cargo('markers', [path.join(out, 'markers')]);
cargo('equation', [path.join(out, 'equation')]);
cargo('panorama', [path.join(out, 'panorama')]);
cargo('hiddenstate', [path.join(out, 'hiddenstate')]);
cargo('clicks', [path.join(out, 'clicks')]);
cargo('tabs', [path.join(out, 'tabs')]);
cargo('pause', [path.join(out, 'pause')]);
cargo('playerui', [path.join(out, 'playerui')]);
cargo('playerui', [path.join(out, 'playerui-right'), 'right']);
cargo('playerui', [path.join(out, 'playerui-icons'), 'off', 'icons']);
cargo('buttoneffects', [path.join(out, 'buttoneffects')]);
cargo('buttonstyle', [path.join(out, 'buttonstyle')]);
cargo('transport', [path.join(out, 'transport')]);
cargo('transport', [path.join(out, 'transport-icons'), 'icons']);
cargo('bank', [path.join(out, 'bank')]);
cargo('bank', [path.join(out, 'bank12.zip'), 'scorm12']);
unzip(path.join(out, 'bank12.zip'), path.join(out, 'bank12'));
cargo('quiz2', [path.join(out, 'quiz2')]);
cargo('weblesson', [path.join(out, 'weblesson')]);
cargo('weblesson', [path.join(out, 'weblesson12.zip'), 'scorm12']);
unzip(path.join(out, 'weblesson12.zip'), path.join(out, 'weblesson12'));
cargo('controls', [path.join(out, 'controls')]);
cargo('course', [path.join(out, 'course12.zip')]);
unzip(path.join(out, 'course12.zip'), path.join(out, 'course12'));
cargo('quiz3', [path.join(out, 'quiz3')]);
cargo('quiz4', [path.join(out, 'quiz4')]);
for (const v of ['12', '2004']) {
  cargo('quiz', [path.join(out, `quiz${v}.zip`), `scorm${v}`]);
  unzip(path.join(out, `quiz${v}.zip`), path.join(out, `quiz${v}`));
}
