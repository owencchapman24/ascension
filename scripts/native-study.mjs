// Installed Stage 3c verification. All records and shifted historical times are
// explicitly synthetic; this never enrolls a normal profile or observes a human study.
import assert from 'node:assert/strict';
import {readFile, writeFile, readdir} from 'node:fs/promises';
import {DatabaseSync} from 'node:sqlite';
import path from 'node:path';
import {randomUUID} from 'node:crypto';

const acknowledgements = ['e00_assessed','frozen_protocol','schedule_epoch','assignment_and_window','matched_support','as_assigned_analysis','voluntary_stop'];
const utc = value => new Date(value).toISOString();
function shiftStudy(records, delta) {
  const copy = structuredClone(records);
  const timestamps = new Set(['created_at','effective_from','cue_at','eligibility_decided_at','assigned_at','observed_at','started_at','deadline_at']);
  for (const rows of Object.values(copy.study)) if (Array.isArray(rows)) for (const row of rows) {
    for (const key of Object.keys(row)) if (timestamps.has(key) && row[key] !== null) row[key] = utc(Date.parse(row[key]) + delta);
  }
  return copy;
}

export async function verifyStudy(h) {
  const {evaluate,clientSend,clickButton,typeAt,workspace,until,notice,root,exportFile,previewFile,
    nativeDialog,ready,closeApplication,launch,checks} = h;
  const presentationTimings=[];
  async function change(command, fields) {
    // A visible-delivery effect may commit between the read and this write.
    // Only an explicit optimistic-concurrency rejection is safe to retry.
    for(let attempt=0;attempt<3;attempt++) {
      const input={expected_generation:(await workspace()).generation,...fields};
      const result=await evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, {input:${JSON.stringify(input)}}).then(value=>({ok:true,value}), error=>({ok:false,error}))`);
      if(result.ok)return result.value;
      if(result.error?.code!=='conflict'||attempt===2)assert.fail(`${command}: ${JSON.stringify(result.error)}`);
    }
  }
  async function clickMatch(expression) {
    await until(()=>evaluate(`(()=>{const e=${expression};if(!e)return false;const r=e.getBoundingClientRect();return e.getClientRects().length>0&&r.width>0&&r.height>0;})()`),'visible study control');
    const point = await evaluate(`(()=>{const e=${expression};if(!e)throw new Error('Missing Stage 3c control');e.scrollIntoView({block:'center'});const r=e.getBoundingClientRect();return{x:r.x+r.width/2,y:r.y+r.height/2};})()`);
    await clientSend('Input.dispatchMouseEvent',{type:'mousePressed',button:'left',clickCount:1,...point});
    await clientSend('Input.dispatchMouseEvent',{type:'mouseReleased',button:'left',clickCount:1,...point});
  }
  async function openSummary(prefix) {
    const selector = `Array.from(document.querySelectorAll('summary')).find(e=>e.textContent.trim().startsWith(${JSON.stringify(prefix)}))`;
    if (!await evaluate(`(${selector})?.parentElement.open`)) await clickMatch(selector);
    await until(()=>evaluate(`(${selector})?.parentElement.open===true`),'open study disclosure');
  }
  async function input(id,text) { await clickMatch(`document.getElementById(${JSON.stringify(id)})`); await typeAt(id,text); }
  async function refresh() { await clientSend('Page.reload'); await until(()=>evaluate("document.querySelector('#work-heading')?.textContent==='Work'"),'Stage 3c reload'); }
  async function restore(records,name) {
    const target = path.join(root,name); await writeFile(target,JSON.stringify(records));
    await previewFile(target,await workspace(),records);
    await clickButton('Replace local records');
    await until(()=>evaluate("!document.querySelector('[role=dialog]')"),'study restore');
    return workspace();
  }
  async function screenshot(name) { const shot=await clientSend('Page.captureScreenshot',{format:'png'});await writeFile(path.join(root,name),Buffer.from(shot.data,'base64')); }

  // Reconstruct a populated Stage 3b schema using the just-verified synthetic
  // reward history. All new tables must be empty before dropping them. Exact
  // unchanged migrations 001–003 and their constraints remain in this database.
  const beforeMigration=await workspace();
  assert.equal(beforeMigration.progress.wallet_bc,4);
  await closeApplication();
  const database=path.join(root,'profile','ascension.sqlite3');
  const db=new DatabaseSync(database);
  const tables=['world_commands','world_spending','world_chapter','presentation_preferences','study_reflections','study_burdens','study_exposures','opportunity_outcomes','study_assignments','work_opportunities','study_protocol_events','study_manifests','schedule_epochs'];
  try {
    assert.equal(db.prepare('PRAGMA user_version').get().user_version,5);
    db.exec('PRAGMA foreign_keys=ON;BEGIN IMMEDIATE');
    for(const table of tables) {
      const exists=db.prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name=?").get(table);
      if(exists){assert.equal(db.prepare(`SELECT COUNT(*) AS n FROM ${table}`).get().n,0);db.exec(`DROP TABLE ${table}`);}
    }
    db.exec('PRAGMA user_version=3;COMMIT');
  } finally {db.close();}
  await launch();
  const migrated=await workspace();
  for(const key of ['projects','objectives','actions','notes','progress'])assert.deepEqual(migrated[key],beforeMigration[key]);
  assert.equal(migrated.study.opportunities.length,0);assert.equal(migrated.study.manifests.length,0);
  const backupNames=(await readdir(path.join(root,'profile','backups'))).filter(n=>n.endsWith('.sqlite3'));
  assert.equal(backupNames.length,1);
  const backup=new DatabaseSync(path.join(root,'profile','backups',backupNames[0]),{readOnly:true});
  try{assert.equal(backup.prepare('PRAGMA user_version').get().user_version,3);assert.equal(backup.prepare('SELECT SUM(amount_bc) AS bc FROM ledger_entries').get().bc,4);}finally{backup.close();}
  checks.push('Installed populated Stage 3b schema-v3 migration preserves exact accepted/corrected authority history and four BC, creates coherent v3 backup, enrolls nobody');

  await openSummary('Work opportunities'); await openSummary('Schedules and optional studies');
  await input('epoch-label','Synthetic ordinary routine');await clickButton('Save schedule epoch');await notice('Schedule epoch saved');
  await clickButton('Configure E00');await notice('E00 configured');
  await clickButton('Review E00 lifecycle');await clickButton('Begin E00 feasibility logging');await notice('E00 is now active');
  assert.equal(await evaluate("window.__TAURI_INTERNALS__.invoke('load_world').then(()=>false,()=>true)"),true,'River Reach is withheld during active E00');
  await clickButton('World');
  await until(()=>evaluate("document.querySelector('.world-withheld')?.textContent.includes('E00')"),'explicit E00 strategic-game isolation');
  assert.equal(await evaluate("document.querySelector('.river-reach')"),null);
  await clickButton('Work');
  await clickButton('Plan an opportunity');
  await input('opportunity-start','Inspect the synthetic example and document one testable finding.');
  await clickButton('Confirm opportunity');await notice('Opportunity saved');
  const planned=await workspace();const first=planned.study.opportunities[0];
  assert.equal(first.action_snapshot,planned.actions.find(a=>a.objective_id===first.objective_id).description);
  assert.equal(first.meaningful_start,'Inspect the synthetic example and document one testable finding.');
  assert.equal(first.outcome_state,'pending');assert.equal(planned.study.assignments.length,0);
  assert.deepEqual(planned.progress,migrated.progress);
  await closeApplication();await launch();
  assert.deepEqual((await workspace()).study.opportunities,planned.study.opportunities);
  assert.ok(await evaluate("document.querySelector('.saved-action').textContent.includes('Write the first example step.')"));
  checks.push('Installed E00 UI creates a prospective action/start-definition snapshot and retains it through offline restart; no assignment or grant is created');

  const e00=planned.study.manifests.find(m=>m.phase==='e00');const epoch=planned.study.epochs[0];
  const planInput={objective_id:first.objective_id,epoch_id:epoch.id,manifest_id:e00.id,domain:'synthetic coding',meaningful_start:'Inspect one code path and record a testable diagnosis.',timezone:'UTC',window_seconds:900,eligible:true,unavailability:null,priority:'ordinary',available_minutes:30,deadline_at:null,context:null};
  await change('plan_opportunity',{...planInput,cue_at:utc(Date.now()+35*60_000)});
  await change('plan_opportunity',{...planInput,cue_at:utc(Date.now()+40*60_000)});
  await refresh();
  const prospective=await exportFile('study-prospective-synthetic.json');
  // Shift only declared synthetic study times together, retaining relative
  // ordering and real frozen identities. Import validates the historical fixture.
  await restore(shiftStudy(prospective,-90*60_000),'study-historical-synthetic.json');
  let current=await workspace();
  assert.ok(current.study.opportunities.every(o=>o.outcome_state==='missing'));
  assert.equal(current.study.outcomes.length,0);
  await openSummary('Work opportunities');await openSummary('Opportunity history');
  const observations=[...current.study.opportunities].sort((a,b)=>a.cue_at.localeCompare(b.cue_at));
  await until(()=>evaluate(`document.querySelector('[data-opportunity-id="${observations[0].id}"] button')!==null`),'lazy opportunity history');
  await clickMatch(`document.querySelector('[data-opportunity-id="${observations[0].id}"] button')`);
  await clickMatch("document.querySelector('input[name=outcome-report][value=nonstart]')");
  await input('outcome-note','Synthetic observation: no qualifying start occurred in the window.');
  await clickButton('Save observation');await notice('Observation saved');
  await change('record_opportunity_outcome',{opportunity_id:observations[1].id,report:'started',started_at:utc(Date.parse(observations[1].cue_at)+120_000),precision:'exact',uncertainty_seconds:null,note:'Synthetic factual start: inspected and documented the chosen code path.',technical_failure:false,uncertainty_reason:null,correction_reason:null,priority_advanced:true,quality_note:'The testable diagnosis names the inspected branch.',acceptance_id:null});
  await change('record_opportunity_outcome',{opportunity_id:observations[0].id,report:'started',started_at:utc(Date.parse(observations[0].cue_at)+1_200_000),precision:'exact',uncertainty_seconds:null,note:'A later synthetic start was reported.',technical_failure:false,uncertainty_reason:null,correction_reason:'Add a late start without changing the primary nonstart.',priority_advanced:null,quality_note:null,acceptance_id:null});
  await change('record_study_burden',{opportunity_id:observations[1].id,category:'logging',minutes:1,note:'Synthetic exclusive estimate.'});
  await change('record_study_reflection',{manifest_id:e00.id,note:'Synthetic feasibility reflection only; no human participation or product benefit is asserted.'});
  current=await workspace();
  assert.equal(current.study.opportunities.find(o=>o.id===observations[1].id).outcome_state,'observed_start');
  assert.equal(current.study.opportunities.find(o=>o.id===observations[2].id).outcome_state,'missing');
  const late=current.study.outcomes.find(o=>o.sequence===2);
  assert.equal(late.state,'observed_nonstart');assert.equal(late.late_start,true);assert.equal(late.censored_at_seconds,900);assert.equal(late.latency_seconds,null);
  assert.equal(current.study.outcomes.filter(o=>o.opportunity_id===observations[0].id).length,2);
  assert.deepEqual(current.progress,migrated.progress);
  checks.push('Installed observation UI and real Rust commands distinguish start, nonstart, unobserved missing, and appended late-start correction, with burden/quality/reflection and no reward mutation');

  await change('transition_study',{manifest_id:e00.id,status:'completed',reason:'Synthetic fixture closes E00; this is not a human feasibility assessment.',acknowledgements:[]});
  await change('configure_study',{phase:'e01',epoch_id:epoch.id});
  current=await workspace();const e01=current.study.manifests.find(m=>m.phase==='e01');
  const denied=await evaluate(`window.__TAURI_INTERNALS__.invoke('transition_study',{input:${JSON.stringify({expected_generation:current.generation,manifest_id:e01.id,status:'active',reason:'Synthetic unauthorized direct transition must fail.',acknowledgements:[]})}}).then(()=>null,error=>error)`);
  assert.equal(denied.code,'invalid_input');
  await change('transition_study',{manifest_id:e01.id,status:'ready',reason:'Synthetic owner readiness fixture; isolated profile only.',acknowledgements});
  await change('transition_study',{manifest_id:e01.id,status:'active',reason:'Synthetic test authorization after simulated feasibility assessment; never a real enrollment.',acknowledgements});
  const anchor=Date.now();
  for(let index=0;index<4;index++)await change('plan_opportunity',{...planInput,manifest_id:e01.id,cue_at:utc(anchor+(30+index*20)*60_000)});
  current=await workspace();assert.equal(current.study.assignments.length,4);
  assert.equal(await evaluate("window.__TAURI_INTERNALS__.invoke('load_world').then(()=>false,()=>true)"),true,'River Reach is withheld during active E01');
  await refresh(); // The synthetic activation above uses IPC rather than the UI adoption path.
  await clickButton('World');
  await until(()=>evaluate("document.querySelector('.world-withheld')?.textContent.includes('E01')"),'explicit E01 strategic-game isolation');
  assert.equal(await evaluate("document.querySelector('.river-reach')"),null);
  assert.ok(await evaluate("Array.from(document.querySelectorAll('button')).some(e=>e.textContent==='Return to study settings')"));
  await clickButton('Work');
  assert.ok(current.study.assignments.every(a=>a.condition===null));
  assert.ok(!('seed' in current.study.manifests.find(m=>m.id===e01.id)));assert.ok(!('allocation' in current.study.manifests.find(m=>m.id===e01.id)));
  await refresh();const assigned=await exportFile('study-assigned-complete.json');
  const assignments=assigned.study.assignments.filter(a=>a.manifest_id===e01.id).sort((a,b)=>a.position-b.position);
  assert.equal(assignments.filter(a=>a.condition==='d').length,2);assert.equal(assignments.filter(a=>a.condition==='d_plus_b').length,2);
  const frozen=assigned.study.manifests.find(m=>m.id===e01.id);
  assert.equal(frozen.allocation.length,60);assert.ok(frozen.seed&&frozen.allocation_hash);
  await closeApplication();await launch();assert.deepEqual((await workspace()).study.assignments,current.study.assignments);
  checks.push('Installed E01 rejects accidental activation, exercises explicit synthetic readiness/activation, freezes balanced allocations, hides future conditions, and never rerolls on restart');

  const previewAssignment=assignments.find(a=>a.condition==='d_plus_b');
  const previewOpportunity=assigned.study.opportunities.find(o=>o.id===previewAssignment.opportunity_id);
  await restore(shiftStudy(assigned,Date.now()+45_000-Date.parse(previewOpportunity.cue_at)),'study-pre-cue-preview.json');
  assert.equal((await workspace()).study.presentation.boundary,'preview');
  await evaluate("document.querySelector('.world-panel').scrollIntoView({block:'center'})");
  await until(()=>evaluate("document.querySelector('.horizon-preview')?.getAttribute('data-restored-units')==='4'"),'fixed pre-cue H1 preview');
  await until(async()=>(await workspace()).study.exposures.some(e=>e.kind==='preview_delivered'&&e.opportunity_id===previewAssignment.opportunity_id),'visible pre-cue delivery');
  assert.ok(!(await workspace()).study.exposures.some(e=>e.kind==='seen'));
  await screenshot('h1-preview.png');

  for(const condition of ['d','d_plus_b']) {
    const target=assignments.find(a=>a.condition===condition);
    const opportunity=assigned.study.opportunities.find(o=>o.id===target.opportunity_id);
    const delta=Date.now()-16*60_000-Date.parse(opportunity.cue_at);
    await restore(shiftStudy(assigned,delta),`study-${condition}-checkpoint.json`);
    current=await workspace();assert.equal(current.study.presentation.condition,condition);assert.equal(current.study.presentation.boundary,'checkpoint');
    assert.ok(await evaluate("document.querySelector('.numeric-progress').textContent.includes('4 issued Build Credits')"));
    if(condition==='d') {assert.equal(await evaluate("document.querySelector('.horizon-scene')"),null);}
    else {
      await evaluate("document.querySelector('.world-panel').scrollIntoView({block:'center'})");
      await until(()=>evaluate("document.querySelector('.horizon-scene')?.getAttribute('data-restored-units')==='4'"),'four exact H1 units');
      assert.equal(await evaluate("document.querySelectorAll('[data-restored=true]').length"),4);
      await until(async()=>(await workspace()).study.exposures.some(e=>e.opportunity_id===target.opportunity_id&&e.kind==='checkpoint_delivered'),'delivered construction record');
      assert.ok(!(await workspace()).study.exposures.some(e=>e.kind==='seen'));
      await screenshot('h1-four-units.png');
    }
    assert.deepEqual((await workspace()).progress,migrated.progress);
    assert.equal((await workspace()).study.assignments.length,4);
  }
  checks.push('P5T029/P5T030 installed synthetic arms retain identical support/numeric feedback/ledger and all assignments; D hides H1, D_PLUS_B displays exactly four retained-credit units, delivery does not imply seen');

  current=await workspace();
  const interrupted=current.study.presentation.opportunity_id;
  await change('record_opportunity_outcome',{opportunity_id:interrupted,report:'unknown',started_at:null,precision:'unknown',uncertainty_seconds:null,note:null,technical_failure:true,uncertainty_reason:'Synthetic technical failure: observation was unavailable.',correction_reason:null,priority_advanced:null,quality_note:null,acceptance_id:null});
  await change('record_study_exposure',{opportunity_id:interrupted,kind:'deviation',note:'Synthetic post-assignment interruption; retain original eligibility and assignment.'});
  await refresh();current=await workspace();
  assert.equal(current.study.assignments.length,4);
  assert.equal(current.study.outcomes.find(e=>e.opportunity_id===interrupted).latency_seconds,null);
  const complete=await exportFile('study-complete.json');
  const legacy={...complete,schema_version:2};delete legacy.study;delete legacy.world;
  const legacyPath=path.join(root,'unsafe-stage3b.json');await writeFile(legacyPath,JSON.stringify(legacy));
  await clickButton('Import records');nativeDialog('Preview an Ascension export','Select',legacyPath);
  await until(()=>evaluate("Array.from(document.querySelectorAll('[role=alert]')).some(e=>e.textContent.includes('study history')||e.textContent.includes('opportunity'))"),'unsafe v2 import rejected');await ready();
  assert.deepEqual((await workspace()).progress,current.progress);assert.equal((await workspace()).study.assignments.length,4);
  const broken=structuredClone(complete);broken.study.manifests.find(m=>m.id===e01.id).allocation_hash='0'.repeat(64);
  const brokenPath=path.join(root,'broken-study.json');await writeFile(brokenPath,JSON.stringify(broken));
  await clickButton('Import records');nativeDialog('Preview an Ascension export','Select',brokenPath);
  await until(()=>evaluate("document.querySelector('[role=alert]')!==null"),'malformed study rejected');await ready();
  assert.equal((await workspace()).study.assignments.length,4);
  const empty=structuredClone(complete);empty.selected_objective_id=null;for(const key of ['projects','objectives','actions','notes'])empty[key]=[];
  empty.progress={registry_revision:0,parents:[],slots:[],criteria:[],evidence:[],validators:[],snapshots:[],acceptances:[],corrections:[],grants:[],ledger:[],audit:[],commands:[],wallet_bc:0};
  for(const key of Object.keys(empty.study))if(Array.isArray(empty.study[key]))empty.study[key]=[];
  await restore(empty,'study-empty.json');await restore(complete,'study-restored.json');
  const exported=await exportFile('study-roundtrip.json');assert.deepEqual(exported,complete);
  await closeApplication();await launch();
  const reopened=await workspace();assert.equal(reopened.progress.wallet_bc,4);assert.equal(reopened.study.assignments.length,4);
  assert.deepEqual(reopened.study.outcomes,complete.study.outcomes);assert.deepEqual(reopened.progress,complete.progress);
  checks.push('Native v4 export/empty restore/exact re-export/offline restart preserve frozen allocation, missing assigned opportunities, corrections and retained H1 progress without reminting; unsafe v2 and malformed hash imports are rejected');
  await screenshot('study-final.png');

  // Exercise saturation using actual synthetic evidence/acceptance commands,
  // never edited wallet values or renderer-supplied rewards.
  await change('transition_study',{manifest_id:e01.id,status:'completed',reason:'End isolated synthetic test protocol; no human experiment occurred.',acknowledgements:[]});
  await change('set_construction_enabled',{enabled:false});
  await refresh();
  const savedSelection=reopened.selected_objective_id;
  async function earnSyntheticGrant(index) {
    let w=await change('create_objective',{project_id:reopened.projects[0].id,title:`Synthetic saturation outcome ${index}`,criterion:'One synthetic example has a documented input and result.'});
    const parent=w.objectives.find(o=>o.id===w.selected_objective_id);
    const criterion=w.progress.criteria.find(c=>c.parent_id===parent.id);
    const basis={parent_id:parent.id,criterion_lineage_id:criterion.criterion_lineage_id,criterion_version:criterion.criterion_version};
    await change('lock_criterion',basis);
    w=await change('record_evidence',{...basis,kind:'user_attestation',description:'I inspected the synthetic input and result against the prospectively locked example criterion.',reference:null,observed_at:null,status:'supports',assistance:'none',uncertainty:null,supersedes:null});
    const review=await change('review_progress',{...basis,evidence_refs:w.progress.evidence.filter(e=>e.parent_id===parent.id).map(e=>e.id),factual_summary:'The synthetic example contains its required documented input and result.',assistance:'none'});
    const s=review.snapshot;
    const accepted=await change('accept_progress',{schema_version:1,command_id:randomUUID(),...basis,expected_parent_revision:review.workspace.objectives.find(o=>o.id===parent.id).revision,reviewed_snapshot_id:s.id,reviewed_snapshot_digest:s.digest,evidence_refs:s.evidence_refs,factual_summary:s.factual_summary,assistance:'none'});
    assert.equal(accepted.result.issued_bc,4);
  }
  for(let index=0;index<15;index++)await earnSyntheticGrant(index);
  assert.equal((await workspace()).progress.wallet_bc,64);
  await change('select_objective',{id:savedSelection});
  await refresh();assert.equal(await evaluate("document.querySelector('.horizon-scene')"),null);
  await change('set_construction_enabled',{enabled:true});await refresh();
  const completeRenderStarted=performance.now();
  await clickButton('View construction');
  await until(()=>evaluate("document.querySelector('.horizon-scene')?.getAttribute('data-restored-units')==='64'"),'64 native restoration units');
  presentationTimings.push({units:64,automation_open_to_dom_ms:Number((performance.now()-completeRenderStarted).toFixed(2))});
  await evaluate("document.querySelector('.horizon-scene').scrollIntoView({block:'center'})");
  assert.equal(await evaluate("document.querySelectorAll('[data-restored=true]').length"),64);
  await until(async()=>(await workspace()).study.exposures.some(e=>e.kind==='voluntary_world'&&e.issued_bc===64),'64-unit voluntary delivery');
  await ready();
  await screenshot('h1-complete.png');
  await earnSyntheticGrant(15);await change('select_objective',{id:savedSelection});
  await refresh();await clickButton('View construction');
  await until(()=>evaluate("document.querySelector('.horizon-scene')?.getAttribute('data-restored-units')==='64'"),'saturated native restoration');
  await evaluate("document.querySelector('.horizon-scene').scrollIntoView({block:'center'})");
  assert.equal((await workspace()).progress.wallet_bc,68);
  assert.equal(await evaluate("document.querySelectorAll('[data-unit-id]').length"),64);
  await until(async()=>(await workspace()).study.exposures.some(e=>e.h1_saturated&&e.issued_bc===68),'explicit saturation exposure');
  await clientSend('Emulation.setDeviceMetricsOverride',{width:660,height:850,deviceScaleFactor:1,mobile:false});
  await evaluate("document.querySelector('.horizon-scene').scrollIntoView({block:'center'})");
  await screenshot('h1-narrow.png');
  assert.ok(await evaluate("document.documentElement.scrollWidth<=window.innerWidth+1"),'narrow desktop avoids horizontal overflow');
  await clientSend('Emulation.clearDeviceMetricsOverride');
  await clickButton('Hide construction');
  await ready();
  const saturated=await exportFile('study-saturated.json');
  await restore(empty,'study-saturation-empty.json');await restore(saturated,'study-saturation-restored.json');
  assert.deepEqual(await exportFile('study-saturation-roundtrip.json'),saturated);
  await closeApplication();await launch();
  assert.equal((await workspace()).progress.wallet_bc,68);
  assert.equal((await workspace()).progress.grants.length,17);
  assert.ok(await evaluate("document.querySelector('.saved-action').textContent.includes('Write the first example step.')"));
  await writeFile(path.join(root,'presentation-timings.json'),JSON.stringify({presentationTimings,limitations:'Includes CDP automation round trips and polling; warm installed DOM presentation, not an isolated browser rendering benchmark.'},null,2));
  checks.push('Installed authentic synthetic Stage 3b grants advance hidden H1 to 64; further issuance reaches 68 without adding units/spending, saturation is recorded, narrow layout fits, exact saturated export/restore/restart retains 17 unique grants and saved action');
}
