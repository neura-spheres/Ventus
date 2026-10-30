const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const source = fs.readFileSync(path.join(__dirname, '../src/ui/devtools-bridge.js'), 'utf8');

function bridge(imports) {
  class NativeSocket { constructor(url) { this.url = url; this.native = true; } }
  class CloseEvent extends Event { constructor(name, options) { super(name); Object.assign(this, options); } }
  const messages = [];
  const window = new EventTarget();
  window.WebSocket = NativeSocket;
  window.ipc = { postMessage: message => messages.push(JSON.parse(message)) };
  const timers = [];
  window.testImport = name => imports[name];
  const context = vm.createContext({ window, EventTarget, Event, MessageEvent, CloseEvent, DOMException, URL, queueMicrotask,
    setTimeout: callback => timers.push(callback), console,
    location: { href: 'https://msedgedevtools.microsoft.com/frontend/inspector.html' } });
  const script = imports ? source.replace(/import\('([^']+)'\)/g, 'window.testImport(\'$1\')') : source;
  vm.runInContext(script, context);
  return { window, messages, timers };
}

test('the inspector transport forwards commands and responses without opening a network socket', async () => {
  const b = bridge();
  const socket = new b.window.WebSocket('ws://ventus-devtools/page');
  assert.equal(socket.readyState, 0);
  assert.throws(() => socket.send('{}'), /closed/);
  const received = [];
  socket.onmessage = event => received.push(JSON.parse(event.data));
  b.window.__ventusDevtoolsReceive({method: 'Runtime.executionContextCreated', params: {}});
  await Promise.resolve();
  assert.equal(socket.readyState, 1);
  socket.send('{"id":1,"method":"DOM.getDocument"}');
  assert.deepEqual(b.messages, [{type: 'protocol', message: '{"id":1,"method":"DOM.getDocument"}'}]);
  b.window.__ventusDevtoolsReceive({id: 1, result: {root: {nodeId: 1}}});
  assert.equal(received.length, 2);
  assert.equal(received[1].result.root.nodeId, 1);
  socket.close();
  assert.equal(socket.readyState, 3);
  assert.throws(() => socket.send('{}'), /closed/);
});

test('other socket destinations keep the original WebSocket implementation', () => {
  const b = bridge();
  const socket = new b.window.WebSocket('wss://example.test/socket');
  assert.equal(socket.native, true);
  assert.equal(b.messages.length, 0);
});

test('first inspection waits for frontend initialization and removes its page preview', async () => {
  let target, initialized = false;
  const calls = [];
  const b = bridge({
    './ui/legacy/legacy.js': {
      DockController: { DockController: { instance() {
        if (!initialized) throw Error('Frontend is still starting');
        return {setDockSide: side => calls.push(side)};
      } } },
      InspectorView: { InspectorView: { instance: () => ({showPanel: async panel => calls.push(panel)}) } },
    },
    './core/sdk/sdk.js': {
      TargetManager: { TargetManager: { instance: () => ({primaryPageTarget: () => target}) } },
      DOMModel: { DOMModel: {} },
    },
    './core/common/common.js': {
      Settings: { Settings: { instance: () => ({createSetting: key => ({set: value => calls.push([key, value])})}) } },
    },
  });
  b.window.dispatchEvent(new Event('DOMContentLoaded'));
  await new Promise(setImmediate);
  assert.equal(b.messages.length, 0);
  target = {model: () => ({requestDocument: async () => calls.push('document')})};
  await b.timers.shift()();
  assert.equal(b.messages.length, 0);
  assert.deepEqual(calls.splice(0), [['screencast-enabled', false]]);
  initialized = true;
  await b.timers.shift()();
  assert.deepEqual(calls, [['screencast-enabled', false], 'undocked', 'elements', 'document']);
  assert.deepEqual(b.messages, [{type: 'ready'}]);
});
