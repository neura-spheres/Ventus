const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const html = fs.readFileSync(path.join(__dirname, '../src/ui/chrome.html'), 'utf8');

function menu() {
  let open = true;
  const messages = [];
  const element = { classList: {
    contains: () => open,
    remove: () => { open = false; },
  } };
  const context = vm.createContext({
    __ctxData: { tabId: 'clicked-tab' },
    document: { getElementById: () => element },
    clearChromeOverlay: () => {},
    send: (command, data) => messages.push({ command, data }),
  });
  const actions = html.slice(html.indexOf('function ctxAction('), html.indexOf('function showBrowserContextMenu('));
  const close = html.slice(html.indexOf('function closeContextMenu('), html.indexOf('function isSelectableAiTarget('));
  vm.runInContext(actions + close, context);
  return { messages, run: code => vm.runInContext(code, context), isOpen: () => open };
}

test('Inspect dispatches the captured tab after closing the menu', () => {
  const m = menu();
  m.run('ctxAction("inspect")');
  assert.equal(m.isOpen(), false);
  assert.deepEqual(m.messages.map(message => message.command), ['InspectElement']);
  assert.equal(m.messages[0].data.tab_id, 'clicked-tab');
});

test('normal context actions dismiss the native menu before acting', () => {
  const m = menu();
  m.run('ctxAction("devtools")');
  assert.equal(m.isOpen(), false);
  assert.deepEqual(m.messages.map(message => message.command), ['OpenDevtools']);
});

test('dismissal closes the menu without sending page commands', () => {
  const m = menu();
  m.run('closeContextMenu(); closeContextMenu();');
  assert.deepEqual(m.messages.map(message => message.command), []);
});

for (const [key, shiftKey, expected] of [['F12', false, 'OpenDevtools'], ['I', true, 'OpenDevtools'], ['C', true, 'InspectPicker']]) {
  test(`${key} developer shortcut is handled by chrome`, () => {
    const messages = [];
    let handler;
    let prevented = false;
    const context = vm.createContext({
      document: { addEventListener: (_, fn) => { handler = fn; } },
      send: command => messages.push(command),
    });
    const section = html.indexOf('// KEYBOARD SHORTCUTS');
    const start = html.indexOf("document.addEventListener('keydown'", section);
    const end = html.indexOf("window.addEventListener('resize'", start);
    assert(section >= 0 && start >= 0 && end > start);
    vm.runInContext(html.slice(start, end), context);
    handler({ key, ctrlKey: shiftKey, shiftKey, preventDefault: () => { prevented = true; } });
    assert(prevented);
    assert.deepEqual(messages, [expected]);
  });
}
