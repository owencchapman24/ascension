// River Reach verification in the actual disposable installed Windows app.
// Uses real Rust commands, SQLite persistence, native file dialogs and WebView2.
// The caller verifies the isolated identifier/profile before invoking this module.
import assert from 'node:assert/strict';
import { readFile, writeFile, readdir } from 'node:fs/promises';
import { DatabaseSync } from 'node:sqlite';
import { randomUUID } from 'node:crypto';
import path from 'node:path';

const contentVersion = 'river-reach-v1';
const causeway = () => [
  { id: null, building_type: 'foundry', parcels: ['a1', 'b1'] },
  { id: null, building_type: 'foundry', parcels: ['a2', 'b2'] },
  { id: null, building_type: 'grand_archive', parcels: ['a3', 'b3'] },
];
const mesh = () => [
  { id: null, building_type: 'survey_lodge', parcels: ['a1'] },
  { id: null, building_type: 'survey_lodge', parcels: ['a2'] },
  { id: null, building_type: 'observatory', parcels: ['a3'] },
  { id: null, building_type: 'observatory', parcels: ['b1'] },
  { id: null, building_type: 'caravan_house', parcels: ['b2'] },
  { id: null, building_type: 'caravan_house', parcels: ['b3'] },
];

export async function verifyWorld(h) {
  const { evaluate, clientSend, clickButton, typeAt, workspace, until, root,
    exportFile, previewFile, nativeDialog, ready, closeApplication, launch, checks, key, resizeWindow } = h;
  const timings = [];
  let firstWorldRender = true;
  const world = () => evaluate("window.__TAURI_INTERNALS__.invoke('load_world')");
  const invoke = (command, input) => evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify({ input })}).then(value=>({ok:true,value}),error=>({ok:false,error}))`);
  async function measured(operation, fn) {
    const started = performance.now();
    const result = await fn();
    timings.push({ operation, milliseconds: Number((performance.now() - started).toFixed(2)) });
    return result;
  }
  async function change(command, fields) {
    for (let attempt = 0; attempt < 3; attempt++) {
      const response = await invoke(command, { expected_generation: (await workspace()).generation, ...fields });
      if (response.ok) return response.value;
      if (response.error?.code !== 'conflict' || attempt === 2) assert.fail(`${command}: ${JSON.stringify(response.error)}`);
    }
  }
  async function worldInput(operation) {
    const current = await world();
    return { command_id: randomUUID(), schema_version: 1, content_version: contentVersion,
      expected_world_revision: current.chapter?.revision ?? 0, chapter_id: current.chapter?.id ?? null, operation };
  }
  async function transact(operation) {
    const input = await worldInput(operation);
    const response = await measured(operation.kind, () => invoke('world_command', input));
    assert.equal(response.ok, true, `${operation.kind}: ${JSON.stringify(response.error)}`);
    return { input, response: response.value, world: await world() };
  }
  async function rejected(operation) {
    const before = await workspace();
    const response = await invoke('world_command', await worldInput(operation));
    assert.equal(response.ok, false, `Expected ${operation.kind} rejection`);
    assert.deepEqual(await workspace(), before, 'Rejected world decision preserves every durable Work/world record');
    return response.error;
  }
  async function refresh() {
    await clientSend('Page.reload');
    await until(() => evaluate("document.querySelector('#work-heading')?.textContent==='Work'"), 'Work after world reload');
  }
  async function screenshot(name) {
    const image = await clientSend('Page.captureScreenshot', { format: 'png' });
    await writeFile(path.join(root, name), Buffer.from(image.data, 'base64'));
  }
  async function openWorld() {
    const started = performance.now();
    await measured('world_open', async () => {
      await clickButton('World');
      await until(() => evaluate("document.querySelector('#river-reach-heading')?.textContent==='River Reach'"), 'voluntary River Reach view');
    });
    await until(() => evaluate("document.querySelector('.river-reach .rr-map svg')?.getBoundingClientRect().width>0"), 'River Reach province rendered');
    if (firstWorldRender) {
      timings.push({ operation: 'river_reach_first_render', milliseconds: Number((performance.now() - started).toFixed(2)) });
      firstWorldRender = false;
    }
  }
  async function keyboardButton(name) {
    await evaluate(`(() => {const e=Array.from(document.querySelectorAll('button')).find(e=>e.textContent.trim()===${JSON.stringify(name)}&&!e.matches(':disabled')&&e.getClientRects().length);if(!e)throw new Error('Missing keyboard control');e.scrollIntoView({block:'center'});e.focus();})()`);
    await key('Enter', 'Enter', 13);
  }
  async function chooseBuilding(name) {
    await measured('building_selection', async () => {
      await clickButton(`Inspect ${name}`);
      await until(() => evaluate(`document.querySelector('.rr-selected-building strong')?.textContent===${JSON.stringify(name)}`), 'selected building description');
    });
  }
  async function waitRevisionAfter(revision) {
    await until(async () => (await world()).chapter.revision > revision, 'acknowledged native world decision');
    await until(() => evaluate("!document.querySelector('[role=dialog]') && document.querySelector('.workspace-controls')?.disabled===false"), 'world command controls settled');
  }
  async function uiLayout(placements, sandbox = false) {
    await refresh(); await openWorld();
    await clickButton(sandbox ? 'Explore sandbox layouts' : 'Reconfigure');
    while (await evaluate("document.querySelectorAll('.rr-draft-list li').length")) {
      const name = await evaluate("Array.from(document.querySelectorAll('.rr-draft-list button')).find(e=>e.textContent.startsWith('Remove ')).textContent");
      await clickButton(name);
    }
    for (const placement of placements) {
      const definition = (await world()).buildings.find(row => row.building_type === placement.building_type);
      await chooseBuilding(definition.name);
      for (const site of placement.parcels) await keyboardButton(site.toUpperCase());
      await clickButton('Add to layout preview');
    }
    await until(() => evaluate("Array.from(document.querySelectorAll('button')).some(e=>e.textContent==='Save free layout'&&!e.disabled)"), 'backend validates full layout preview');
    const revision = (await world()).chapter.revision;
    await measured(sandbox ? 'sandbox_reconfiguration_ui' : 'reconfiguration_ui', async () => {
      await keyboardButton('Save free layout'); await waitRevisionAfter(revision);
    });
  }
  // Keep a concrete pre-preview workspace: previewFile compares it synchronously.
  async function restoreExact(records, name) {
    const target = path.join(root, name);
    await writeFile(target, JSON.stringify(records));
    const before = await workspace();
    await measured('import_preview', () => previewFile(target, before, records));
    await measured('import_restore', async () => {
      await clickButton('Replace local records');
      await until(() => evaluate("!document.querySelector('[role=dialog]')"), 'world restore');
    });
    return workspace();
  }
  async function earn(index) {
    const before = await workspace();
    let current = await change('create_objective', { project_id: before.projects[0].id,
      title: `Synthetic River Reach outcome ${index}`, criterion: 'One synthetic example includes its documented input and checked result.' });
    const parent = current.objectives.find(row => row.id === current.selected_objective_id);
    const criterion = current.progress.criteria.find(row => row.parent_id === parent.id);
    const basis = { parent_id: parent.id, criterion_lineage_id: criterion.criterion_lineage_id, criterion_version: criterion.criterion_version };
    await change('lock_criterion', basis);
    current = await change('record_evidence', { ...basis, kind: 'user_attestation',
      description: 'I inspected the synthetic example input and result against the prospectively locked criterion.',
      reference: null, observed_at: null, status: 'supports', assistance: 'none', uncertainty: null, supersedes: null });
    const review = await change('review_progress', { ...basis,
      evidence_refs: current.progress.evidence.filter(row => row.parent_id === parent.id).map(row => row.id),
      factual_summary: 'The synthetic example contains its required documented input and checked result.', assistance: 'none' });
    const snapshot = review.snapshot;
    const accepted = await change('accept_progress', { schema_version: 1, command_id: randomUUID(), ...basis,
      expected_parent_revision: review.workspace.objectives.find(row => row.id === parent.id).revision,
      reviewed_snapshot_id: snapshot.id, reviewed_snapshot_digest: snapshot.digest,
      evidence_refs: snapshot.evidence_refs, factual_summary: snapshot.factual_summary, assistance: 'none' });
    assert.equal(accepted.result.issued_bc, 4, 'Only genuine Stage 3b acceptance commands create synthetic credits');
    return parent.id;
  }

  // The Stage 3c suite leaves a populated study history but no game decisions.
  const populated = await exportFile('stage4-populated-stage3c.json');
  assert.equal(populated.world.chapter, null);
  assert.equal(populated.world.commands.length, 0);
  assert.equal(populated.world.spending.length, 0);
  const beforeMigration = await workspace();
  const backupRoot = path.join(root, 'profile', 'backups');
  const priorBackups = new Set((await readdir(backupRoot)).filter(name => name.endsWith('.sqlite3')));
  await closeApplication();
  const database = new DatabaseSync(path.join(root, 'profile', 'ascension.sqlite3'));
  try {
    assert.equal(database.prepare('PRAGMA user_version').get().user_version, 6);
    database.exec('PRAGMA foreign_keys=ON;BEGIN IMMEDIATE');
    for (const table of ['checkpoint_commands','work_checkpoints','objective_planning']) database.exec(`DROP TABLE ${table}`);
    for (const table of ['world_commands', 'world_spending', 'world_chapter']) {
      assert.equal(database.prepare(`SELECT COUNT(*) AS n FROM ${table}`).get().n, 0);
      database.exec(`DROP TABLE ${table}`);
    }
    database.exec('PRAGMA user_version=4;COMMIT');
  } finally { database.close(); }
  await launch();
  const migratedWorkspace = await workspace();
  assert.deepEqual({...migratedWorkspace,daily:beforeMigration.daily}, beforeMigration);
  const addedBackups = (await readdir(backupRoot)).filter(name => name.startsWith('before-migration-') && name.endsWith('.sqlite3') && !priorBackups.has(name));
  assert.equal(addedBackups.length, 1);
  const backup = new DatabaseSync(path.join(backupRoot, addedBackups[0]), { readOnly: true });
  try {
    assert.equal(backup.prepare('PRAGMA user_version').get().user_version, 4);
    assert.equal(backup.prepare('SELECT SUM(amount_bc) AS bc FROM ledger_entries').get().bc, 68);
    assert.equal(backup.prepare('SELECT COUNT(*) AS n FROM study_assignments').get().n, 4);
  } finally { backup.close(); }
  checks.push('Installed populated Stage 3c schema-v4 upgrade preserves every Work/reward/study record, creates coherent v4 backup, and starts with no world decisions');

  // Reuse the authentically accepted four-credit Stage 3b fixture. Modern restore
  // preserves authority and allows a low-balance partial-reservation exercise.
  const seed = JSON.parse(await readFile(path.join(root, 'stage3b-complete.json'), 'utf8'));
  await restoreExact(seed, 'world-four-credit-seed.json');
  const savedSelection = seed.selected_objective_id;
  const initial = await workspace();
  assert.equal(initial.progress.wallet_bc, 4);
  assert.equal((await world()).chapter, null);
  assert.ok(await evaluate("document.querySelector('.saved-action')?.textContent.includes('Write the first example step.')"));

  await openWorld();
  await screenshot('river-reach-initial-camp.png');
  await keyboardButton('Enter River Reach');
  await until(async () => !!(await world()).chapter, 'native voluntary chapter initialization');
  await until(() => evaluate("document.querySelector('.workspace-controls')?.disabled===false"), 'world entry controls settled');
  let current = await world();
  assert.equal(current.parcels.length, 6);
  assert.equal(current.buildings.length, 6);
  assert.equal(current.routes.length, 3);
  assert.equal(current.chapter.funded_site_count, 0);
  assert.deepEqual(current.bank, { issued_bc: 4, spent_bc: 0, reserved_bc: 0, available_bc: 4 });
  await until(() => evaluate("document.querySelectorAll('.rr-building-catalog article').length===6"), 'six building descriptions');
  assert.equal(await evaluate("document.querySelectorAll('.rr-routes article').length"), 3);
  await evaluate("document.querySelector('.rr-building-catalog').scrollIntoView({block:'center'})");
  await screenshot('river-reach-building-catalog.png');
  await evaluate("document.querySelector('.rr-routes').scrollIntoView({block:'center'})");
  await screenshot('river-reach-route-requirements.png');
  await evaluate("document.querySelector('.rr-province').scrollIntoView({block:'center'})");
  await screenshot('river-reach-six-sites.png');
  await chooseBuilding('Foundry');
  await keyboardButton('A1'); await keyboardButton('B2');
  await until(() => evaluate("document.querySelector('.rr-preview')?.textContent.includes('This layout cannot be committed.')"), 'invalid diagonal placement preview');
  assert.ok(await evaluate("Array.from(document.querySelectorAll('button')).find(e=>e.textContent==='Queue construction').disabled"));
  await screenshot('river-reach-invalid-placement.png');
  const preview = await measured('invalid_placement_preview', () => invoke('preview_world', {
    chapter_id: current.chapter.id, content_version: contentVersion,
    placements: [{ id: null, building_type: 'foundry', parcels: ['a1', 'b2'] }],
  }));
  assert.equal(preview.ok, true);
  assert.equal(preview.value.legal, false);
  await rejected({ kind: 'queue', building_type: 'foundry', parcels: ['a1', 'b2'], reserve_available: true });
  await clickButton('Choose Stone Causeway');
  await until(async () => (await world()).chapter.selected_route === 'stone_causeway', 'native route selection');
  await until(() => evaluate("document.querySelector('.workspace-controls')?.disabled===false"), 'route selection controls settled');
  await chooseBuilding('Survey Lodge'); await keyboardButton('A1');
  await until(() => evaluate("Array.from(document.querySelectorAll('button')).some(e=>e.textContent==='Queue construction'&&!e.disabled)"), 'valid mixed building preview');
  await measured('queue_ui', async () => {
    await keyboardButton('Queue construction');
    await until(async () => !!(await world()).chapter.queue, 'native queued mixed building');
    await until(() => evaluate("document.querySelector('.workspace-controls')?.disabled===false"), 'queued building controls settled');
  });
  assert.equal((await world()).bank.reserved_bc, 4);
  await clickButton('Complete construction');
  assert.equal(await evaluate("document.querySelector('[role=dialog]')?.getAttribute('aria-label')"), 'Complete this construction?');
  await key('Tab', 'Tab', 9, 8);
  assert.equal(await evaluate('document.activeElement.textContent'), 'Confirm construction and spend');
  await key('Tab', 'Tab', 9);
  assert.equal(await evaluate('document.activeElement.textContent'), 'Keep planning');
  await measured('construction_completion_ui', async () => {
    await keyboardButton('Confirm construction and spend');
    await until(async () => (await world()).bank.spent_bc === 4, 'native construction committed');
    await until(() => evaluate("!document.querySelector('[role=dialog]') && document.querySelector('.workspace-controls')?.disabled===false"), 'completed building controls settled');
  });
  const firstCommand = (await workspace()).world.commands.at(-1);
  const completed = { input: firstCommand.payload, response: { result: firstCommand.result }, world: await world() };
  assert.deepEqual(completed.world.bank, { issued_bc: 4, spent_bc: 4, reserved_bc: 0, available_bc: 0 });
  assert.deepEqual(completed.world.capabilities, { engineering: 2, knowledge: 1, diplomacy: 0 });
  assert.equal(completed.world.chapter.funded_site_count, 1);
  await evaluate("document.querySelector('.rr-province').scrollIntoView({block:'center'})");
  await screenshot('river-reach-first-building.png');
  const afterFirst = await workspace();
  await closeApplication(); await launch();
  assert.deepEqual(await workspace(), afterFirst);
  const exactReplay = await invoke('world_command', completed.input);
  assert.equal(exactReplay.ok, true);
  assert.deepEqual(exactReplay.value.result, completed.response.result);
  assert.deepEqual(await workspace(), afterFirst, 'Exact stale replay returns the committed result without spending again');
  const changedReplay = await invoke('world_command', { ...completed.input, operation: { kind: 'cancel_queue' } });
  assert.equal(changedReplay.ok, false);
  assert.deepEqual(await workspace(), afterFirst);
  checks.push('Installed River Reach initializes six sites/six buildings/three routes; diagonal placement rejects; authentic four-BC construction, IDs, capabilities, exact replay and changed-payload rejection survive offline restart');

  await earn(1);
  const canceledQueue = await transact({ kind: 'queue', building_type: 'foundry', parcels: ['a2', 'a3'], reserve_available: true });
  assert.deepEqual((await world()).bank, { issued_bc: 8, spent_bc: 4, reserved_bc: 4, available_bc: 0 });
  await rejected({ kind: 'complete_construction' });
  await transact({ kind: 'cancel_queue' });
  assert.deepEqual((await world()).bank, { issued_bc: 8, spent_bc: 4, reserved_bc: 0, available_bc: 4 });
  const canceledState = await workspace();
  assert.equal((await invoke('world_command', canceledQueue.input)).ok, true);
  assert.deepEqual(await workspace(), canceledState, 'Replaying a canceled blueprint does not reactivate its reservation');
  await transact({ kind: 'queue', building_type: 'foundry', parcels: ['a2', 'a3'], reserve_available: true });
  const partial = await exportFile('world-partial-reservation.json');
  await refresh(); await openWorld();
  const blueprintDetails = await evaluate("Object.fromEntries(Array.from(document.querySelectorAll('.rr-blueprint-details > div')).map(e=>[e.querySelector('dt').textContent,e.querySelector('dd').textContent]))");
  assert.equal(blueprintDetails['Globally available BC'], '0 BC');
  assert.equal(blueprintDetails['Chapter spent BC'], '4 BC');
  assert.equal(blueprintDetails['Total building capacity'], '8 BC');
  assert.equal(blueprintDetails['Current affordability'], '4 more issued BC needed');
  await evaluate("document.querySelector('.rr-queue').scrollIntoView({block:'center'})");
  await screenshot('river-reach-partial-specialist.png');
  await closeApplication(); await launch();
  assert.equal((await world()).chapter.queue.reserved_bc, 4);
  await restoreExact(seed, 'world-partial-empty.json');
  await restoreExact(partial, 'world-partial-restored.json');
  assert.deepEqual(await exportFile('world-partial-roundtrip.json'), partial);
  await earn(2);
  await transact({ kind: 'reserve' });
  assert.equal((await world()).bank.reserved_bc, 8);
  await transact({ kind: 'complete_construction' });
  current = await world();
  assert.equal(current.chapter.funded_site_count, 3);
  assert.equal(current.bank.spent_bc, 12);
  assert.deepEqual(current.chapter.layout.find(row => row.building_type === 'foundry').parcels, ['a2', 'a3']);
  checks.push('Installed eight-BC specialist keeps partial reservation without capability; cancellation restores only its hold; partial export/restore/restart is exact; additional genuine issuance completes adjacent two-site construction');

  for (let index = 3; index <= 5; index++) await earn(index);
  await transact({ kind: 'queue', building_type: 'grand_archive', parcels: ['b1', 'b2'], reserve_available: true });
  await transact({ kind: 'complete_construction' });
  await transact({ kind: 'queue', building_type: 'observatory', parcels: ['b3'], reserve_available: true });
  await transact({ kind: 'complete_construction' });
  current = await world();
  assert.equal(current.chapter.funded_site_count, 6);
  assert.deepEqual(current.bank, { issued_bc: 24, spent_bc: 24, reserved_bc: 0, available_bc: 0 });
  await rejected({ kind: 'complete_chapter' });
  const beforeLayout = structuredClone(current.chapter.layout);
  await uiLayout(causeway());
  assert.deepEqual((await world()).capabilities, { engineering: 10, knowledge: 5, diplomacy: 0 });
  const undoRevision = (await world()).chapter.revision;
  await measured('undo_ui', async () => { await keyboardButton('Undo last layout'); await waitRevisionAfter(undoRevision); });
  assert.deepEqual((await world()).chapter.layout, beforeLayout);
  assert.deepEqual((await world()).bank, current.bank);
  await uiLayout(causeway());
  await clickButton('Review chapter ending');
  assert.equal(await evaluate("document.querySelector('[role=dialog]')?.getAttribute('aria-label')"), 'Preserve this chapter ending?');
  await screenshot('river-reach-ending-review.png');
  await measured('chapter_completion_ui', async () => {
    await keyboardButton('Commit chapter ending');
    await until(async () => !!(await world()).chapter.completion, 'native Chronicle ending commit');
    await until(() => evaluate("!document.querySelector('[role=dialog]') && document.querySelector('.workspace-controls')?.disabled===false"), 'completed ending controls settled');
  });
  const ending = structuredClone((await world()).chapter.completion);
  assert.equal(ending.route, 'stone_causeway');
  assert.equal(ending.spent_bc, 24);
  assert.ok(ending.narrative.length > 30);
  await clickButton('View original Chronicle ending');
  await until(() => evaluate("document.querySelector('#chronicle-heading')?.textContent==='Chronicle'"), 'native Chronicle');
  await screenshot('river-reach-chronicle.png');
  await uiLayout(mesh(), true);
  assert.deepEqual((await world()).capabilities, { engineering: 6, knowledge: 6, diplomacy: 6 });
  assert.deepEqual((await world()).chapter.completion, ending);
  assert.deepEqual((await world()).bank, current.bank);
  await rejected({ kind: 'complete_chapter' });
  assert.equal((await workspace()).progress.grants.length, 6);
  await resizeWindow(660, 850);
  await evaluate("document.querySelector('.rr-province').scrollIntoView({block:'center'})");
  assert.ok(await evaluate('document.documentElement.scrollWidth <= window.innerWidth + 1'), 'Actual narrow native window avoids horizontal overflow');
  await screenshot('river-reach-narrow-native-window.png');
  const viewport = await evaluate('({innerWidth,innerHeight,outerWidth,outerHeight})');
  timings.push({ operation: 'narrow_native_window_dimensions', ...viewport });
  await resizeWindow(1080, 820);
  await evaluate("document.querySelector('.rr-province').scrollIntoView({block:'center'})");
  await screenshot('river-reach-sandbox.png');
  checks.push('Installed full six-site investment is capped at 24 BC; unmet route rejects; free causeway reconfiguration and undo conserve capacity/bank; explicit completion preserves one authored ending while a balanced sandbox creates no reward');

  const beforeCorrection = await workspace();
  const acceptance = beforeCorrection.progress.acceptances.find(row => row.status === 'accepted');
  await change('correct_acceptance', { acceptance_id: acceptance.id, reason: 'Synthetic factual correction after construction; retain the fictional courtesy credit.', kind: 'incorrect_acceptance' });
  assert.equal((await workspace()).progress.wallet_bc, 24);
  assert.deepEqual((await world()).bank, current.bank);
  assert.deepEqual((await world()).chapter.completion, ending);
  assert.ok((await world()).retained_credit_bc >= 4);
  await change('select_objective', { id: savedSelection });
  await change('set_construction_enabled', { enabled: true });
  await refresh();
  await clickButton('View construction');
  await until(() => evaluate("document.querySelector('.horizon-scene')?.getAttribute('data-restored-units')==='24'"), 'H1 remains cumulative issuance despite zero available balance');
  assert.equal((await world()).bank.available_bc, 0);
  await until(async () => (await workspace()).study.exposures.some(row => row.kind === 'voluntary_world' && row.issued_bc === 24), 'visible H1 delivery after world spend');
  await screenshot('river-reach-h1-retained-issuance.png');
  await clickButton('Hide construction');
  checks.push('Installed factual correction retains credit, investment and Chronicle; H1 shows 24 cumulative issued units with zero available BC; keyboard layout controls/dialog focus work; actual narrow native window has no horizontal overflow');

  const complete = await measured('complete_export', () => exportFile('world-complete.json'));
  const unsafe = { ...complete, schema_version: 3 }; delete unsafe.world; delete unsafe.daily;
  const unsafePath = path.join(root, 'world-unsafe-stage3c.json');
  await writeFile(unsafePath, JSON.stringify(unsafe));
  const beforeUnsafe = await workspace();
  await clickButton('Import records'); nativeDialog('Preview an Ascension export', 'Select', unsafePath);
  await until(() => evaluate("Array.from(document.querySelectorAll('[role=alert]')).some(e=>e.textContent.includes('River Reach history'))"), 'unsafe world erasure rejection');
  await ready(); assert.deepEqual(await workspace(), beforeUnsafe);
  await restoreExact(seed, 'world-complete-empty.json');
  await restoreExact(complete, 'world-complete-restored.json');
  assert.deepEqual(await exportFile('world-complete-roundtrip.json'), complete);
  await closeApplication(); await launch();
  assert.deepEqual((await workspace()).world, complete.world);
  assert.deepEqual((await workspace()).progress, complete.progress);
  assert.deepEqual((await world()).chapter.completion, ending);
  assert.deepEqual((await world()).bank, current.bank);
  assert.ok(await evaluate("document.querySelector('.saved-action')?.textContent.includes('Write the first example step.')"));
  checks.push('Installed complete v4 world export/replacement/re-export/restart preserves exact world/authority/command history and bank; older v3 erasure rejects with Work and province intact');

  // Each route receives an independent real chapter ending in this disposable
  // profile. Restore a full coherent synthetic snapshot between alternatives;
  // do not rewrite the first ending of an already-completed province.
  await restoreExact(populated, 'river-compact-source.json');
  await transact({ kind: 'initialize' });
  await transact({ kind: 'select_route', route: 'river_compact' });
  for (const placement of [
    { building_type: 'embassy', parcels: ['a1', 'b1'] },
    { building_type: 'embassy', parcels: ['a2', 'b2'] },
    { building_type: 'grand_archive', parcels: ['a3', 'b3'] },
  ]) {
    await transact({ kind: 'queue', ...placement, reserve_available: true });
    await transact({ kind: 'complete_construction' });
  }
  assert.deepEqual((await world()).capabilities, { engineering: 0, knowledge: 5, diplomacy: 10 });
  await transact({ kind: 'complete_chapter' });
  const compact = await exportFile('river-compact-complete.json');
  assert.equal(compact.world.chapter.completion.route, 'river_compact');
  assert.deepEqual(compact.progress, populated.progress);
  assert.deepEqual(compact.study, populated.study);
  await closeApplication(); await launch();
  assert.deepEqual((await workspace()).world, compact.world);
  await clickButton('Chronicle');
  await until(() => evaluate("document.querySelector('.rr-chronicle h3')?.textContent==='River Compact'"), 'native River Compact Chronicle');
  await screenshot('river-reach-river-compact-chronicle.png');
  await evaluate("document.querySelector('.rr-chronicle .rr-map').scrollIntoView({block:'center'})");
  await screenshot('river-reach-river-compact-fullmap-native.png');
  checks.push('Installed River Compact independently completes with two adjacent Embassies and a Grand Archive, 0/5/10 capabilities, unchanged issuance/study history, and the same original ending after restart');

  // Also exercise one complete profile containing the frozen historical study
  // graph and a finished province together. The low-wallet exercise above must
  // remain separate because the populated Stage 3c fixture legitimately has 68 BC.
  await restoreExact(populated, 'world-with-study-source.json');
  await transact({ kind: 'initialize' });
  await transact({ kind: 'select_route', route: 'beacon_mesh' });
  for (const placement of mesh()) {
    await transact({ kind: 'queue', building_type: placement.building_type, parcels: placement.parcels, reserve_available: true });
    await transact({ kind: 'complete_construction' });
  }
  await transact({ kind: 'complete_chapter' });
  const combined = await measured('combined_complete_export', () => exportFile('world-and-study-complete.json'));
  assert.equal(combined.study.assignments.length, 4);
  assert.equal(combined.progress.wallet_bc, 68);
  assert.equal(combined.world.chapter.completion.route, 'beacon_mesh');
  assert.deepEqual(combined.study, populated.study);
  assert.deepEqual(combined.progress, populated.progress);
  await restoreExact(seed, 'world-and-study-empty.json');
  await restoreExact(combined, 'world-and-study-restored.json');
  assert.deepEqual(await exportFile('world-and-study-roundtrip.json'), combined);
  await closeApplication(); await launch();
  assert.deepEqual((await workspace()).world, combined.world);
  assert.deepEqual((await world()).bank, { issued_bc: 68, spent_bc: 24, reserved_bc: 0, available_bc: 44 });
  assert.deepEqual(await exportFile('world-and-study-restarted.json'), combined);
  await clickButton('Chronicle');
  await until(() => evaluate("document.querySelector('.rr-chronicle h3')?.textContent==='Beacon Mesh'"), 'native Beacon Mesh Chronicle');
  await screenshot('river-reach-beacon-mesh-chronicle.png');
  await evaluate("document.querySelector('.rr-chronicle .rr-map').scrollIntoView({block:'center'})");
  await screenshot('river-reach-beacon-mesh-fullmap-native.png');
  checks.push('Installed combined completed province and populated Stage 3c profile round-trip and restart exactly: 17 authentic grants, 68 issued/24 invested/44 available BC, four frozen assignments, every study observation/correction/exposure and one original Beacon Mesh ending');

  await writeFile(path.join(root, 'world-timings.json'), JSON.stringify({ timings,
    profiles: ['Six authentic grants / 24 issued BC for partial reservations, construction, reconfiguration, undo and Causeway UI.', 'Seventeen authentic grants / 68 issued BC with four historical study assignments for Compact and combined Mesh portability.'],
    limitations: 'Warm observations include CDP round trips and polling. Export/import preview includes native file dialogs and automation. Native process-to-Work timings are recorded separately. No OS-cold cache or assistive-technology audit is claimed.' }, null, 2));
}
