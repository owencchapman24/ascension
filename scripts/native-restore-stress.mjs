// Opt-in restore diagnostics against the actual isolated Windows installation.
// Every input/profile is synthetic and retained. Restore failures are never retried
// implicitly; the two deliberate failures require explicit fixture repair first.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile, readdir, rename, unlink, lstat } from 'node:fs/promises';
import { DatabaseSync } from 'node:sqlite';
import { isDeepStrictEqual } from 'node:util';
import path from 'node:path';
import { shiftStudy, studyTimestampFields } from './native-study.mjs';

export function historicalStressSource(original, startedAt) {
  const latestWindowEnd = Math.max(...original.study.opportunities.map(row => Date.parse(row.cue_at) + row.window_seconds * 1000));
  assert.ok(Number.isFinite(latestWindowEnd), 'Stress source must contain a valid opportunity window');
  const delta = Math.min(0, startedAt - 24 * 60 * 60 * 1000 - latestWindowEnd);
  const full = shiftStudy(original, delta);
  const restoredTimes = structuredClone(full);
  const timestampFields = new Set(studyTimestampFields);
  let timestampCount = 0;
  for (const [table, rows] of Object.entries(original.study)) if (Array.isArray(rows)) {
    for (const [index, row] of rows.entries()) for (const [field, value] of Object.entries(row)) {
      if (field.endsWith('_at') || field === 'effective_from') assert.ok(timestampFields.has(field), 'Every study timestamp must be covered by the uniform shift');
      if (timestampFields.has(field) && value !== null) {
        const shifted = full.study[table][index][field];
        assert.equal(Date.parse(shifted) - Date.parse(value), delta, 'All study timestamps must retain their relative timing');
        restoredTimes.study[table][index][field] = value;
        timestampCount += 1;
      }
    }
  }
  assert.deepEqual(restoredTimes, original, 'Historical derivation must preserve every non-timestamp field, identity, frozen allocation/hash and non-study record');
  const derivedWindowEnd = Math.max(...full.study.opportunities.map(row => Date.parse(row.cue_at) + row.window_seconds * 1000));
  assert.ok(derivedWindowEnd <= startedAt - 24 * 60 * 60 * 1000, 'Every synthetic cue window must end at least one day before stress starts');
  return { full, derivation: { kind: 'uniform-synthetic-study-time-shift', started_at: new Date(startedAt).toISOString(),
    delta_ms: delta, timestamp_count: timestampCount, timestamp_fields: studyTimestampFields,
    original_latest_window_end: new Date(latestWindowEnd).toISOString(), derived_latest_window_end: new Date(derivedWindowEnd).toISOString(),
    non_timestamp_authority_unchanged: true, relative_study_timing_unchanged: true } };
}

