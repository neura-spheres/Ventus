const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const rust = fs.readFileSync('src/runtime/system.rs', 'utf8');
const raw = rust.match(/fn copy_image_fetch_script[\s\S]*?r#"([\s\S]*?)"#,/)[1];

async function copy(url, failAnonymous = false) {
  const requests = [], messages = [];
  class FileReader {
    readAsDataURL() { this.result = 'data:image/jpeg;base64,AA=='; this.onload(); }
  }
  const script = raw.replaceAll('{{', '{').replaceAll('}}', '}').replace('{url}', JSON.stringify(url));
  vm.runInNewContext(script, {
    URL, FileReader, location: {href: 'https://x.com/home', origin: 'https://x.com'},
    window: {ipc: {postMessage: value => messages.push(JSON.parse(value))}},
    fetch: async (_, options) => {
      requests.push(options.credentials);
      if (failAnonymous && options.credentials === 'omit') throw Error('Authentication required');
      return {ok: true, blob: async () => ({type: 'image/jpeg'})};
    },
  });
  await new Promise(setImmediate);
  return {requests, messages};
}

test('Twitter CDN images use anonymous CORS and return image data', async () => {
  const result = await copy('https://pbs.twimg.com/media/fixture.jpg');
  assert.deepEqual(result.requests, ['omit']);
  assert.equal(result.messages.length, 1);
  assert.equal(result.messages[0].ok, true);
  assert.equal(result.messages[0].src, 'https://pbs.twimg.com/media/fixture.jpg');
});

test('same-origin images retain session credentials', async () => {
  const result = await copy('https://x.com/protected-image');
  assert.deepEqual(result.requests, ['include']);
  assert.equal(result.messages[0].ok, true);
});

test('a protected cross-origin image can retry with credentials', async () => {
  const result = await copy('https://private-cdn.test/fixture.jpg', true);
  assert.deepEqual(result.requests, ['omit', 'include']);
  assert.equal(result.messages.length, 1);
  assert.equal(result.messages[0].ok, true);
});
