const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const source = fs.readFileSync('src/ui/ubol-ready.js', 'utf8');

function probe({config, scripts, rulesets = ['ublock-filters'], fail = false, none = [], sites = []}) {
  const messages = [], timers = [], modes = [];
  let start;
  const window = {
    addEventListener: (_, handler) => { start = handler; },
    ipc: {postMessage: message => messages.push(JSON.parse(message))},
  };
  const chrome = {
    runtime: {sendMessage: async message => {
      if (fail) throw Error('Worker starting');
      if (message.what === 'getFilteringModeDetails') return {none};
      if (message.what === 'setFilteringMode') return modes.push([message.hostname, message.level]);
      return await config;
    }},
    scripting: {getRegisteredContentScripts: async () => scripts},
    declarativeNetRequest: {getEnabledRulesets: async () => rulesets},
  };
  vm.runInNewContext(source.replace('__UBOL_SITES__', JSON.stringify(sites)), {window, chrome, setTimeout: callback => timers.push(callback)});
  return {start, messages, timers, modes};
}

const config = {hasOmnipotence: true, defaultFilteringMode: 2};
const scripts = [{id: 'scriptlets-main', runAt: 'document_start'}];

test('the first page waits for the worker startup promise', async () => {
  let resolve;
  const pending = new Promise(done => { resolve = done; });
  const p = probe({config: pending, scripts});
  const done = p.start();
  await Promise.resolve();
  assert.deepEqual(p.messages, []);
  resolve(config);
  await done;
  assert.deepEqual(p.messages.map(message => message.ready), [true]);
});

test('an enabled extension without registered scriptlets is not ready', async () => {
  for (const registered of [[], [{id: 'ublock-filters.main', runAt: 'document_start'}]]) {
    const p = probe({config, scripts: registered});
    await p.start();
    assert.deepEqual(p.messages.map(message => message.ready), [false]);
  }
});

test('basic mode and missing network rules do not report full protection', async () => {
  for (const options of [{config: {...config, defaultFilteringMode: 1}, scripts}, {config, scripts, rulesets: []}]) {
    const p = probe(options);
    await p.start();
    assert.deepEqual(p.messages.map(message => message.ready), [false]);
  }
});

test('excepted sites are switched off and removed exceptions switched back on', async () => {
  const p = probe({config, scripts, none: ['old.test'], sites: ['new.test']});
  await p.start();
  assert.deepEqual(p.modes, [['old.test', 2], ['new.test', 0]]);
  assert.deepEqual(p.messages.map(message => message.ready), [true]);
});

test('a worker that is still starting is retried without releasing navigation', async () => {
  const p = probe({config, scripts, fail: true});
  await p.start();
  assert.deepEqual(p.messages, []);
  assert.equal(p.timers.length, 1);
});
