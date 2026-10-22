// Stage 4.5 in the actual isolated Windows installation. No test-only app commands.
import assert from 'node:assert/strict';
import { readFile, writeFile, readdir, rename, mkdir, unlink } from 'node:fs/promises';
import { DatabaseSync } from 'node:sqlite';
import path from 'node:path';

export async function verifyDaily(h) {
  const { evaluate, clientSend, clickButton, typeAt, workspace, until, notice, root,
    exportFile, nativeDialog, ready, closeApplication, launch, checks, key, resizeWindow, backupsRoot } = h;
  const timings = [];
  const invoke = (command, input) => evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(input === undefined ? {} : {input})})`);
  async function timed(operation, task) {
    const start = performance.now(); const result = await task();
    timings.push({operation,milliseconds:Number((performance.now()-start).toFixed(2))}); return result;
  }
  async function change(command, input) { return invoke(command,{expected_generation:(await workspace()).generation,...input}); }
  async function refresh() { await clientSend('Page.reload'); await until(()=>evaluate("document.querySelector('#work-heading')?.textContent==='Work'"),'Work reload'); }
  async function summary(prefix) {
    await evaluate(`(()=>{const e=Array.from(document.querySelectorAll('summary')).find(e=>e.textContent.startsWith(${JSON.stringify(prefix)}));if(!e)throw new Error('Summary missing');e.closest('details').open=true;e.scrollIntoView({block:'center'});})()`);
    await until(()=>evaluate(`(()=>{const e=Array.from(document.querySelectorAll('summary')).find(e=>e.textContent.startsWith(${JSON.stringify(prefix)}));const d=e?.closest('details');return !!d?.open&&Array.from(d.children).slice(1).some(c=>c.getClientRects().length>0);})()`),'expanded daily controls');
  }
  async function input(id,text) { await until(()=>evaluate(`(()=>{const e=document.getElementById(${JSON.stringify(id)});return !!e&&!e.matches(':disabled')&&e.getClientRects().length>0;})()`),'visible daily input');await evaluate(`document.getElementById(${JSON.stringify(id)}).focus()`); await typeAt(id,text); }
  async function keyboard(name) {
    await until(()=>evaluate(`Array.from(document.querySelectorAll('button')).some(e=>e.textContent.trim()===${JSON.stringify(name)}&&!e.matches(':disabled')&&e.getClientRects().length)`),'enabled daily keyboard control');
    await evaluate(`(()=>{const e=Array.from(document.querySelectorAll('button')).find(e=>e.textContent.trim()===${JSON.stringify(name)}&&!e.matches(':disabled')&&e.getClientRects().length);if(!e)throw new Error('Keyboard control missing');e.scrollIntoView({block:'center'});e.focus();})()`);
    await key('Enter','Enter',13);
  }
  async function shot(name) { const result=await clientSend('Page.captureScreenshot',{format:'png'});await writeFile(path.join(root,name),Buffer.from(result.data,'base64')); }
  function authority(value) { return {progress:value.progress,world:value.world}; }
  async function exactExport(expected,name) { assert.deepEqual(await exportFile(name),expected); }
  async function importFile(file) {
    await clickButton('Import records');nativeDialog('Preview an Ascension export','Select',file);
    await until(()=>evaluate("document.querySelector('[role=dialog]')?.getAttribute('aria-label')==='Review import'"),'daily import preview');
    await clickButton('Replace local records');await until(()=>evaluate("!document.querySelector('[role=dialog]')"),'daily import confirmed');await ready();
  }
  const profile = path.join(root,'profile');
  const stage4 = await exportFile('daily-stage4-source.json');
  assert.equal(stage4.daily.checkpoints.length,0);
  const old = await workspace();
  await closeApplication();
  const db = new DatabaseSync(path.join(profile,'ascension.sqlite3'));
  try {
    assert.equal(db.prepare('PRAGMA user_version').get().user_version,6);
    db.exec('PRAGMA foreign_keys=ON;BEGIN IMMEDIATE');
    for(const table of ['checkpoint_commands','work_checkpoints']) {
      assert.equal(db.prepare(`SELECT COUNT(*) n FROM ${table}`).get().n,0);db.exec(`DROP TABLE ${table}`);
    }
    db.exec('DROP TABLE objective_planning;PRAGMA user_version=5;COMMIT');
  } finally {db.close();}
  await launch();
  const migrated = await workspace();
  for(const field of ['projects','objectives','actions','notes','progress','study','world','selected_objective_id','generation']) assert.deepEqual(migrated[field],old[field]);
  assert.ok(migrated.daily.objectives.every(o=>o.status==='active'&&o.revision===1));
  assert.deepEqual([...migrated.daily.objectives].sort((a,b)=>a.priority-b.priority).map(o=>o.objective_id),migrated.objectives.map(o=>o.id));
  const migrationNames=(await readdir(backupsRoot)).filter(n=>n.startsWith('before-migration-')&&n.endsWith('.sqlite3'));
  let v5Backups=0;
  for(const name of migrationNames){const snapshot=new DatabaseSync(path.join(backupsRoot,name),{readOnly:true});try{if(snapshot.prepare('PRAGMA user_version').get().user_version===5){v5Backups++;assert.equal(snapshot.prepare('PRAGMA quick_check').get().quick_check,'ok');assert.equal(snapshot.prepare('SELECT COUNT(*) n FROM world_commands').get().n,old.world.commands.length);}}finally{snapshot.close();}}
  assert.equal(v5Backups,1);
  checks.push('Stage 4.5 installed populated schema-v5 upgrade preserves all Work/reward/frozen study/world identities, selected focus, and coherent v5 recovery; legacy priority defaults follow prior list order');

  const ordered=[...migrated.daily.objectives].sort((a,b)=>a.priority-b.priority).map(p=>migrated.objectives.find(o=>o.id===p.objective_id));
  assert.ok(ordered.length>=3);
  const [first,second,third]=ordered;
  assert.equal(migrated.daily.objectives.find(p=>p.objective_id===first.id).priority,0);
  assert.equal(migrated.daily.objectives.find(p=>p.objective_id===second.id).priority,1);
  // The historical acceptance fixtures intentionally had only one saved action.
  // Supply ordinary action context on the same accepted identities, never new
  // reward parents, so lifecycle/support checks cover three meaningful choices.
  for(const objective of [first,second,third]) if(!migrated.actions.some(a=>a.objective_id===objective.id)) {
    await change('save_work',{objective_id:objective.id,description:'Inspect one synthetic example detail.',stop_condition:'One detail has a written observation.',note:'Continue at the next synthetic detail.'});
  }
  await change('select_objective',{id:first.id});await refresh();
  await summary('Manage objectives and priority');
  await timed('objective_reorder_ui',async()=>{
    await evaluate(`Array.from(document.querySelectorAll('button')).find(e=>e.getAttribute('aria-label')===${JSON.stringify(`Move ${first.title} lower`)}).focus()`);
    await key('Enter','Enter',13);
    await until(async()=>(await workspace()).daily.objectives.find(o=>o.objective_id===first.id).priority===1,'durable reorder');
  });
  await clickButton(`Pause ${first.title}`);await notice('Objective paused');
  assert.equal((await workspace()).selected_objective_id,second.id);
  await clickButton(`Archive ${third.title}`);await notice('Objective archived');
  await clickButton(`Restore ${third.title}`);await notice('Objective restored');
  await clickButton(`Reactivate ${first.title}`);await notice('Objective restored');
  await clickButton(`Focus ${first.title}`);await until(async()=>(await workspace()).selected_objective_id===first.id,'explicit focus');
  assert.deepEqual(authority(await workspace()),authority(migrated));
  checks.push('Installed keyboard reorder, focus, pause with active fallback, archive/restore and reactivation preserve every acceptance, grant, reward slot, world spend and Chronicle');

  await keyboard('Save where I stopped');
  await input('checkpoint-note','Prepared the first synthetic daily-use example.');
  await input('checkpoint-description','Check the second synthetic example step.');
  await input('checkpoint-stop_condition','The second step has a stated input and result.');
  await input('checkpoint-resumption_note','Resume with the second step in the synthetic example.');
  await timed('checkpoint_save_ui',async()=>{await clickButton('Save checkpoint');await notice('Checkpoint and next action saved');});
  let current=await workspace();const checkpoint=current.daily.checkpoints[0];
  assert.equal(checkpoint.note,'Prepared the first synthetic daily-use example.');
  assert.equal(checkpoint.provenance,'self_report');
  assert.equal(current.actions.find(a=>a.objective_id===first.id).description,checkpoint.description);
  assert.deepEqual(authority(current),authority(migrated));
  const replay=current.daily.commands.find(c=>c.checkpoint_id===checkpoint.id).input;
  await closeApplication();await launch();assert.deepEqual(await workspace(),current);
  const replayed=await invoke('record_checkpoint',replay);assert.equal(replayed.checkpoint.id,checkpoint.id);assert.deepEqual(replayed.workspace,current);
  const changedPayload=await evaluate(`window.__TAURI_INTERNALS__.invoke('record_checkpoint',{input:${JSON.stringify({...replay,note:'Changed payload'})}}).then(()=>null,e=>e.code)`);
  assert.equal(changedPayload,'idempotency_conflict');assert.deepEqual(await workspace(),current);
  await summary('Checkpoint history');await clickButton('Correct checkpoint');
  await input('checkpoint-note','Prepared only the first part of the synthetic example.');
  await input('checkpoint-correction_reason','The earlier account included a step that was still unfinished.');
  await clickButton('Save checkpoint correction');await notice('Checkpoint correction saved');
  current=await workspace();assert.equal(current.daily.checkpoints.length,2);assert.deepEqual(current.daily.checkpoints[0],checkpoint);assert.equal(current.daily.checkpoints[1].supersedes,checkpoint.id);
  checks.push('Installed compact checkpoint atomically updates action/note, survives offline restart, replays original command after lost response without duplication, rejects altered payload, and appends a correction preserving the original');

  // An external authoritative focus change must not remount and lose this draft.
  await keyboard('Save where I stopped');
  await input('checkpoint-note','Keep this synthetic draft through a concurrent focus change.');
  await change('set_objective_status',{objective_id:first.id,status:'paused'});
  await clickButton('Save checkpoint');
  await until(()=>evaluate("Array.from(document.querySelectorAll('button')).some(e=>e.textContent==='Check saved history and keep draft')"),'stale checkpoint retains a refresh path');
  await clickButton('Check saved history and keep draft');
  await until(()=>evaluate("document.body.textContent.includes('Your unsaved work still belongs to')"),'draft kept with original objective context');
  assert.equal(await evaluate("document.getElementById('checkpoint-note')?.value"),'Keep this synthetic draft through a concurrent focus change.');
  assert.equal((await workspace()).selected_objective_id,second.id);
  assert.equal((await workspace()).daily.checkpoints.length,2);
  await clickButton('Discard checkpoint draft');
  await change('set_objective_status',{objective_id:first.id,status:'active'});
  await change('select_objective',{id:first.id});await refresh();
  checks.push('Installed stale checkpoint save and history refresh preserve the original-objective draft through an external pause/focus change, without saving or duplicating its text');

  const beforeSupport=await workspace();
  await summary('Need a different way to start?');await keyboard('Make the entry smaller');
  await input('small-action','Write one synthetic input label.');await input('small-stop','One input is labeled.');
  await keyboard('Use this smaller entry');assert.deepEqual(await workspace(),beforeSupport);
  await keyboard('Rest or step away');assert.deepEqual(await workspace(),beforeSupport);
  await keyboard('Choose another priority');
  // Alternate support and manager both show this control; choose its visible support instance.
  await until(()=>evaluate(`Array.from(document.querySelectorAll('.initiation-support button')).some(e=>e.textContent===${JSON.stringify(`Focus ${second.title}`)}&&!e.matches(':disabled')&&e.getClientRects().length)`),'explicit alternate priority control');
  await evaluate(`(()=>{const e=Array.from(document.querySelectorAll('.initiation-support button')).find(e=>e.textContent===${JSON.stringify(`Focus ${second.title}`)});e.focus();})()`);
  await key('Enter','Enter',13);await until(async()=>(await workspace()).selected_objective_id===second.id,'support priority selection');
  await change('select_objective',{id:first.id});await refresh();
  const afterSupport=await workspace();assert.deepEqual(afterSupport.actions,beforeSupport.actions);assert.deepEqual(afterSupport.study,beforeSupport.study);assert.deepEqual(authority(afterSupport),authority(beforeSupport));
  assert.ok(!afterSupport.study.manifests.some(m=>m.status==='active'));
  checks.push('Installed voluntary smaller step and rest make no durable changes; another-priority choice changes only focus; no criterion, reward, debt, timer or experimental outcome is created');
  await resizeWindow(660,850);await evaluate('window.scrollTo(0,0)');assert.ok(await evaluate('document.documentElement.scrollWidth<=innerWidth+1'));await shot('daily-work-narrow.png');
  await resizeWindow(1080,820);await evaluate('window.scrollTo(0,0)');await shot('daily-work-normal.png');
  await clientSend('Emulation.setEmulatedMedia',{features:[{name:'prefers-reduced-motion',value:'reduce'}]});
  assert.ok(await evaluate("matchMedia('(prefers-reduced-motion: reduce)').matches"));
  await keyboard('Save where I stopped');await evaluate("document.querySelector('.checkpoint-editor').scrollIntoView({block:'start'})");await shot('daily-checkpoint.png');await clickButton('Discard checkpoint draft');
  checks.push('Installed Work stays first at normal and narrow window sizes without horizontal overflow; checkpoint/support are keyboard operable; checkpoint flow exercised with reduced-motion preference enabled (no screen-reader audit claimed)');

  const full=await exportFile('daily-complete.json');
  const unsafe={...full,schema_version:4};delete unsafe.daily;
  const unsafePath=path.join(root,'daily-unsafe-v4.json');await writeFile(unsafePath,JSON.stringify(unsafe));
  const beforeUnsafe=await workspace();
  await clickButton('Import records');nativeDialog('Preview an Ascension export','Select',unsafePath);
  await until(()=>evaluate("Array.from(document.querySelectorAll('[role=alert]')).some(e=>e.textContent.includes('checkpoint')||e.textContent.includes('priority'))"),'unsafe v4 rejection');await ready();
  assert.deepEqual(await workspace(),beforeUnsafe);
  checks.push('Installed modern v5 export includes exact objective order, checkpoint/correction/replay history and full authority; legacy v4 cannot erase Stage 4.5 history');

  // A distinct fresh disposable profile verifies setup-free ordinary planning and
  // failed protection independently of the already-completed daily snapshot.
  await closeApplication();await until(async()=>{await rename(profile,path.join(root,'daily-original-profile'));return true;},'closed original disposable profile released');await mkdir(profile);
  await writeFile(backupsRoot,'Synthetic unavailable backup destination');
  await launch();
  let w=await workspace();w=await change('create_project',{title:'Synthetic ordinary work'});
  w=await change('create_objective',{project_id:w.projects[0].id,title:'Document one synthetic example',criterion:'The example records input and output.'});
  w=await change('save_work',{objective_id:w.objectives[0].id,description:'Write the synthetic input.',stop_condition:'One input is recorded.',note:'Continue with the output.'});
  assert.equal(w.study.epochs.length,0);await refresh();
  await until(async()=>(await invoke('backup_status')).state==='warning','nonblocking backup failure');
  assert.equal((await workspace()).actions[0].description,'Write the synthetic input.');
  await summary('Work opportunities');await clickButton('Plan an opportunity');
  await input('opportunity-start','Write the first synthetic input label.');
  await timed('ordinary_plan_ui',async()=>{await clickButton('Confirm opportunity');await notice('Ordinary opportunity saved');});
  w=await workspace();assert.equal(w.study.epochs.length,1);assert.equal(w.study.manifests.length,0);
  const cue=new Date(Date.now()+2500).toISOString();
  await timed('ordinary_plan_ipc',()=>change('plan_ordinary_opportunity',{objective_id:w.objectives[0].id,meaningful_start:'Write the first synthetic input label.',cue_at:cue,timezone:w.study.epochs[0].timezone,available_minutes:30}));
  w=await workspace();assert.equal(w.study.epochs.length,1);assert.equal(w.study.manifests.length,0);assert.equal(w.study.assignments.length,0);assert.equal(w.progress.wallet_bc,0);
  await until(()=>Date.now()>Date.parse(cue)+100,'prospective cue passes');
  w=await change('record_opportunity_outcome',{opportunity_id:w.study.opportunities.find(o=>o.cue_at===cue).id,report:'started',started_at:cue,precision:'exact',uncertainty_seconds:null,note:'Synthetic observed first action.',technical_failure:false,uncertainty_reason:null,correction_reason:null,priority_advanced:null,quality_note:null,acceptance_id:null});
  assert.equal(w.study.outcomes[0].state,'observed_start');assert.equal(w.progress.wallet_bc,0);
  await refresh();await unlink(backupsRoot);await mkdir(backupsRoot);await invoke('retry_backup');
  await until(async()=>(await invoke('backup_status')).state==='ready','backup retry succeeds');
  // Showing this real short cue schedules its existing delivery observation,
  // briefly disabling controls and updating layout. Settle it before a dialog.
  const observedOpportunity=w.study.opportunities.find(o=>o.cue_at===cue).id;
  await evaluate("document.querySelector('.opportunity-cue')?.scrollIntoView({block:'center'})");
  await until(async()=>(await workspace()).study.exposures.some(e=>e.opportunity_id===observedOpportunity&&e.kind==='cue_delivered'),'ordinary cue delivery settles');
  await ready();
  checks.push('Fresh installed profile plans an ordinary prospective opportunity without any manual epoch, records a factual exact outcome with no study/assignment/BC, and retains committed edits through automatic backup failure and explicit retry');

  // Restore modern data into this different profile using actual Windows dialogs.
  await timed('complete_restore_including_dialog',()=>importFile(path.join(root,'daily-complete.json')));
  await exactExport(full,'daily-other-profile-roundtrip.json');
  await closeApplication();await until(async()=>{await rename(profile,path.join(root,'daily-ordinary-profile'));return true;},'closed ordinary disposable profile released');
  await launch();await timed('complete_restore_fresh_profile',()=>importFile(path.join(root,'daily-complete.json')));
  await until(async()=>(await invoke('backup_status')).state==='ready','full authority daily snapshot');
  const status=await invoke('backup_status');assert.equal(status.snapshot_count,1);
  const dailyFiles=(await readdir(backupsRoot)).filter(n=>/^daily-v1-\d{4}-\d{2}-\d{2}\.sqlite3$/.test(n));assert.equal(dailyFiles.length,1);
  const snapshot=new DatabaseSync(path.join(backupsRoot,dailyFiles[0]),{readOnly:true});
  try{assert.equal(snapshot.prepare('PRAGMA quick_check').get().quick_check,'ok');assert.deepEqual(snapshot.prepare('PRAGMA foreign_key_check').all(),[]);assert.equal(snapshot.prepare('SELECT COUNT(*) n FROM work_checkpoints').get().n,2);assert.equal(snapshot.prepare('SELECT SUM(amount_bc) bc FROM ledger_entries').get().bc,full.progress.wallet_bc);assert.equal(snapshot.prepare('SELECT COUNT(*) n FROM world_commands').get().n,full.world.commands.length);}finally{snapshot.close();}
  const recoveryBytes=await readFile(path.join(backupsRoot,dailyFiles[0]));
  await timed('saved_action_ipc',()=>change('save_work',{objective_id:first.id,description:'Temporary synthetic action after recovery point.',stop_condition:'Temporary stop.',note:'Temporary context.'}));
  await invoke('retry_backup');await until(async()=>(await invoke('backup_status')).state==='ready','same-day retry');
  assert.deepEqual(await readFile(path.join(backupsRoot,dailyFiles[0])),recoveryBytes,'Same local day never overwrites its verified snapshot');
  const preview=await invoke('preview_recovery');assert.equal(preview.incoming_counts.checkpoints,2);
  await timed('snapshot_restore_ipc',()=>invoke('restore_import',{token:preview.token,expected_generation:preview.generation}));await refresh();
  await exactExport(full,'daily-snapshot-recovered.json');
  await closeApplication();await launch();await exactExport(full,'daily-snapshot-reopened.json');
  assert.deepEqual((await workspace()).world,full.world);
  checks.push('Complete installed SQLite recovery drill: verified daily snapshot reopened independently, same-day retry creates no new point, preview/confirmed recovery preserves exact organization/checkpoints/rewards/frozen studies/Chronicle and survives restart in a different disposable profile');

  // Corrupt only this disposable primary. Recovery export must work without
  // reopening/replacing it, and the exported copy must preserve every identity.
  await closeApplication();const corruptPath=path.join(profile,'ascension.sqlite3');
  // A closed WAL-mode process may leave committed pages in its sidecars. Preserve
  // the whole database family so they cannot repair the intentionally invalid file.
  const primaryNames=['ascension.sqlite3','ascension.sqlite3-wal','ascension.sqlite3-shm'];
  const preservedPrimary=path.join(profile,'preserved-before-corruption');await mkdir(preservedPrimary);
  const presentPrimary=(await readdir(profile)).filter(name=>primaryNames.includes(name));
  assert.ok(presentPrimary.includes('ascension.sqlite3'));
  for(const name of presentPrimary)await rename(path.join(profile,name),path.join(preservedPrimary,name));
  for(const [caseName,invalidPrimary] of [['corrupt',Buffer.from('Synthetic invalid SQLite primary')],['empty',Buffer.alloc(0)]]) {
    await writeFile(corruptPath,invalidPrimary);await launch(true);
    await clickButton('Export recovery copy');
    const recoveryExport=path.join(root,`daily-${caseName}-primary-recovery.json`);
    nativeDialog('Export an Ascension recovery copy','Select',recoveryExport);
    await notice('Recovery copy exported');
    assert.deepEqual(JSON.parse(await readFile(recoveryExport,'utf8')),full);
    assert.ok((await readFile(corruptPath)).equals(invalidPrimary),'Recovery export must leave the failed primary byte-for-byte unchanged');
    assert.ok(await evaluate("!!document.querySelector('[role=alert]') && ['Retry connection','Export recovery copy'].every(label=>Array.from(document.querySelectorAll('button')).some(e=>e.textContent===label&&!e.disabled))"),'Recovery controls remain available after successful export');
    await shot(`daily-${caseName}-recovery-error.png`);await closeApplication();
    assert.ok((await readFile(corruptPath)).equals(invalidPrimary),'Closing the failed service must not replace the failed primary');
  }
  for(const name of (await readdir(profile)).filter(name=>primaryNames.includes(name)))await unlink(path.join(profile,name));
  for(const name of presentPrimary)await rename(path.join(preservedPrimary,name),path.join(profile,name));
  await launch();await exactExport(full,'daily-final.json');
  checks.push('Corrupt and zero-byte installed disposable primaries remain untouched while native Save exports the latest validated recovery snapshot with exact complete identity; restoring the preserved primary resumes the same context');
  await writeFile(path.join(root,'daily-timings.json'),JSON.stringify({timings,limitations:'Synthetic engineering timings only. Dialog operations include native automation; not human capture time. Retention across simulated local dates is in real-SQLite unit tests.'},null,2));
}
