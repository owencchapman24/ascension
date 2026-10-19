// Drives the actual installed WebView2 renderer over loopback CDP, using Node 22 only.
// Native file dialogs use the bounded Windows helper; all persistence uses real commands.
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { once } from 'node:events';
import { readFile, writeFile, readdir, rename, unlink, stat } from 'node:fs/promises';
import net from 'node:net';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';
import { verifyOffline } from './native-offline.mjs';

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const root = path.resolve(process.argv[2] ?? '');
assert.ok(root.startsWith(path.join(repository, '.artifacts') + path.sep), 'A repository-local isolated run is required.');
const config = JSON.parse((await readFile(path.join(root, 'tauri.conf.json'), 'utf8')).replace(/^\uFEFF/, ''));
assert.equal(config.app.appDirectoriesOverride, path.join(root, 'profile'));
assert.match(config.identifier, /^io\.ascension\.verification\.r[0-9a-f]{32}$/);
const executable = path.join(root, 'installed', 'ascension.exe');
const checks = [];
const dialogs = [];
const offline = [];
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
function nativeDialog(title, action = 'Inspect', selectedPath = '') {
  const output = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
    '& ([scriptblock]::Create([IO.File]::ReadAllText($env:ASCENSION_NATIVE_DIALOG_SCRIPT))) -RunRoot $env:ASCENSION_NATIVE_RUN_ROOT -Repository $env:ASCENSION_NATIVE_REPOSITORY -ApplicationId $env:ASCENSION_NATIVE_PID -Title $env:ASCENSION_NATIVE_TITLE -Action $env:ASCENSION_NATIVE_ACTION -SelectedPath $env:ASCENSION_NATIVE_PATH'], {
    windowsHide: true, encoding: 'utf8', timeout: 25000,
    env: { ...process.env, ASCENSION_NATIVE_DIALOG_SCRIPT: path.join(repository, 'scripts', 'native-dialog.ps1'),
      ASCENSION_NATIVE_RUN_ROOT: root, ASCENSION_NATIVE_REPOSITORY: repository, ASCENSION_NATIVE_PID: String(application.pid), ASCENSION_NATIVE_TITLE: title,
      ASCENSION_NATIVE_ACTION: action, ASCENSION_NATIVE_PATH: selectedPath },
  });
  if (action !== 'Inspect') dialogs.push(JSON.parse(output));
  return output;
}
const recordSet = value => ({ format: 'ascension-records', schema_version: 1,
  selected_objective_id: value.selected_objective_id,
  ...Object.fromEntries(['projects', 'objectives', 'actions', 'notes'].map(key => [key, [...value[key]].sort((a, b) => a.id.localeCompare(b.id))])) });