export async function verifyRestoreStress(h) {
  const { evaluate, clientSend, clickButton, workspace, until, root, exportFile,
    nativeDialog, ready, closeApplication, launch, checks, backupsRoot, recordSet, durableView } = h;
  const resolveOwned = name => {
    assert.ok(!path.isAbsolute(name) && !name.split(/[\\/]/).includes('..'), 'Only fixed owned relative artifact names are allowed');
    const resolved = path.resolve(root, name);
    assert.ok(resolved.toLowerCase().startsWith(path.resolve(root).toLowerCase() + path.sep), 'Stress artifacts must remain inside this isolated run');
    return resolved;
  };
  const profile = resolveOwned('profile');
  assert.equal(path.resolve(backupsRoot), resolveOwned('profile/backups'));
  const originalBytes = await readFile(resolveOwned('daily-complete.json'));
  const { full, derivation } = historicalStressSource(JSON.parse(originalBytes.toString('utf8')), Date.now());
  const sourcePath = resolveOwned('restore-stress-historical-source.json');
  const bytes = Buffer.from(JSON.stringify(full));
  await writeFile(sourcePath, bytes, { flag: 'wx' });
  assert.equal(full.schema_version, 5);
  assert.ok(full.daily.objectives.length && full.daily.checkpoints.length >= 2 && full.daily.commands.length >= 2);
  assert.ok(full.progress.acceptances.length && full.progress.grants.length && full.progress.ledger.length);
  assert.ok(full.study.manifests.length && full.study.assignments.length && full.study.outcomes.length);
  assert.ok(full.world.spending.length && full.world.commands.length && full.world.chapter.completion);
  const report = {
    format: 'ascension-native-restore-stress-v1', completed: false,
    environment: { platform: process.platform, arch: process.arch, node: process.version },
    source: { file: 'restore-stress-historical-source.json', bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex'),
      objectives: full.objectives.length, checkpoints: full.daily.checkpoints.length, grants: full.progress.grants.length,
      assignments: full.study.assignments.length, world_commands: full.world.commands.length },
    derivation: { ...derivation, original_file: 'daily-complete.json', original_bytes: originalBytes.length,
      original_sha256: createHash('sha256').update(originalBytes).digest('hex') },
    planned_fresh_cycles: 12, completed_restart_checks: 0, attempts: [], restart_checks: [], verification_failures: [],
    limitations: [
      'A bounded successful run does not prove an intermittent failure impossible.',
      'Native backup phases are observed at diagnostic boundaries; deterministic inventory/snapshot overlap is covered by separate real-SQLite tests.',
      'Synthetic engineering verification, not personal records or physical power-loss testing.',
    ],
  };
  const reportPath = resolveOwned('restore-stress-result.json');
  function totals() {
    return { scenarios_started: report.attempts.length, attempts: report.attempts.filter(a => a.confirmation_started).length,
      diagnostic_attempts: report.attempts.reduce((sum, a) => sum + (a.diagnostics?.length ?? 0), 0),
      successes: report.attempts.filter(a => a.observed === 'success').length,
      command_failures: report.attempts.filter(a => a.observed === 'error').length,
      failures: report.attempts.filter(a => a.observed === 'error' || a.verification_failed).length
        + report.verification_failures.filter(f => f.stage !== 'restore').length,
      expected_rejections: report.attempts.filter(a => a.observed === 'error' && a.expected_code === a.command_error_code && a.exact_destination).length,
      verification_failures: report.verification_failures.length,
      restart_checks_started: report.restart_checks.length, successful_restarts: report.completed_restart_checks,
      failed_restarts: report.restart_checks.filter(r => r.observed === 'failed').length,
      unexpected_failures: report.verification_failures.length
        + report.attempts.filter(a => a.observed === 'error' && !a.expected_code && !a.verification_failed).length };
  }
  const persist = () => writeFile(reportPath, JSON.stringify({ ...report, totals: totals() }, null, 2));
  const invoke = (command, input) => evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(input === undefined ? {} : { input })})`);
  const backupStatus = async () => {
    const status = await invoke('backup_status');
    return { state: status.state, snapshot_count: status.snapshot_count, latest_successful_day: status.latest_successful_day };
  };
  async function diagnostics() {
    try {
      return (await readFile(resolveOwned('profile/restore-diagnostics-v1.jsonl'), 'utf8')).trim().split('\n')
        .filter(Boolean).map(line => JSON.parse(line)).filter(item => !item.diagnostic_format);
    } catch (error) { if (error.code === 'ENOENT') return []; throw error; }
  }
  async function restoreBackups() {
    try { return (await readdir(backupsRoot)).filter(name => /^before-restore-[0-9a-f-]+\.json$/.test(name)).sort(); }
    catch (error) { if (['ENOENT', 'ENOTDIR'].includes(error.code)) return []; throw error; }
  }
  async function preserveClosedProfile(name) {
    await closeApplication();
    const destination = resolveOwned(name);
    assert.equal((await lstat(profile)).isSymbolicLink(), false, 'Profile relocation refuses a link');
    await assert.rejects(lstat(destination), { code: 'ENOENT' });
    // Only release of our already-closed process is polled. No restore is retried.
    await until(async () => { await rename(profile, destination); return true; }, 'closed stress profile released');
  }
  function verifyClosedDatabase() {
    const db = new DatabaseSync(resolveOwned('profile/ascension.sqlite3'), { readOnly: true });
    try {
      assert.equal(db.prepare('PRAGMA quick_check').get().quick_check, 'ok');
      assert.deepEqual(db.prepare('PRAGMA foreign_key_check').all(), []);
      assert.equal(db.prepare('SELECT COUNT(*) n FROM work_checkpoints').get().n, full.daily.checkpoints.length);
      assert.equal(db.prepare('SELECT COUNT(*) n FROM reward_grants').get().n, full.progress.grants.length);
      assert.equal(db.prepare('SELECT COUNT(*) n FROM world_commands').get().n, full.world.commands.length);
    } finally { db.close(); }
  }
  async function restartExact(label) {
    const entry = { label, started_at: new Date().toISOString(), phase: 'close', observed: 'pending' };
    report.restart_checks.push(entry); await persist();
    const start = performance.now();
    try {
      await closeApplication(); entry.phase = 'sqlite_integrity'; verifyClosedDatabase();
      entry.phase = 'launch'; await launch(); entry.phase = 'export';
      const reopened = await exportFile(`restore-stress-${label}-reopened.json`);
      entry.phase = 'exact_comparison';
      assert.deepEqual(reopened, full, 'Restart must retain the complete authority graph');
      entry.observed = 'success'; entry.phase = 'complete';
      entry.elapsed_ms = Number((performance.now() - start).toFixed(2));
      report.completed_restart_checks += 1; await persist();
    } catch (error) {
      entry.observed = 'failed'; entry.elapsed_ms = Number((performance.now() - start).toFixed(2));
      report.verification_failures.push({ stage: 'restart', label, phase: entry.phase });
      try { await writeFile(resolveOwned(`restore-stress-${label}-restart-failure-workspace.json`), JSON.stringify(await workspace(), null, 2)); } catch { /* Keep the original failure. */ }
      try { const shot = await clientSend('Page.captureScreenshot', { format: 'png' }); await writeFile(resolveOwned(`restore-stress-${label}-restart-failure.png`), Buffer.from(shot.data, 'base64')); } catch { /* A closed renderer must not hide the failure. */ }
      await persist(); throw error;
    }
  }
  async function restore(label, beforeRecords, { recovery = false, expectedCode = null, expectedBackup = true, cases = [] } = {}) {
    await ready();
    const before = await workspace();
    const entry = { label, cases, expected_code: expectedCode, expected_backup: expectedBackup,
      started_at: new Date().toISOString(), backup_before: await backupStatus(), observed: 'pending' };
    report.attempts.push(entry); await persist();
    const start = performance.now();
    const priorDiagnostics = (await diagnostics()).length;
    const priorBackups = await restoreBackups();
    try {
      if (recovery) await clickButton('Preview recovery');
      else { await clickButton('Import records'); nativeDialog('Preview an Ascension export', 'Select', sourcePath); }
      await until(() => evaluate("document.querySelector('[role=dialog]')?.getAttribute('aria-label')==='Review import'"), 'stress import preview');
      assert.deepEqual(durableView(await workspace()), durableView(before), 'Preview must not change the destination');
      entry.backup_at_confirmation = await backupStatus();
      entry.confirmation_started = true; await persist();
      await clickButton('Replace local records');
      const terminal = await until(() => evaluate("(()=>{const d=document.querySelector('[role=dialog]');return !d ? 'success' : d.querySelector('.modal-error') ? 'error' : false})()"), 'stress restore result');
      entry.observed = terminal;
      entry.diagnostics = (await diagnostics()).slice(priorDiagnostics);
      assert.equal(entry.diagnostics.length, 1, 'Each confirmed restore must have exactly one privacy-safe diagnostic summary');
      const diagnostic = entry.diagnostics[0];
      entry.command_error_code = diagnostic.command_error_code ?? null;
      entry.backup_after = await backupStatus();
      entry.elapsed_ms = Number((performance.now() - start).toFixed(2));
      const after = await workspace();
      entry.destination_unchanged = isDeepStrictEqual(durableView(after), durableView(before));
      await writeFile(resolveOwned(`restore-stress-${label}-destination.json`), JSON.stringify(after, null, 2));
      const addedBackups = (await restoreBackups()).filter(name => !priorBackups.includes(name));
      entry.displaced_backups_created = addedBackups.length;
      assert.equal(addedBackups.length, expectedBackup ? 1 : 0, 'Displaced-state backup count must match the known failure point');
      for (const name of addedBackups) assert.deepEqual(JSON.parse(await readFile(resolveOwned(`profile/backups/${name}`), 'utf8')), beforeRecords, 'Displaced backup must exactly preserve the prior complete records');
      assert.equal(diagnostic.displaced_backup_created, expectedBackup);
      assert.equal(entry.command_error_code, expectedCode, 'Restore command category must match this explicit scenario');
      assert.equal(terminal, expectedCode ? 'error' : 'success');
      assert.equal(diagnostic.diagnostic_version, 1);
      assert.equal(diagnostic.outcome, expectedCode ? 'error' : 'success');
      assert.equal(diagnostic.transaction_outcome, expectedCode ? 'rolled_back' : 'committed');
      assert.equal(diagnostic.rollback_sqlite_primary_code, null);
      assert.equal(diagnostic.rollback_sqlite_extended_code, null);
      assert.equal(typeof diagnostic.backup_phase_start, 'string');
      assert.equal(typeof diagnostic.backup_phase_end, 'string');
      if (expectedCode === 'storage_error') {
        assert.equal(diagnostic.phase, 'insert_projects'); assert.equal(diagnostic.operation, 'insert_projects');
        assert.equal(diagnostic.sqlite_primary_code, 19); assert.equal(diagnostic.sqlite_extended_code, 1811);
        assert.equal(diagnostic.filesystem_category, null); assert.equal(typeof diagnostic.backup_phase_failure, 'string');
      } else if (expectedCode === 'backup_failed') {
        assert.equal(diagnostic.phase, 'create_backup_directory'); assert.equal(diagnostic.operation, 'create_backup_directory');
        assert.ok(['already_exists', 'not_a_directory'].includes(diagnostic.filesystem_category), 'The deliberate directory blocker must produce a controlled filesystem category');
        assert.ok(Number.isInteger(diagnostic.os_error_code)); assert.equal(diagnostic.sqlite_primary_code, null);
        assert.equal(diagnostic.sqlite_extended_code, null); assert.equal(typeof diagnostic.backup_phase_failure, 'string');
      } else {
        assert.equal(diagnostic.phase, 'commit'); assert.equal(diagnostic.operation, 'commit');
        assert.equal(diagnostic.sqlite_primary_code, null); assert.equal(diagnostic.sqlite_extended_code, null);
        assert.equal(diagnostic.filesystem_category, null);
      }
      if (expectedCode) {
        assert.deepEqual(durableView(after), durableView(before), 'Rejected restore must preserve destination and generation');
        await clickButton('Cancel import'); await ready();
        assert.deepEqual(await exportFile(`restore-stress-${label}-unchanged.json`), beforeRecords);
      } else {
        await ready();
        assert.deepEqual(await exportFile(`restore-stress-${label}-export.json`), full, 'Every imported authority, identity, command and revision must round-trip exactly');
      }
      entry.exact_destination = true; await persist();
    } catch (error) {
      entry.verification_failed = true; entry.elapsed_ms = Number((performance.now() - start).toFixed(2));
      report.verification_failures.push({ stage: 'restore', label });
      entry.diagnostics = (await diagnostics()).slice(priorDiagnostics);
      entry.command_error_code = entry.diagnostics.at(-1)?.command_error_code ?? entry.command_error_code ?? null;
      try {
        const retained = await workspace(); entry.destination_unchanged = isDeepStrictEqual(durableView(retained), durableView(before));
        await writeFile(resolveOwned(`restore-stress-${label}-failure-workspace.json`), JSON.stringify(retained, null, 2));
      } catch { /* Original failure remains primary. */ }
      try { const shot = await clientSend('Page.captureScreenshot', { format: 'png' }); await writeFile(resolveOwned(`restore-stress-${label}-failure.png`), Buffer.from(shot.data, 'base64')); } catch { /* Preserve diagnostics if the renderer closed. */ }
      await persist(); throw error;
    }
    return entry;
  }
  async function mutateContext(label) {
    const current = await workspace(); const action = full.actions[0];
    await invoke('save_work', { expected_generation: current.generation, objective_id: action.objective_id,
      description: `Synthetic restore stress: ${label}.`, stop_condition: action.stop_condition, note: `Synthetic context for ${label}.` });
    await clientSend('Page.reload');
    await until(() => evaluate("!!document.querySelector('#work-heading')"), 'stress changed Work view');
    const changed = await exportFile(`restore-stress-${label}-before.json`);
    for (const key of ['projects', 'objectives', 'progress', 'study', 'world', 'daily', 'selected_objective_id']) assert.deepEqual(changed[key], full[key], 'Stress setup must preserve non-Work authority');
    return changed;
  }
  try {
    await persist(); await preserveClosedProfile('restore-stress-inherited-profile');
    for (let cycle = 1; cycle <= 12; cycle += 1) {
      const label = `fresh-${String(cycle).padStart(2, '0')}`;
      if (cycle > 1) await preserveClosedProfile(`restore-stress-fresh-${String(cycle - 1).padStart(2, '0')}-profile`);
      await launch(false, async () => {
        const empty = await workspace(); assert.equal(empty.objectives.length, 0); assert.equal(empty.daily.checkpoints.length, 0);
        await restore(label, recordSet(empty), { cases: ['A', 'B', 'C', 'D', 'E', 'F'] });
      });
      await restartExact(label);
    }
    checks.push('Installed restore stress: 12 fresh-profile complete restores immediately after verified first Work, with exact displaced backups, complete re-exports, SQLite integrity and 12 offline restart comparisons');

    await until(async () => (await backupStatus()).state === 'ready', 'full stress recovery snapshot ready');
    const beforeRecovery = await mutateContext('recovery');
    await restore('recovery', beforeRecovery, { recovery: true, cases: ['K'] });
    await restartExact('recovery');
    checks.push('Installed restore stress: validated recovery preview and confirmed snapshot replacement preserve the exact complete authority graph across restart');

    // J: fail actual automatic recovery and pre-restore backup with a regular file
    // at the owned backup-directory path. Repair is explicit and checked.
    await closeApplication();
    const heldBackups = resolveOwned('profile/restore-stress-held-backups');
    assert.equal((await lstat(backupsRoot)).isSymbolicLink(), false);
    await assert.rejects(lstat(heldBackups), { code: 'ENOENT' });
    await rename(backupsRoot, heldBackups);
    await writeFile(backupsRoot, 'Synthetic restore stress backup blocker', { flag: 'wx' });
    await launch();
    const beforeBackupFailure = await mutateContext('backup-failure');
    await until(async () => (await backupStatus()).state === 'warning', 'deliberate automatic backup failure');
    await restore('backup-rejected', beforeBackupFailure, { expectedCode: 'backup_failed', expectedBackup: false, cases: ['J', 'L'] });
    await closeApplication();
    assert.equal(await readFile(backupsRoot, 'utf8'), 'Synthetic restore stress backup blocker');
    await unlink(backupsRoot); await rename(heldBackups, backupsRoot); await launch();
    await invoke('retry_backup'); await until(async () => (await backupStatus()).state === 'ready', 'explicit repaired backup retry');
    await restore('backup-repaired', beforeBackupFailure, { cases: ['J', 'L'] });
    await restartExact('backup-repaired');
    checks.push('Installed restore stress: deliberate automatic/pre-restore backup failure preserves exact destination; explicit fixture repair and fresh-preview retry restore all authority and survive restart');

    // L: inject a deterministic SQLite constraint abort after the displaced backup,
    // only while the exact disposable destination process is closed.
    const beforeSqliteFailure = await mutateContext('sqlite-failure');
    await closeApplication();
    const databasePath = resolveOwned('profile/ascension.sqlite3');
    assert.equal((await lstat(databasePath)).isSymbolicLink(), false);
    const injected = new DatabaseSync(databasePath);
    try { injected.exec("CREATE TRIGGER restore_stress_abort BEFORE INSERT ON projects BEGIN SELECT RAISE(ABORT, 'synthetic restore stress'); END;"); } finally { injected.close(); }
    await launch();
    await restore('sqlite-rejected', beforeSqliteFailure, { expectedCode: 'storage_error', cases: ['L'] });
    await closeApplication();
    const repaired = new DatabaseSync(databasePath);
    try { repaired.exec('DROP TRIGGER restore_stress_abort'); } finally { repaired.close(); }
    await launch();
    await restore('sqlite-repaired', beforeSqliteFailure, { cases: ['L'] });
    await restartExact('sqlite-repaired');
    checks.push('Installed restore stress: injected SQLite abort after displaced backup rolls back exactly with diagnostic codes; explicit trigger removal and fresh-preview retry succeed without changing authority');
    assert.equal(report.attempts.length, 17); assert.equal(totals().attempts, 17); assert.equal(totals().diagnostic_attempts, 17);
    assert.equal(totals().successes, 15); assert.equal(totals().expected_rejections, 2); assert.equal(totals().unexpected_failures, 0);
    assert.equal(report.completed_restart_checks, 15);
    report.completed = true; await persist();
    return { ...totals(), restart_checks: report.completed_restart_checks, report: 'restore-stress-result.json' };
  } catch (error) {
    report.failure = 'verification_failed';
    if (!report.verification_failures.length) report.verification_failures.push({ stage: 'scenario_setup_or_final_checks' });
    await persist(); throw error;
  }
}
