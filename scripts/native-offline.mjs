// Read-only network verification against an owned loopback server and real native IPC.
// A successful result leaves this CDP target offline for the caller's lifecycle tests.
import { randomUUID } from 'node:crypto';
import { once } from 'node:events';
import http from 'node:http';
import { setTimeout as delay } from 'node:timers/promises';

const conditions = offline => ({
  offline,
  latency: 0,
  downloadThroughput: -1,
  uploadThroughput: -1,
});

async function nativeWorkspace(client, expectedUrl) {
  const result = await client.send('Runtime.evaluate', {
    expression: `(() => {
      if (location.href !== ${JSON.stringify(expectedUrl)} || document.querySelector('#work-heading')?.textContent !== 'Work') return null;
      return window.__TAURI_INTERNALS__.invoke('load_workspace').then(value => JSON.stringify(value));
    })()`,
    awaitPromise: true,
    returnByValue: true,
  });
  if (result.exceptionDetails || typeof result.result?.value !== 'string') {
    throw new Error('The real native workspace command did not return saved records.');
  }
  return result.result.value;
}

async function until(operation, label) {
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    let timer;
    try {
      const result = await Promise.race([
        operation(),
        new Promise((_, reject) => {
          timer = setTimeout(() => reject(new Error('Probe deadline exceeded.')), Math.max(1, deadline - Date.now()));
        }),
      ]);
      if (result) return result;
    } catch { /* Navigation temporarily removes the previous execution context. */ }
    finally { clearTimeout(timer); }
    await delay(Math.max(0, Math.min(100, deadline - Date.now())));
  }
  throw new Error(`Timed out after 30 seconds: ${label}.`);
}

async function navigate(client, url) {
  const result = await client.send('Page.navigate', { url });
  if (result.errorText) throw new Error(`Navigation failed: ${result.errorText}.`);
  return result;
}