const backupsRoot = path.join(root, 'profile', 'backups');
async function backups() {
  try { return (await readdir(backupsRoot)).filter(name => name.endsWith('.json')).sort(); }
  catch (error) { if (error.code === 'ENOENT') return []; throw error; }
}
const ready = () => until(() => evaluate("Array.from(document.querySelectorAll('button')).some(e => e.textContent === 'Import records' && !e.matches(':disabled'))"), 'file command completed');
async function exportFile(name) {
  const target = path.join(root, name);
  await clickButton('Export records');
  nativeDialog('Export saved Ascension records', 'Select', target);
  await notice(`Saved records exported to ${name}.`);
  return JSON.parse(await readFile(target, 'utf8'));
}
async function previewFile(target, current, incoming) {
  await clickButton('Import records');
  nativeDialog('Preview an Ascension export', 'Select', target);
  await until(() => evaluate("document.querySelector('[role=dialog]')?.getAttribute('aria-label') === 'Review import'"), 'native import preview');
  assert.deepEqual(await workspace(), current, 'Preview cannot change saved state or generation');
  const rows = await evaluate("Array.from(document.querySelectorAll('[role=dialog] tbody tr')).map(row => Array.from(row.querySelectorAll('td')).map(cell => Number(cell.textContent)))");
  assert.deepEqual(rows, ['projects', 'objectives', 'actions', 'notes'].map(key => [current[key].length, incoming[key].length]));
  assert.ok(await evaluate("document.querySelector('[role=dialog]').textContent.includes('Records missing from the file will be removed.')"));
  assert.equal(await focused(), 'Cancel import');
}
async function restoreFile(target, current, incoming) {
  await previewFile(target, current, incoming);
  await clickButton('Replace local records');
  await until(() => evaluate("!document.querySelector('[role=dialog]')"), 'restore completion');
  assert.deepEqual(recordSet(await workspace()), incoming);
}
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
  const actualDataPath = await evaluate("window.__TAURI_INTERNALS__.invoke('plugin:path|resolve_directory', {directory:14})");
  assert.equal(path.resolve(actualDataPath).toLowerCase(), path.join(root, 'profile').toLowerCase(), 'Actual native app-data path must be isolated');
  const args = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
    `$items = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = ${application.pid}"); if (!$items.Count) { throw 'WebView2 child missing' }; $text = ($items | ForEach-Object { $_.CommandLine }) -join ' '; [bool]($text.Contains('--proxy-server=http://127.0.0.1:9') -and $text.Contains('tauri.localhost;ipc.localhost'))`],
  { windowsHide: true, encoding: 'utf8' }).trim();
  assert.equal(args, 'True', 'Verify offline proxy flags on actual child process');
  const evidence = await verifyOffline(client);
  offline.push(evidence);
  assert.ok(evidence.supported || (evidence.restoredOnlineMode && evidence.restoredApplication && evidence.restoredWorkspaceUnchanged), 'Unsupported offline emulation must restore the original application and workspace safely');
  checks.push('Native launch: actual app-data directory, isolated identifier, Rust workspace, and WebView2 proxy flags verified');
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

  const original = await exportFile('original.json');
  assert.deepEqual(original, recordSet(created));
  checks.push('Actual Windows Save dialog exports exact versioned data, relationships, IDs, revisions and selection');
  const filesBeforeCancel = (await readdir(root)).sort();
  for (const [button, title] of [['Export records', 'Export saved Ascension records'], ['Import records', 'Preview an Ascension export']]) {
    await clickButton(button);
    nativeDialog(title, 'Cancel');
    await ready();
    assert.deepEqual(await workspace(), created);
    assert.equal(await evaluate("document.querySelector('[role=dialog]')"), null);
    await tab();
    assert.ok(await evaluate("document.activeElement?.matches('button:not(:disabled), select:not(:disabled), input:not(:disabled), textarea:not(:disabled)')"), 'Tab remains usable after native dialog cancellation');
  }
  assert.deepEqual((await readdir(root)).sort(), filesBeforeCancel);
  assert.deepEqual(await backups(), []);
  checks.push('Cancel actual Windows Save/Open dialogs: no files, backups or workspace/generation changes');

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

  // A second local project makes the replacement/removal counts observable.
  await clickButton('New project');
  await typeAt('project-title', 'Synthetic project removed by restore');
  await tab(); await enter(); await notice('Project saved');
  const modified = await workspace();
  const modifiedRecords = recordSet(modified);
  const originalPath = path.join(root, 'original.json');
  await previewFile(originalPath, modified, original);
  await clickButton('Cancel import');
  assert.deepEqual(await workspace(), modified);
  assert.deepEqual(await backups(), []);
  checks.push('Actual Open dialog preview reports 2/1/1/1 local versus 1/1/1/1 incoming; cancel preview preserves state');

  // Simulate interruption before confirmation, without guessing a SQLite commit boundary.
  await previewFile(originalPath, modified, original);
  application.kill();
  await until(() => application.exitCode !== null || application.signalCode !== null, 'interrupted preview exit');
  client.close();
  await launch();
  assert.deepEqual(await workspace(), modified);
  assert.equal(await evaluate("document.querySelector('[role=dialog]')"), null);
  assert.deepEqual(await backups(), []);
  checks.push('Interrupted native import preview: restart retains exact pre-import state and permits a fresh preview');

  await restoreFile(originalPath, modified, original);
  const restored = await workspace();
  assert.notEqual(restored.generation, modified.generation);
  const recoveryFiles = await backups();
  assert.equal(recoveryFiles.length, 1);
  const recoveryPath = path.join(backupsRoot, recoveryFiles[0]);
  assert.deepEqual(JSON.parse(await readFile(recoveryPath, 'utf8')), modifiedRecords);
  assert.deepEqual(await exportFile('restored.json'), original);
  checks.push('Confirmed native restore preserves exact records/IDs/relationships/revisions; backup equals displaced records; native re-export matches');
  await restoreFile(originalPath, restored, original);
  assert.deepEqual(await workspace(), restored);
  assert.deepEqual(await backups(), recoveryFiles);
  await notice('already matches');
  checks.push('Identical native import replay is a no-op: unchanged generation, records and backup count');

  const invalid = [
    ['malformed.json', '{"format":', 'valid'],
    ['unsupported.json', JSON.stringify({ ...original, schema_version: 99 }), 'not supported'],
    ['conflicting-ids.json', JSON.stringify({ ...original, notes: [{ ...original.notes[0], id: original.actions[0].id }] }), 'unique'],
    ['dangling-reference.json', JSON.stringify({ ...original, objectives: [{ ...original.objectives[0], project_id: original.actions[0].id }] }), 'missing project'],
    ['oversized.json', ' '.repeat(10 * 1024 * 1024 + 1), '10 MiB'],
  ];
  for (const [name, contents, expectedError] of invalid) {
    const target = path.join(root, name);
    await writeFile(target, contents, 'utf8');
    await clickButton('Import records');
    nativeDialog('Preview an Ascension export', 'Select', target);
    await until(() => evaluate(`Array.from(document.querySelectorAll('[role=alert]')).some(e => e.textContent.includes(${js(expectedError)}))`), `reject ${name}`);
    await ready();
    assert.deepEqual(await workspace(), restored);
    assert.deepEqual(await backups(), recoveryFiles);
    assert.equal(await evaluate("document.querySelector('[role=dialog]')"), null);
  }
  checks.push('Actual Open dialog rejects malformed, future-version, duplicate-ID, dangling-reference and oversized files without mutations/backups');

  const protectedExport = path.join(root, 'profile', 'forbidden.json');
  await clickButton('Export records');
  nativeDialog('Export saved Ascension records', 'Select', protectedExport);
  await until(() => evaluate("document.querySelector('[role=alert]')?.textContent.includes('outside Ascension')"), 'protected export rejection');
  await assert.rejects(stat(protectedExport), { code: 'ENOENT' });
  assert.deepEqual(await workspace(), restored);
  checks.push('Actual Save dialog rejects an application-data destination without creating a file or changing records');

  // Restore a real recovery backup through the same native Open/preview/replace path.
  await restoreFile(recoveryPath, restored, modifiedRecords);
  const recovered = await workspace();
  assert.equal((await backups()).length, 2);
  checks.push('A real pre-import recovery backup restores the displaced workspace through the native Open dialog');

  // A disposable file blocks backup-directory creation; valid records must survive.
  const heldBackups = path.join(root, 'profile', 'backups-held');
  await rename(backupsRoot, heldBackups);
  let blockerCreated = false;
  try {
    await writeFile(backupsRoot, 'Synthetic backup failure fixture', { flag: 'wx' });
    blockerCreated = true;
    await previewFile(originalPath, recovered, original);
    await clickButton('Replace local records');
    await until(() => evaluate("document.querySelector('[role=alert]')?.textContent.includes('recovery backup could not be saved')"), 'failed backup rejection');
    assert.deepEqual(await workspace(), recovered);
    await clickButton('Cancel import');
  } finally {
    try { if (blockerCreated) await unlink(backupsRoot); }
    finally { await rename(heldBackups, backupsRoot); }
  }
  assert.equal((await backups()).length, 2);
  await restoreFile(originalPath, recovered, original);
  assert.equal((await backups()).length, 3);
  checks.push('Failed import backup preserves exact local state; repairing only the disposable fixture allows fresh native preview/restore');

  const empty = { ...original, selected_objective_id: null, projects: [], objectives: [], actions: [], notes: [] };
  const emptyPath = path.join(root, 'empty.json');
  await writeFile(emptyPath, JSON.stringify(empty), 'utf8');
  await restoreFile(emptyPath, await workspace(), empty);
  assert.equal((await backups()).length, 4);
  await restoreFile(originalPath, await workspace(), original);
  assert.equal((await backups()).length, 5, 'Every changed restore preserves the previous record set, including an empty one');
  checks.push('Native import into an empty workspace preserves original stable IDs/revisions and selection');
  const finalWorkspace = await workspace();

  const shot = await client.send('Page.captureScreenshot', { format: 'png' });
  await writeFile(path.join(root, 'work-view.png'), Buffer.from(shot.data, 'base64'));
  await closeApplication();
  await launch();
  assert.deepEqual(await workspace(), finalWorkspace);
  assert.ok(await evaluate("document.querySelector('.saved-action')?.textContent.includes('Write the first example step.')"));
  assert.ok(await evaluate("document.querySelector('.saved-note')?.textContent.includes('Resume with the synthetic materials list.')"));
  checks.push('Graceful native close/reopen after restore: exact workspace/selection/IDs/revisions and immediate visible original action/note');
  // An abrupt stop additionally verifies recovery of an acknowledged SQLite commit.
  application.kill();
  await until(() => application.exitCode !== null || application.signalCode !== null, 'abrupt stop');
  client.close();
  await launch();
  assert.deepEqual(await workspace(), finalWorkspace);
  checks.push('Abrupt process stop/reopen retains the exact last acknowledged workspace');
  await closeApplication();
  await writeFile(path.join(root, 'native-result.json'), JSON.stringify({ passed: true, checks, dialogs, offline,
    limitations: ['Offline denial is scoped to WebView2; Rust/OS-wide networking is not blocked.', 'Interruption tested before import confirmation and after acknowledged writes, not during a physical power failure.'] }, null, 2));
  console.log(`PASS: ${checks.length} installed WebView2 lifecycle checks`);
} catch (error) {
  if (client) {
    try {
      const shot = await client.send('Page.captureScreenshot', { format: 'png' });
      await writeFile(path.join(root, 'failure.png'), Buffer.from(shot.data, 'base64'));
      await writeFile(path.join(root, 'failure-ui.json'), JSON.stringify(await evaluate('({text:document.body.innerText, focus:document.activeElement?.outerHTML})'), null, 2));
    } catch { /* Preserve the original failure if the renderer is already gone. */ }
  }
  await writeFile(path.join(root, 'native-result.json'), JSON.stringify({ passed: false, checks, dialogs, offline, failure: error.message }, null, 2));
  throw error;
} finally {
  client?.close();
  if (application && application.exitCode === null && application.signalCode === null) application.kill();
}
