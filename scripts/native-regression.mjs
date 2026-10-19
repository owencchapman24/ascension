// Drives the actual installed WebView2 renderer over loopback CDP, using Node 22 only.
// No mocked commands or test-only application APIs. Native file dialogs are excluded.
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { once } from 'node:events';
import { readFile, writeFile } from 'node:fs/promises';
import net from 'node:net';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const root = path.resolve(process.argv[2] ?? '');
assert.ok(root.startsWith(path.join(repository, '.artifacts') + path.sep), 'A repository-local isolated run is required.');
const config = JSON.parse((await readFile(path.join(root, 'tauri.conf.json'), 'utf8')).replace(/^\uFEFF/, ''));
assert.equal(config.app.appDirectoriesOverride, path.join(root, 'profile'));
assert.match(config.identifier, /^io\.ascension\.verification\.r[0-9a-f]{32}$/);
const executable = path.join(root, 'installed', 'ascension.exe');
const checks = [];
let application;
let client;

async function until(operation, description, timeout = 30000) {
  const end = Date.now() + timeout;
  let last;
  while (Date.now() < end) {
    try { const result = await operation(); if (result) return result; } catch (error) { last = error; }
    await delay(100);
  }
  throw new Error(`Timed out: ${description}`, { cause: last });
}

async function freePort() {
  const server = net.createServer();
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  const { port } = server.address();
  await new Promise(resolve => server.close(resolve));
  return port;
}

async function connect(url) {
  const socket = new WebSocket(url);
  await once(socket, 'open');
  let sequence = 0;
  const pending = new Map();
  socket.addEventListener('message', ({ data }) => {
    const response = JSON.parse(data);
    const request = pending.get(response.id);
    if (!request) return;
    pending.delete(response.id);
    clearTimeout(request.timer);
    if (response.error) request.reject(new Error(response.error.message));
    else request.resolve(response.result);
  });
  socket.addEventListener('close', () => {
    for (const request of pending.values()) { clearTimeout(request.timer); request.reject(new Error('CDP closed')); }
    pending.clear();
  });
  return {
    close: () => socket.close(),
    send(method, params = {}) {
      return new Promise((resolve, reject) => {
        const id = ++sequence;
        const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 10000);
        pending.set(id, { resolve, reject, timer });
        socket.send(JSON.stringify({ id, method, params }));
      });
    },
  };
}

async function evaluate(expression) {
  const response = await client.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
  if (response.exceptionDetails) throw new Error(response.exceptionDetails.text);
  return response.result.value;
}
const js = JSON.stringify;
const focused = () => evaluate('document.activeElement?.id || document.activeElement?.textContent?.trim()');
const workspace = () => evaluate("window.__TAURI_INTERNALS__.invoke('load_workspace')");
const notice = text => until(() => evaluate(`Array.from(document.querySelectorAll('[role=status]')).some(e => e.textContent.includes(${js(text)}))`), text);
async function key(key, code, windowsVirtualKeyCode, modifiers = 0) {
  await client.send('Input.dispatchKeyEvent', { type: 'keyDown', key, code, windowsVirtualKeyCode, modifiers,
    ...(key === 'Enter' ? { text: '\r', unmodifiedText: '\r' } : {}) });
  await client.send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode, modifiers });
}
const tab = () => key('Tab', 'Tab', 9);
const enter = () => key('Enter', 'Enter', 13);
async function clickButton(name) {
  const point = await evaluate(`(() => {
    const matches = Array.from(document.querySelectorAll('button')).filter(e => e.textContent.trim() === ${js(name)} && !e.matches(':disabled'));
    if (matches.length !== 1) throw new Error('Expected one enabled button');
    matches[0].scrollIntoView({block:'center'});
    const rect = matches[0].getBoundingClientRect(); return {x:rect.x + rect.width/2,y:rect.y + rect.height/2};
  })()`);
  await client.send('Input.dispatchMouseEvent', { type: 'mousePressed', button: 'left', clickCount: 1, ...point });
  await client.send('Input.dispatchMouseEvent', { type: 'mouseReleased', button: 'left', clickCount: 1, ...point });
}
async function typeAt(id, value) {
  await until(async () => await focused() === id, `focus on ${id}`);
  await key('a', 'KeyA', 65, 2);
  await client.send('Input.insertText', { text: value });
}
async function closeApplication() {
  if (!application || application.exitCode !== null || application.signalCode !== null) return;
  // WM_CLOSE targets only the child process started by this harness. No UIA/system settings.
  const result = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
    `$p = Get-Process -Id ${application.pid} -ErrorAction Stop; if (!$p.CloseMainWindow()) { throw 'No main window' }`],
  { windowsHide: true, encoding: 'utf8' });
  assert.equal(result.trim(), '');
  await until(() => application.exitCode !== null || application.signalCode !== null, 'native app exit');
  client?.close();
}
async function launch() {
  const port = await freePort();
  const environment = Object.fromEntries(Object.entries(process.env)
    .filter(([name]) => !name.toUpperCase().startsWith('WEBVIEW2_')));
  application = spawn(executable, [], {
    cwd: path.dirname(executable), windowsHide: true, stdio: 'ignore',
    env: { ...environment, WEBVIEW2_USER_DATA_FOLDER: path.join(root, 'profile', 'webview'), WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:
      `--remote-debugging-port=${port} --proxy-server=http://127.0.0.1:9 --proxy-bypass-list=tauri.localhost;ipc.localhost` },
  });
  application.on('error', error => { console.error(error.message); });
  const page = await until(async () => {
    const response = await fetch(`http://127.0.0.1:${port}/json/list`, { signal: AbortSignal.timeout(1000) });
    const pages = await response.json();
    return pages.find(item => item.type === 'page' && item.url === 'http://tauri.localhost/');
  }, 'installed app WebView2', 45000);
  client = await connect(page.webSocketDebuggerUrl);
  await until(() => evaluate("document.querySelector('#work-heading')?.textContent === 'Work'"), 'Rust-backed Work view');
  assert.equal(await evaluate("window.__TAURI_INTERNALS__.invoke('plugin:app|identifier')"), config.identifier);
  const args = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
    `$items = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = ${application.pid}"); if (!$items.Count) { throw 'WebView2 child missing' }; $text = ($items | ForEach-Object { $_.CommandLine }) -join ' '; [bool]($text.Contains('--proxy-server=http://127.0.0.1:9') -and $text.Contains('tauri.localhost;ipc.localhost'))`],
  { windowsHide: true, encoding: 'utf8' }).trim();
  assert.equal(args, 'True', 'Verify offline proxy flags on actual child process');
  checks.push('Native launch: isolated identifier, Rust workspace, and WebView2 proxy flags verified');
}

