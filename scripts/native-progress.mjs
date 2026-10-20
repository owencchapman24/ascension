// Stage 3b checks reuse the installed-WebView2 and real file-dialog harness.
import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import path from 'node:path';
import {randomUUID} from 'node:crypto';

export async function verifyProgress(h) {
  const {evaluate,clientSend,clickButton,typeAt,workspace,until,notice,root,recordSet,exportFile,restoreFile,
    nativeDialog,ready,closeApplication,launch,checks,emptyProgress} = h;
  const invoke = (command,input) => evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, {input:${JSON.stringify(input)}}).then(value=>({ok:true,value}), error=>({ok:false,error}))`);
  async function clickMatch(expression) {
    const point=await evaluate(`(()=>{const e=${expression}; if(!e)throw new Error('Missing native progress control');e.scrollIntoView({block:'center'});const r=e.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2};})()`);
    await clientSend('Input.dispatchMouseEvent',{type:'mousePressed',button:'left',clickCount:1,...point});
    await clientSend('Input.dispatchMouseEvent',{type:'mouseReleased',button:'left',clickCount:1,...point});
  }
  const openPanel=()=>clickMatch("document.querySelector('#progress-heading')");
  const inputText=async(id,text)=>{await clickMatch(`document.getElementById(${JSON.stringify(id)})`);await typeAt(id,text);};
  await openPanel();
  assert.equal(await evaluate("document.querySelector('.reward-summary')"),null,'Fictional progression starts hidden');
  await clickMatch("Array.from(document.querySelectorAll('label')).find(e=>e.textContent.includes('I have reviewed this criterion')).querySelector('input')");
  await clickButton('Lock criterion'); await notice('Criterion locked');
  let current=await workspace();
  assert.equal(current.progress.criteria[0].state,'locked');
  await clickButton('Record evidence');
  await inputText('evidence-description','I prepared and reviewed the three labeled synthetic steps after declaring this criterion.');
  await clickButton('Save evidence');
  await notice('Evidence saved');
  await until(async()=> (await workspace()).progress.evidence.length===1,'native evidence commit');
  await clickButton('Review progress');
  await inputText('review-summary','The synthetic example now contains three reviewed labeled steps.');
  await clickButton('Save review snapshot'); await notice('Review snapshot saved');
  current=await workspace();
  const beforeCorrectionReview=current.progress.snapshots[0];
  await clickButton('Review progress');
  await inputText('review-summary','The current synthetic review confirms the three labeled steps.');
  await clickButton('Save review snapshot'); await notice('Review snapshot saved');
  await until(async()=> (await workspace()).progress.snapshots.length===2,'second native review commit');
  current=await workspace();
  const snapshot=current.progress.snapshots.find(s=>s.id!==beforeCorrectionReview.id);
  assert.equal(snapshot.eligible,true); assert.deepEqual(snapshot.missing,[]); assert.deepEqual(snapshot.conflicts,[]);
  await clickButton('Accept qualifying progress'); await notice('Progress accepted');
  const awarded=await workspace();
  assert.equal(awarded.progress.wallet_bc,4); assert.equal(awarded.progress.grants.length,1);
  assert.equal(awarded.progress.ledger.length,1); assert.equal(awarded.progress.acceptances.length,1);
  assert.equal(awarded.progress.acceptances[0].actor,'local_user');
  checks.push('Installed UI locks a prospective criterion, records self-attested evidence, reviews the complete snapshot, accepts and issues exactly four BC');
  const parent=awarded.objectives[0];
  const duplicate={schema_version:1,command_id:randomUUID(),expected_generation:awarded.generation,parent_id:parent.id,
    criterion_lineage_id:snapshot.criterion_lineage_id,criterion_version:snapshot.criterion_version,expected_parent_revision:parent.revision,
    reviewed_snapshot_id:snapshot.id,reviewed_snapshot_digest:snapshot.digest,evidence_refs:snapshot.evidence_refs,
    factual_summary:snapshot.factual_summary,assistance:snapshot.assistance};
  const attempted=await invoke('accept_progress',duplicate);
  assert.ok(!attempted.ok || attempted.value.result.issued_bc===0);
  assert.equal((await workspace()).progress.wallet_bc,4); assert.equal((await workspace()).progress.grants.length,1);
  checks.push('Installed Rust command rejects/reconciles a new-ID duplicate acceptance without another grant');
  const beforeRestart=await workspace();
  await closeApplication(); await launch();
  assert.deepEqual(await workspace(),beforeRestart);
  checks.push('Accepted evidence, immutable reward entitlement, grant and ledger survive installed-app restart offline');
  await openPanel();
  await clickButton('Correct this accomplishment');
  await inputText('correction-reason','The earlier synthetic review overlooked a missing explanation; retract its factual acceptance.');
  await clickButton('Save factual correction'); await notice('Factual correction saved');
  const corrected=await workspace();
  assert.equal(corrected.progress.acceptances[0].status,'retracted');
  assert.equal(corrected.progress.corrections.length,1); assert.equal(corrected.progress.wallet_bc,4);
  const staleReview=await invoke('accept_progress',{...duplicate,command_id:randomUUID(),expected_generation:corrected.generation,
    reviewed_snapshot_id:beforeCorrectionReview.id,reviewed_snapshot_digest:beforeCorrectionReview.digest,
    evidence_refs:beforeCorrectionReview.evidence_refs,factual_summary:beforeCorrectionReview.factual_summary,assistance:beforeCorrectionReview.assistance});
  assert.equal(staleReview.ok,false,'A review made before correction cannot reaccept afterward');
  assert.deepEqual(await workspace(),corrected);
  checks.push('Installed Rust command rejects an unused pre-correction review without changing factual history or retained credit');
  await clickButton('Record evidence');
  await inputText('evidence-description','The missing synthetic explanation was completed and reviewed against the same criterion.');
  await clickButton('Save evidence');
  await notice('Evidence saved');
  await until(async()=> (await workspace()).progress.evidence.length===2,'corrected evidence saved');
  await clickButton('Review progress');
  await inputText('review-summary','All three synthetic steps now include their complete explanation.');
  await clickButton('Save review snapshot'); await notice('Review snapshot saved');
  await clickButton('Accept qualifying progress'); await notice('Progress accepted');
  const reaccepted=await workspace();
  assert.equal(reaccepted.progress.acceptances.length,2); assert.equal(reaccepted.progress.grants.length,1);
  assert.equal(reaccepted.progress.ledger.length,1); assert.equal(reaccepted.progress.wallet_bc,4);
  assert.equal(reaccepted.progress.acceptances.filter(a=>a.status==='accepted').length,1);
  checks.push('Installed UI factual correction retains courtesy credit; fresh evidence/review/reacceptance creates factual history without reminting');
  await clickMatch("Array.from(document.querySelectorAll('label')).find(e=>e.textContent.includes('Show fictional rewards')).querySelector('input')");
  assert.ok(await evaluate("document.querySelector('.reward-summary').textContent.includes('4 Build Credits')"));
  const shot=await clientSend('Page.captureScreenshot',{format:'png'});
  await writeFile(path.join(root,'progress-view.png'),Buffer.from(shot.data,'base64'));
  const full=await exportFile('stage3b-complete.json');
  assert.deepEqual(full,recordSet(reaccepted));
  const legacyPath=path.join(root,'unsafe-legacy.json');
  const {progress:_progress,study:_study,...legacyWork}=full;
  await writeFile(legacyPath,JSON.stringify({...legacyWork,schema_version:1}));
  await clickButton('Import records'); nativeDialog('Preview an Ascension export','Select',legacyPath);
  await until(()=>evaluate("Array.from(document.querySelectorAll('[role=alert]')).some(e=>e.textContent.includes('cannot replace earned reward history'))"),'unsafe legacy import rejected');
  await ready(); assert.deepEqual(await workspace(),reaccepted);
  checks.push('Actual Windows Open dialog rejects a legacy file that would erase earned history, without state changes');
  const empty={...full,selected_objective_id:null,projects:[],objectives:[],actions:[],notes:[],progress:emptyProgress()};
  const emptyPath=path.join(root,'stage3b-empty.json'); await writeFile(emptyPath,JSON.stringify(empty));
  await restoreFile(emptyPath,reaccepted,empty);
  await restoreFile(path.join(root,'stage3b-complete.json'),await workspace(),full);
  const restored=await workspace();
  await restoreFile(path.join(root,'stage3b-complete.json'),restored,full);
  assert.deepEqual(await workspace(),restored);
  assert.deepEqual(await exportFile('stage3b-restored.json'),full);
  checks.push('Native complete modern export, explicitly confirmed replacement, restore into empty state and replay preserve exact criteria/evidence/corrections/grants/ledger/command identities');
  await closeApplication(); await launch();
  assert.deepEqual(await workspace(),restored);
  assert.ok(await evaluate("document.querySelector('.saved-action')?.textContent.includes('Write the first example step.')"));
  checks.push('Restored Stage 3b state survives offline installed restart and immediately exposes the saved next action');
}