export async function verifyOffline(client) {
  const nonce = randomUUID();
  const positivePath = `/online-${nonce}`;
  const negativePath = `/offline-${nonce}`;
  const documentPath = `/document-${nonce}`;
  const expectedBody = `ascension-offline-probe-${nonce}`;
  const requests = [];
  const server = http.createServer((request, response) => {
    requests.push(request.url);
    const document = request.url === documentPath;
    response.writeHead(document || request.url === positivePath || request.url === negativePath ? 200 : 404, {
      'Content-Type': document ? 'text/html; charset=utf-8' : 'text/plain; charset=utf-8',
      'Cache-Control': 'no-store',
      'Connection': 'close',
    });
    response.end(document ? `<!doctype html><meta charset="utf-8"><title>Ascension isolated offline probe</title><p>${expectedBody}</p>` : expectedBody);
  });
  let stage = 'capture original native app and saved workspace';
  let supported = false;
  let originalUrl;
  let before;
  let navigationStarted = false;
  const evidence = { supported: false, offline: false, probe: {} };
  try {
    const { frameTree: originalFrame } = await client.send('Page.getFrameTree');
    if (originalFrame.frame.url !== 'http://tauri.localhost/') {
      throw new Error('Refusing to navigate a target other than the native app launch page.');
    }
    originalUrl = originalFrame.frame.url;
    const draftGuard = await client.send('Runtime.evaluate', {
      expression: "document.querySelector('.editor-panel, [role=dialog]') === null",
      returnByValue: true,
    });
    if (draftGuard.exceptionDetails || draftGuard.result?.value !== true) {
      throw new Error('Refusing to navigate while an editor or confirmation dialog is open.');
    }
    before = await nativeWorkspace(client, originalUrl);
    stage = 'owned loopback server startup';
    server.listen(0, '127.0.0.1');
    await once(server, 'listening');
    const { port } = server.address();
    const probeUrl = `http://127.0.0.1:${port}${documentPath}`;
    stage = 'CDP network-domain initialization';
    await client.send('Network.enable');
    await client.send('Network.emulateNetworkConditions', conditions(false));
    // Run only at launch, with no draft. This tiny owned document has no scripts or
    // external resources. Its network probe does not alter or bypass the app's CSP.
    stage = 'temporary navigation to owned loopback document';
    navigationStarted = true;
    await navigate(client, probeUrl);
    await until(async () => {
      const result = await client.send('Runtime.evaluate', {
        expression: `location.href === ${JSON.stringify(probeUrl)} && document.body?.textContent.trim() === ${JSON.stringify(expectedBody)}`,
        returnByValue: true,
      });
      return !result.exceptionDetails && result.result?.value === true;
    }, 'owned loopback document');
    evidence.probe.documentReachedServer = requests.includes(documentPath);
    if (!evidence.probe.documentReachedServer) throw new Error('The owned probe document was not requested.');
    const { frameTree } = await client.send('Page.getFrameTree');
    if (frameTree.frame.url !== probeUrl) throw new Error('The probe frame is not the owned loopback document.');
    evidence.probe.context = 'Temporary owned loopback document in the disposable app target; production CSP unchanged';
    const load = async route => {
      // DevTools' resource loader can bypass renderer network emulation. Exercise
      // the renderer's actual same-origin fetch path on this owned document.
      const result = await client.send('Runtime.evaluate', {
        expression: `(async () => {
          if (location.href !== ${JSON.stringify(probeUrl)}) throw new Error('The owned probe document is no longer active.');
          try {
            const response = await fetch(${JSON.stringify(`http://127.0.0.1:${port}${route}`)}, {
              mode: 'same-origin', cache: 'no-store', credentials: 'omit', redirect: 'error', signal: AbortSignal.timeout(5000),
            });
            return { success: response.ok, status: response.status, body: await response.text(), online: navigator.onLine };
          } catch (failure) {
            return { success: false, name: failure.name, message: failure.message, online: navigator.onLine };
          }
        })()`,
        awaitPromise: true,
        returnByValue: true,
      });
      if (result.exceptionDetails || typeof result.result?.value?.success !== 'boolean') {
        throw new Error('The renderer did not return a bounded same-origin fetch result.');
      }
      return result.result.value;
    };

    stage = 'positive loopback request';
    const positive = await load(positivePath);
    evidence.probe.positiveStatus = positive.status ?? null;
    evidence.probe.positiveNavigatorOnline = positive.online;
    evidence.probe.positiveReachedServer = requests.includes(positivePath);
    if (!positive.success || positive.status !== 200 || positive.online !== true
        || positive.body !== expectedBody || !evidence.probe.positiveReachedServer) {
      throw new Error('The positive control did not reach the owned server and return its expected body.');
    }

    stage = 'offline emulation and negative loopback request';
    await client.send('Network.emulateNetworkConditions', conditions(true));
    const negative = await load(negativePath);
    evidence.probe.negativeSucceeded = negative.success;
    evidence.probe.negativeErrorName = negative.name ?? null;
    evidence.probe.negativeErrorMessage = negative.message ?? null;
    evidence.probe.negativeNavigatorOnline = negative.online;
    evidence.probe.negativeReachedServer = requests.includes(negativePath);
    if (negative.success || negative.name !== 'TypeError' || negative.online !== false
        || evidence.probe.negativeReachedServer) {
      throw new Error('The renderer did not reject the owned loopback fetch while reporting offline.');
    }

    stage = 'return to native app and read workspace while offline';
    await navigate(client, originalUrl);
    const after = await until(() => nativeWorkspace(client, originalUrl), 'native app and workspace while offline');
    evidence.nativeWorkspaceReadable = true;
    evidence.nativeWorkspaceUnchanged = before === after;
    if (!evidence.nativeWorkspaceUnchanged) {
      throw new Error('Saved records changed during the read-only offline probe.');
    }
    supported = true;
    evidence.supported = true;
    evidence.offline = true;
    evidence.nativeAppReloadedWhileOffline = true;
    evidence.scope = 'This installed WebView2 target: owned positive/negative network controls and native app reload while offline; no Rust-process or OS-wide network denial.';
  } catch (error) {
    evidence.reason = `${stage}: ${error.message}`;
  } finally {
    if (!supported) {
      // Never leave native IPC unusable after an unsupported browser protocol mode.
      try {
        await client.send('Network.emulateNetworkConditions', conditions(false));
        evidence.offline = false;
        evidence.restoredOnlineMode = true;
      } catch {
        evidence.offline = null;
        evidence.restoredOnlineMode = false;
        evidence.reason = `${evidence.reason ?? stage} Offline-mode reset could not be confirmed.`;
      }
      if (originalUrl && before) {
        try {
          if (navigationStarted) await navigate(client, originalUrl);
          const recovered = await until(() => nativeWorkspace(client, originalUrl), 'native app recovery after unsupported offline probe');
          evidence.restoredWorkspaceUnchanged = recovered === before;
          evidence.restoredApplication = evidence.restoredWorkspaceUnchanged;
          if (!evidence.restoredWorkspaceUnchanged) {
            evidence.reason = `${evidence.reason} The restored app's saved workspace differs from the pre-probe workspace.`;
          }
        } catch {
          evidence.restoredApplication = false;
          evidence.reason = `${evidence.reason} Return to the native app and saved workspace could not be confirmed.`;
        }
      } else {
        evidence.restoredApplication = false;
      }
    }
    if (server.listening) {
      await new Promise(resolve => {
        server.close(resolve);
        server.closeAllConnections();
      });
    }
  }
  return evidence;
}
