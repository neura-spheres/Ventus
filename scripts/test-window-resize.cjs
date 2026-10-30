const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');

const source = fs.readFileSync(path.join(__dirname, '../src/runtime/scripts.rs'), 'utf8');
const start = source.indexOf('  const RESIZE_ZONE =');
const end = source.indexOf('  const __neuraPostContextMenu =', start);
assert(start >= 0 && end > start, 'Content resize handlers must exist');
const fullscreenChange = source.slice(
  source.indexOf('  const fsChange = function()'),
  source.indexOf('  const fsNames =')
);

function page() {
  const handlers = {};
  const messages = [];
  const document = {
    documentElement: { clientWidth: 1920, clientHeight: 1080, style: { cursor: '' } },
    fullscreenElement: null,
    webkitFullscreenElement: null,
    addEventListener: (name, handler) => { handlers[name] = handler; },
  };
  const window = { ipc: { postMessage: () => {} } };
  const context = vm.createContext({ document, window, post: data => messages.push(data), setTimeout: () => {} });
  vm.runInContext(source.slice(start, end) + fullscreenChange, context);
  return {
    window, document, messages,
    move: (clientX = 1919, clientY = 1079) => handlers.mousemove({ clientX, clientY }),
    down: button => handlers.mousedown({ button }),
    fullscreenChange: () => vm.runInContext('fsChange()', context),
    resizeCount: () => messages.filter(message => message.cmd === 'begin_resize').length,
  };
}

test('ordinary bottom-right resizing still works and ignores right click', () => {
  const p = page();
  p.move();
  assert.equal(p.document.documentElement.style.cursor, 'nwse-resize');
  p.down(2);
  assert.equal(p.resizeCount(), 0);
  p.down(0);
  assert.equal(p.resizeCount(), 1);
  assert.equal(p.messages.find(message => message.cmd === 'begin_resize').edge, 'bottomright');
});

for (const mode of ['browser', 'document', 'webkit']) {
  test(`${mode} fullscreen blocks a cached resize edge and restores resizing on exit`, () => {
    const p = page();
    p.move();
    assert.equal(p.document.documentElement.style.cursor, 'nwse-resize');
    if (mode === 'browser') p.window.__neuraContentFullscreen = true;
    if (mode === 'document') p.document.fullscreenElement = {};
    if (mode === 'webkit') p.document.webkitFullscreenElement = {};
    p.down(0);
    assert.equal(p.resizeCount(), 0, 'Fullscreen must reject clicks before another mousemove');
    p.move();
    assert.equal(p.document.documentElement.style.cursor, '');
    p.down(0);
    assert.equal(p.resizeCount(), 0);
    assert(p.messages.some(message => message.cmd === 'content_pointer_down'));
    p.window.__neuraContentFullscreen = false;
    p.document.fullscreenElement = null;
    p.document.webkitFullscreenElement = null;
    p.move();
    p.down(0);
    assert.equal(p.resizeCount(), 1);
  });
}

test('fullscreenchange clears the resize cursor immediately', () => {
  const p = page();
  p.move();
  p.document.fullscreenElement = {};
  p.fullscreenChange();
  assert.equal(p.document.documentElement.style.cursor, '');
  assert.equal(p.messages.at(-1).cmd, 'content_fullscreen_change');
  assert.equal(p.messages.at(-1).active, true);
});

test('the edge above docked DevTools does not resize the application window', () => {
  const p = page();
  p.window.__vBottomEdge = 320;
  p.move(500, 1079);
  assert.equal(p.document.documentElement.style.cursor, '');
  p.down(0);
  assert.equal(p.resizeCount(), 0);
  p.window.__vBottomEdge = 5;
  p.move(500, 1079);
  assert.equal(p.document.documentElement.style.cursor, 's-resize');
});