try {
  // Verify that the process-scoped proxy is unavailable without contacting an external host.
  const probe = net.connect({ host: '127.0.0.1', port: 9 });
  probe.setTimeout(2000, () => probe.destroy(new Error('Proxy probe timed out')));
  await assert.rejects(once(probe, 'connect'), { code: 'ECONNREFUSED' });
  await launch();
  assert.equal((await workspace()).projects.length, 0);
  await clickButton('Create your first project');
  await typeAt('project-title', 'Synthetic native review');
  await tab(); assert.equal(await focused(), 'Save project'); await enter();
  await notice('Project saved');
  await clickButton('New objective');
  await typeAt('objective-title', 'Prepare one synthetic example');
  await tab(); await typeAt('criterion', 'The example has three labeled steps.');
  await tab(); assert.equal(await focused(), 'Save objective'); await enter();
  await notice('Objective and criterion saved');
  await clickButton('Add next action');
  await typeAt('description', 'Write the first example step.');
  await tab(); await typeAt('stop-condition', 'One step includes its input and result.');
  await tab(); await typeAt('note', 'Resume with the synthetic materials list.');
  await tab(); await enter(); await notice('Next action and note saved');
  const created = await workspace();
  assert.deepEqual([created.projects.length, created.objectives.length, created.actions.length, created.notes.length], [1, 1, 1, 1]);
  assert.equal(created.projects[0].title, 'Synthetic native review');
  assert.equal(created.objectives[0].title, 'Prepare one synthetic example');
  assert.equal(created.objectives[0].criterion, 'The example has three labeled steps.');
  assert.equal(created.objectives[0].project_id, created.projects[0].id);
  assert.equal(created.selected_objective_id, created.objectives[0].id);
  assert.equal(created.actions[0].objective_id, created.objectives[0].id);
  assert.equal(created.actions[0].description, 'Write the first example step.');
  assert.equal(created.actions[0].stop_condition, 'One step includes its input and result.');
  assert.equal(created.notes[0].objective_id, created.objectives[0].id);
  assert.equal(created.notes[0].text, 'Resume with the synthetic materials list.');
  checks.push('Keyboard create/save: project, objective/criterion, next action/stop condition and resumption note');

  await clickButton('Edit project');
  await typeAt('project-title', 'Synthetic native review edited');
  await tab(); await enter(); await notice('Project saved');
  await clickButton('Edit objective');
  await typeAt('objective-title', 'Prepare one revised synthetic example');
  await tab(); await typeAt('criterion', 'The revised example has four labeled steps.');
  await tab(); await enter(); await notice('Objective and criterion saved');
  await clickButton('Edit next action and note');
  await typeAt('description', 'Write the revised example step.');
  await tab(); await typeAt('stop-condition', 'One revised step includes its input and result.');
  await tab(); await typeAt('note', 'Resume with the revised synthetic materials list.');
  await tab(); await enter(); await notice('Next action and note saved');
  const edited = await workspace();
  assert.equal(edited.projects[0].title, 'Synthetic native review edited');
  assert.equal(edited.objectives[0].title, 'Prepare one revised synthetic example');
  assert.equal(edited.objectives[0].criterion, 'The revised example has four labeled steps.');
  assert.equal(edited.actions[0].description, 'Write the revised example step.');
  assert.equal(edited.actions[0].stop_condition, 'One revised step includes its input and result.');
  assert.equal(edited.notes[0].text, 'Resume with the revised synthetic materials list.');
  assert.equal(edited.selected_objective_id, created.selected_objective_id);
  for (const collection of ['projects', 'objectives', 'actions', 'notes']) {
    assert.equal(edited[collection][0].id, created[collection][0].id);
    assert.equal(edited[collection][0].created_at, created[collection][0].created_at);
    assert.equal(edited[collection][0].revision, created[collection][0].revision + 1);
  }
  checks.push('Native edits preserve all IDs and creation timestamps and increment changed revisions once');

  await clickButton('Edit objective');
  await tab(); await typeAt('criterion', ' ');
  await tab(); await enter();
  await until(() => evaluate("document.querySelector('#criterion-error')?.textContent.includes('nonempty')"), 'visible criterion error');
  assert.equal(await focused(), 'criterion');
  assert.ok(await evaluate("(() => {const r=document.querySelector('#criterion-error').getBoundingClientRect(); return r.top>=0 && r.bottom<=innerHeight})()"));
  await clickButton('Cancel editing');
  await until(() => evaluate("document.querySelector('[role=dialog]')?.getAttribute('aria-label') === 'Keep your unsaved draft?'"), 'draft guard');
  assert.equal(await focused(), 'Keep editing');
  await key('Tab', 'Tab', 9, 8); assert.equal(await focused(), 'Discard draft and continue');
  await tab(); assert.equal(await focused(), 'Keep editing');
  await key('Escape', 'Escape', 27);
  await clickButton('New project');
  await clickButton('Discard draft and continue');
  await until(async () => await focused() === 'project-title', 'focus transferred to replacement editor');
  await clickButton('Cancel editing');
  assert.deepEqual(await workspace(), edited);
  checks.push('Visible validation; modal Tab/Shift+Tab containment, Escape, guarded discard, replacement-editor focus; saved records unchanged');

  const shot = await client.send('Page.captureScreenshot', { format: 'png' });
  await writeFile(path.join(root, 'work-view.png'), Buffer.from(shot.data, 'base64'));
  await closeApplication();
  await launch();
  assert.deepEqual(await workspace(), edited);
  assert.ok(await evaluate("document.querySelector('.saved-action')?.textContent.includes('Write the revised example step.')"));
  assert.ok(await evaluate("document.querySelector('.saved-note')?.textContent.includes('Resume with the revised synthetic materials list.')"));
  checks.push('Graceful native close/reopen under proxy-blocked simulation: exact workspace/selection/IDs/revisions and immediate visible action/note');
  // An abrupt stop additionally verifies recovery of an acknowledged SQLite commit.
  application.kill();
  await until(() => application.exitCode !== null || application.signalCode !== null, 'abrupt stop');
  client.close();
  await launch();
  assert.deepEqual(await workspace(), edited);
  checks.push('Abrupt process stop/reopen retains the exact last acknowledged workspace');
  await closeApplication();
  await writeFile(path.join(root, 'native-result.json'), JSON.stringify({ passed: true, checks,
    limitations: ['Native file dialogs are not driven by CDP.', 'Offline simulation blocks WebView2 via a refused proxy; no OS-wide or Rust networking denial.'] }, null, 2));
  console.log(`PASS: ${checks.length} installed WebView2 lifecycle checks`);
} catch (error) {
  if (client) {
    try {
      const shot = await client.send('Page.captureScreenshot', { format: 'png' });
      await writeFile(path.join(root, 'failure.png'), Buffer.from(shot.data, 'base64'));
      await writeFile(path.join(root, 'failure-ui.json'), JSON.stringify(await evaluate('({text:document.body.innerText, focus:document.activeElement?.outerHTML})'), null, 2));
    } catch { /* Preserve the original failure if the renderer is already gone. */ }
  }
  await writeFile(path.join(root, 'native-result.json'), JSON.stringify({ passed: false, checks, failure: error.message }, null, 2));
  throw error;
} finally {
  client?.close();
  if (application && application.exitCode === null && application.signalCode === null) application.kill();
}
