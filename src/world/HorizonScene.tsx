import { useId, type ReactNode } from "react";
import "./HorizonScene.css";

/** A presentation policy, not a reward policy. Never infer issuance from acceptances. */
export const HORIZON_POLICY = "h1-ledger-64-v1";
export const HORIZON_UNIT_COUNT = 64;

export function getRestoredUnitCount(issuedBC: number): number {
  return Number.isSafeInteger(issuedBC) && issuedBC >= 0 ? Math.min(HORIZON_UNIT_COUNT, issuedBC) : 0;
}

type Unit = {
  id: string;
  label: string;
  district: string;
  layer: number;
  draw: (restored: boolean) => ReactNode;
};

function StonePath({ d, restored }: { d: string; restored: boolean }) {
  return restored ? <>
    <path d={d} fill="none" stroke="#8c8970" strokeWidth="25" strokeLinecap="round" />
    <path d={d} fill="none" stroke="#e7d8b5" strokeWidth="21" strokeLinecap="round" />
    <path d={d} fill="none" stroke="#b2a48b" strokeWidth="18" strokeDasharray="2 12" />
  </> : <path d={d} fill="none" stroke="#a4a080" strokeWidth="15" strokeDasharray="8 12" strokeLinecap="round" opacity=".65" />;
}

function HouseWalls({ x, y, restored }: { x: number; y: number; restored: boolean }) {
  return <g transform={`translate(${x} ${y})`}>
    <ellipse cy="3" rx="43" ry="13" fill="#344d4426" />
    {restored ? <>
      <path d="M-33-44H24V0H-33Z" fill="#e5d5ac" stroke="#8b8a70" strokeWidth="1.6" />
      <path d="M24-44 40-37V-7L24 0Z" fill="#bfae87" />
      <path d="M-33-8H24M-33-31H24" stroke="#c4b38e" strokeWidth="1.5" />
      <path d="M-7 0V-23Q1-31 9-23V0" fill="#52706b" />
      <path d="M-24-29H-15V-17H-24ZM15-29H22V-17H15Z" fill="#6a8e86" stroke="#fbebc4" strokeWidth="2" />
      <path d="M-8 0H10" stroke="#f8ebce" strokeWidth="3" />
    </> : <>
      <path d="M-33 0V-24L-23-21-18-32-8-27V-12L5-17 14-13 24-22V0Z" fill="#a5a890" stroke="#8b927b" strokeWidth="1.5" />
      <path d="M24-22 40-16V-7L24 0Z" fill="#8f987f" />
      <path d="M-25-10-17-16-11-9M8-2 12-10" fill="none" stroke="#77866f" />
      <path d="M-38 5-29-1-20 4M17 7 28 2 37 7" fill="#9b9e86" />
    </>}
  </g>;
}

function HouseRoof({ x, y, restored, teal = false }: { x: number; y: number; restored: boolean; teal?: boolean }) {
  return <g transform={`translate(${x} ${y})`}>
    {restored ? <>
      <path d="M-41-43-6-70 32-43Z" fill={teal ? "#486c66" : "#ab6650"} stroke={teal ? "#365953" : "#89523f"} strokeWidth="1.5" />
      <path d="M-6-70 17-66 47-37 32-43Z" fill={teal ? "#365b55" : "#87543f"} />
      <path d="M-30-48H21M-23-54H12M-15-60H4" stroke={teal ? "#6e8e7e" : "#cf9371"} strokeWidth="1.5" />
      <path d="M19-57V-73H27V-49" fill="#c4b18e" stroke="#958a6f" strokeWidth="1.5" />
      <path d="M18-73H28" stroke="#efe0bc" strokeWidth="3" />
    </> : <path d="M-39-41-8-64 8-51M-8-64 16-59" fill="none" stroke="#7c8b75" strokeWidth="4" strokeLinecap="round" />}
  </g>;
}

function Garden({ x, y, restored, flowers = false }: { x: number; y: number; restored: boolean; flowers?: boolean }) {
  return <g transform={`translate(${x} ${y})`}>
    <path d="M-29 0 4-12 34 0 0 14Z" fill={restored ? "#b99c73" : "#a0ad87"} stroke={restored ? "#e1d3ad" : "#8e9e7c"} strokeWidth={restored ? 4 : 1} />
    {restored ? <>
      <path d="M-16 1 5-7M-6 6 16-3M5 10 26 2" stroke="#6c8257" strokeWidth="5" strokeLinecap="round" />
      <path d="M-15-3V1M-3-7V-3M0 1V5M12-4V0M15 4V8" stroke="#345e47" strokeWidth="2" />
      {flowers && <g fill="#ecc589"><circle cx="-15" cy="-4" r="3" /><circle cx="-3" cy="-8" r="3" /><circle cx="0" cy="0" r="3" /><circle cx="12" cy="-5" r="3" /></g>}
    </> : <path d="M-17 2-13-3-9 3M11 4 15-2 19 3" stroke="#7f9473" strokeWidth="2" fill="none" />}
  </g>;
}

function Tree({ x, y, small = false, restored = true }: { x: number; y: number; small?: boolean; restored?: boolean }) {
  return <g transform={`translate(${x} ${y}) scale(${small ? ".7" : "1"})`}>
    <ellipse cy="4" rx="19" ry="6" fill="#34574422" />
    <path d="M0 3V-35M0-18-9-27M0-24 10-34" stroke="#64755a" strokeWidth="4" fill="none" strokeLinecap="round" />
    {restored && <>
      <path d="M-20-26Q-27-41-12-47Q-12-62 2-62Q19-64 21-47Q35-42 27-27Q17-15 1-22Q-13-14-20-26" fill="#55765b" />
      <path d="M-16-39Q-20-51-6-51Q1-67 13-52Q25-50 21-39Q12-33 5-39Q-4-31-16-39" fill="#7d936a" />
      <circle cx="12" cy="-31" r="3" fill="#d8b77a" /><circle cx="-9" cy="-40" r="3" fill="#d8b77a" />
    </>}
  </g>;
}

function Lamp({ x, y, restored }: { x: number; y: number; restored: boolean }) {
  return <g transform={`translate(${x} ${y})`}>
    <ellipse cy="2" rx="9" ry="3" fill="#435c4826" />
    <path d={restored ? "M0 0V-34" : "M0 0V-13L5-19"} stroke="#52675b" strokeWidth="3" fill="none" />
    {restored && <>
      <circle cy="-34" r="14" fill="#ffe0a1" opacity=".17" />
      <path d="M-5-39H5L4-29H-4Z" fill="#ffe1a1" stroke="#647461" strokeWidth="2" />
      <path d="M-7-40 0-45 7-40Z" fill="#566c61" />
      <path d="M-5 1H5" stroke="#c1b394" strokeWidth="3" />
    </>}
  </g>;
}

const pathUnits = [
  ["river-approach", "Lower river approach", "M137 542 163 525"],
  ["approach-turn", "The bend in the approach", "M169 521 194 504"],
  ["bridge-approach", "Bridge approach stonework", "M200 500 225 479"],
  ["bridge-landing", "South bridge landing", "M230 474 260 448"],
  ["north-landing", "North bridge landing", "M375 365 407 351"],
  ["village-path", "Village path", "M414 349 447 344"],
  ["commons-path", "Path to the commons", "M454 343 490 340"],
  ["beacon-path", "Beacon approach", "M497 339 535 337"],
] as const;

const units: Unit[] = pathUnits.map(([id, label, d]) => ({ id, label, district: "The approach", layer: 10, draw: restored => <StonePath d={d} restored={restored} /> }));

units.push(
  { id: "bridge-piers", label: "River bridge piers", district: "The crossing", layer: 11, draw: restored => restored ? <g fill="#b8aa8b" stroke="#8c8a74" strokeWidth="2"><path d="M291 423V456L310 464V415Z" /><path d="M338 390V425L354 431V383Z" /><path d="M283 452 307 460 316 454 292 446Z" fill="#ddd0ac" /></g> : <path d="M291 438V457L310 464V433M338 410V425L354 431V405" fill="#9da08a" /> },
  { id: "bridge-arch", label: "Stone bridge arch", district: "The crossing", layer: 12, draw: restored => restored ? <path d="M265 434 374 357V377L354 391Q312 392 292 438L271 450Z" fill="#b6a687" stroke="#8b8871" strokeWidth="2" /> : <path d="M265 434 279 425 282 443 271 450M356 370 374 357V377L360 386" fill="#989d85" /> },
  { id: "bridge-deck", label: "Bridge walkway", district: "The crossing", layer: 13, draw: restored => restored ? <><path d="M253 429 366 349 389 364 275 445Z" fill="#e6d3ab" stroke="#b6a27e" strokeWidth="2" /><path d="M267 426 278 434M285 412 297 421M304 399 315 408M323 385 334 395M342 372 353 381M361 359 372 368" stroke="#bba988" strokeWidth="2" /></> : <path d="M253 429 276 413 297 427 275 445M347 362 366 349 389 364 370 378" fill="#aca78a" /> },
  { id: "bridge-parapets", label: "Bridge parapets", district: "The crossing", layer: 14, draw: restored => restored ? <g stroke="#e8d9b5" strokeWidth="5" strokeLinecap="round"><path d="M254 427 366 347M276 442 388 362" /><path d="M254 427V417M281 407V397M309 388V378M337 368V358M366 347V337M276 442V432M304 422V412M332 403V393M360 383V373M388 362V352" /></g> : <g stroke="#969c81" strokeWidth="4"><path d="M254 427V417M366 347V337M276 442V432M388 362V352" /></g> },
);

const houses = [
  ["hearth-cottage", "Hearth cottage", 427, 290],
  ["keepers-house", "Keeper’s house", 665, 292],
  ["hill-workshop", "Hill workshop", 571, 242],
  ["riverside-store", "Riverside store", 447, 389],
  ["orchard-house", "Orchard house", 799, 305],
  ["assembly-house", "Assembly house", 717, 392],
] as const;
for (const [id, name, x, y] of houses) {
  units.push(
    { id: `${id}-walls`, label: `${name}: stone walls`, district: "The settlement", layer: y, draw: restored => <HouseWalls x={x} y={y} restored={restored} /> },
    { id: `${id}-roof`, label: `${name}: tiled roof`, district: "The settlement", layer: y + 1, draw: restored => <HouseRoof x={x} y={y} restored={restored} teal={id === "hill-workshop" || id === "assembly-house"} /> },
  );
}

const gardens = [
  ["upper-terrace", "Upper garden terrace", 745, 232],
  ["middle-terrace", "Middle garden terrace", 776, 250],
  ["lower-terrace", "Lower garden terrace", 812, 268],
  ["herb-garden", "Riverside herb garden", 507, 400],
] as const;
for (const [id, label, x, y] of gardens) {
  units.push({ id, label, district: "The terraces", layer: y + 2, draw: restored => <Garden x={x} y={y} restored={restored} flowers={id === "herb-garden"} /> });
}
units.push(
  { id: "terrace-stair", label: "Terrace stair", district: "The terraces", layer: 270, draw: restored => restored ? <g stroke="#b7aa88" strokeWidth="2"><path d="M697 258 717 250 739 273 719 282Z" fill="#dfd2ae" /><path d="M703 264 723 255M709 270 729 261M715 276 735 267" /></g> : <path d="M697 258 717 250 739 273 719 282Z" fill="#98a880" /> },
  { id: "orchard-west", label: "West orchard tree", district: "The terraces", layer: 239, draw: restored => <Tree x={696} y={226} restored={restored} /> },
  { id: "orchard-east", label: "East orchard tree", district: "The terraces", layer: 245, draw: restored => <Tree x={835} y={234} restored={restored} /> },
  { id: "commons-flowers", label: "Commons flower bed", district: "The terraces", layer: 360, draw: restored => <Garden x={642} y={360} restored={restored} flowers /> },
  { id: "river-wall", label: "Riverbank retaining wall", district: "The riverfront", layer: 15, draw: restored => restored ? <><path d="M489 424Q588 424 663 443L661 458Q565 434 487 438Z" fill="#c1b18e" /><path d="M489 424Q588 424 663 443" fill="none" stroke="#eadbb7" strokeWidth="6" /><path d="M511 426V439M543 427V441M575 430V445M607 434V448M640 439V453" stroke="#a6997c" strokeWidth="2" /></> : <path d="M489 433 509 428 528 437M566 439 587 434 606 447M636 450 652 445 662 455" fill="none" stroke="#a3a489" strokeWidth="7" /> },
  { id: "landing-steps", label: "Waterfront steps", district: "The riverfront", layer: 421, draw: restored => restored ? <g stroke="#a9997b" strokeWidth="2"><path d="M548 416 576 417 590 441 558 436Z" fill="#dccba6" /><path d="M552 422 581 424M555 429 585 432" /></g> : <path d="M548 416 576 417 590 441 558 436Z" fill="#a4a78a" /> },
  { id: "dock-piles", label: "Landing jetty piles", district: "The riverfront", layer: 430, draw: restored => <g stroke={restored ? "#785f48" : "#87937c"} strokeWidth={restored ? 7 : 4} strokeLinecap="round"><path d={restored ? "M599 437V456M622 442V462M640 458V477M662 463V483" : "M599 450V456M622 455V462M640 473V477M662 478V483"} /></g> },
  { id: "dock-deck", label: "Landing jetty deck", district: "The riverfront", layer: 440, draw: restored => restored ? <><path d="M590 434 614 429 670 462 645 469Z" fill="#b58e61" stroke="#8c7454" strokeWidth="2" /><path d="M600 440 624 435M611 447 635 442M622 453 647 449M634 460 658 455" stroke="#e0b784" strokeWidth="3" /></> : <path d="M590 434 614 429 627 437 603 443Z" fill="#8d967c" /> },
  { id: "river-boat", label: "River skiff", district: "The riverfront", layer: 451, draw: restored => restored ? <g transform="translate(706 483) rotate(22)"><ellipse cy="6" rx="33" ry="9" fill="#2c615b30" /><path d="M-35-4Q0-23 35-4Q0 21-35-4" fill="#b78450" stroke="#775c42" strokeWidth="2" /><path d="M-23-4Q0-14 24-4Q0 10-23-4" fill="#665d42" /><path d="M-8-10V3M9-9V3" stroke="#d1a673" strokeWidth="4" /></g> : <path d="M675 470 694 476 707 480" fill="none" stroke="#8a9a82" strokeWidth="4" /> },
  { id: "river-mooring", label: "Mooring posts and rope", district: "The riverfront", layer: 452, draw: restored => restored ? <g fill="none" stroke="#78684c" strokeWidth="2"><path d="M658 459Q665 479 678 474" /><path d="M657 455V464" strokeWidth="5" /><path d="M649 452 657 449 665 453" /></g> : null },
  { id: "water-channel", label: "Garden water channel", district: "The riverfront", layer: 16, draw: restored => restored ? <><path d="M821 330Q839 371 814 400L799 419" fill="none" stroke="#c4b78f" strokeWidth="13" /><path d="M821 330Q839 371 814 400L799 419" fill="none" stroke="#7ba79a" strokeWidth="7" /><path d="M817 354H834M815 389 829 393" stroke="#dbcba3" strokeWidth="3" /></> : <path d="M821 330Q839 371 814 400L799 419" fill="none" stroke="#8f9f79" strokeWidth="8" strokeDasharray="13 6" /> },
  { id: "riverside-reeds", label: "Replanted river reeds", district: "The riverfront", layer: 455, draw: restored => restored ? <g fill="none" stroke="#658463" strokeWidth="3" strokeLinecap="round"><path d="M771 464V447M771 460 764 452M781 466V441M790 468V449M790 460 798 451M807 476V454M818 481V460" /><path d="M781 445V439M807 458V451" stroke="#a38a59" strokeWidth="5" /></g> : null },
  { id: "commons-paving", label: "Commons stone circle", district: "Civic life", layer: 18, draw: restored => restored ? <><ellipse cx="579" cy="337" rx="61" ry="27" fill="#d7c7a3" stroke="#ad9f7e" strokeWidth="2" /><ellipse cx="579" cy="337" rx="49" ry="21" fill="none" stroke="#eee0bc" strokeWidth="3" /><path d="M520 337H637M579 311V364M543 316 615 356M544 357 615 318" stroke="#bcae8b" strokeWidth="1.5" /></> : <ellipse cx="579" cy="337" rx="54" ry="24" fill="none" stroke="#98a27d" strokeWidth="4" strokeDasharray="20 8" /> },
  { id: "well-stonework", label: "Village well stonework", district: "Civic life", layer: 319, draw: restored => restored ? <g transform="translate(660 324)"><path d="M-15-6V5Q0 16 15 5V-6" fill="#beaf8d" stroke="#8f9277" strokeWidth="2" /><ellipse cy="-6" rx="16" ry="7" fill="#e2d4ad" /><ellipse cy="-6" rx="10" ry="4" fill="#5b7770" /></g> : <path d="M645 326 648 320 655 323 663 318 675 326" fill="none" stroke="#98a086" strokeWidth="6" /> },
  { id: "well-canopy", label: "Village well canopy", district: "Civic life", layer: 320, draw: restored => restored ? <g transform="translate(660 324)"><path d="M-13-6V-30M13-6V-30" stroke="#81704e" strokeWidth="3" /><path d="M-21-29 0-43 21-29Z" fill="#58776b" stroke="#476658" strokeWidth="2" /><path d="M0-28V-10" stroke="#9a885c" strokeWidth="2" /></g> : null },
  { id: "market-stall", label: "Commons market awning", district: "Civic life", layer: 346, draw: restored => restored ? <g transform="translate(758 345)"><path d="M-25 0V-29M25 0V-29" stroke="#7d7051" strokeWidth="3" /><path d="M-30-24-19-43H21L31-24Z" fill="#ca9b68" /><path d="M-15-24-9-43H1L0-24M12-24 10-43H20L26-24" fill="#f0dfb6" /><path d="M-25-8H25V2H-25Z" fill="#b19366" /><path d="M-22-10H22" stroke="#dac29a" strokeWidth="4" /></g> : <path d="M733 345V330M783 345V325M737 345 758 339 777 345" fill="none" stroke="#8b977b" strokeWidth="3" /> },
  { id: "commons-bench", label: "A bench by the beacon", district: "Civic life", layer: 381, draw: restored => restored ? <g transform="translate(602 379)"><path d="M-17-6V2M16-6V2" stroke="#64745e" strokeWidth="3" /><path d="M-22-10 12-15 24-8-12-3Z" fill="#b08e62" stroke="#8f7956" strokeWidth="1.5" /><path d="M-21-15 12-20" stroke="#c8a97c" strokeWidth="5" /></g> : <path d="M580 377 594 375M607 374 621 371" stroke="#9ca184" strokeWidth="4" /> },
  { id: "waymarker", label: "River path waymarker", district: "Civic life", layer: 386, draw: restored => restored ? <g transform="translate(387 390)"><path d="M0 0V-33" stroke="#6b7257" strokeWidth="4" /><path d="M-14-30H12L18-25 12-20H-14Z" fill="#c8b28a" stroke="#8c8464" strokeWidth="1.5" /><path d="M-7-25H7" stroke="#68745b" strokeWidth="2" /></g> : <path d="M387 390V376" stroke="#929b7e" strokeWidth="4" /> },
  { id: "commons-banner", label: "The commons banner", district: "Civic life", layer: 327, draw: restored => restored ? <g transform="translate(610 324)"><path d="M0 0V-61" stroke="#7c7f5c" strokeWidth="3" /><path d="M2-58Q16-64 31-55V-32Q17-41 2-35Z" fill="#b97453" /><path d="M15-54 21-45 15-38 9-47Z" fill="#f2dbab" /></g> : <path d="M610 324V306" stroke="#96a080" strokeWidth="3" /> },
  { id: "garden-arbor", label: "Orchard arbor", district: "Civic life", layer: 324, draw: restored => restored ? <g transform="translate(861 332)"><path d="M-17 0V-31Q0-50 17-31V0" fill="none" stroke="#b9a681" strokeWidth="5" /><path d="M-21-24Q-25-42-4-42M4-43Q24-40 21-22" fill="none" stroke="#63875f" strokeWidth="8" strokeLinecap="round" /><g fill="#e6bd8f"><circle cx="-15" cy="-37" r="3" /><circle cx="12" cy="-38" r="3" /><circle cx="23" cy="-27" r="3" /></g></g> : <path d="M844 332V316M878 332V311" stroke="#8f9b7c" strokeWidth="4" /> },
);

const lamps = [
  ["approach-lamp", "Approach lantern", 196, 513],
  ["south-bridge-lamp", "South bridge lantern", 269, 428],
  ["north-bridge-lamp", "North bridge lantern", 375, 348],
  ["village-lamp", "Village lantern", 479, 316],
  ["commons-lamp", "Commons lantern", 535, 372],
  ["well-lamp", "Well lantern", 689, 339],
  ["harbor-lamp", "Harbor lantern", 591, 427],
  ["orchard-lamp", "Orchard lantern", 837, 307],
] as const;
for (const [id, label, x, y] of lamps) units.push({ id, label, district: "The lamplight", layer: y + 3, draw: restored => <Lamp x={x} y={y} restored={restored} /> });

units.push(
  { id: "beacon-plinth", label: "Beacon foundation", district: "The beacon", layer: 330, draw: restored => restored ? <g><path d="M549 331 579 316 609 331V343L579 356 549 343Z" fill="#aa9c7c" /><path d="M549 331 579 317 609 331 579 345Z" fill="#ead6ac" /><path d="M555 337 579 348 602 337" fill="none" stroke="#d9c39a" strokeWidth="3" /></g> : <path d="M550 337 579 325 607 338 579 351Z" fill="#a6a68a" /> },
  { id: "beacon-lower-tower", label: "Beacon lower stonework", district: "The beacon", layer: 331, draw: restored => restored ? <g><path d="M558 328V284L579 274 600 284V328L579 339Z" fill="#dbc6a0" stroke="#9a9174" strokeWidth="1.5" /><path d="M579 275 600 284V328L579 339Z" fill="#bbaa86" /><path d="M558 300 579 310 600 300M558 317 579 327 600 317" fill="none" stroke="#ac9e7e" /><path d="M572 334V317Q579 306 586 317V335" fill="#567367" /></g> : <g><path d="M558 328V304L568 307 573 297 582 301 588 292 600 299V328L579 339Z" fill="#a6aa8e" stroke="#859279" strokeWidth="1.5" /><path d="M572 307 568 315 575 321" fill="none" stroke="#7e8d76" /></g> },
  { id: "beacon-upper-tower", label: "Beacon upper stonework", district: "The beacon", layer: 332, draw: restored => restored ? <g><path d="M563 286V250L579 242 595 250V286L579 295Z" fill="#e4d1ab" stroke="#9a9174" strokeWidth="1.5" /><path d="M579 242 595 250V286L579 295Z" fill="#c3b18b" /><path d="M563 263 579 271 595 263M562 286 579 294 596 286" fill="none" stroke="#f0dfb8" strokeWidth="3" /><path d="M571 278V265M586 278V265" stroke="#6b8070" strokeWidth="3" /></g> : <path d="M565 296V275L573 272 580 279 590 274V296" fill="#a2a88b" stroke="#89967b" strokeWidth="2" /> },
  { id: "beacon-gallery", label: "Beacon watch gallery", district: "The beacon", layer: 333, draw: restored => restored ? <g><path d="M553 249 579 236 605 249 579 263Z" fill="#ead6ad" stroke="#b4a17c" strokeWidth="2" /><path d="M553 249V237M566 256V244M579 263V251M592 256V244M605 249V237" stroke="#a08d67" strokeWidth="2" /><path d="M553 237 579 251 605 237" fill="none" stroke="#d9c49b" strokeWidth="3" /></g> : null },
  { id: "beacon-lantern-house", label: "Beacon lantern house", district: "The beacon", layer: 334, draw: restored => restored ? <g><path d="M565 243V218L579 211 593 218V243L579 251Z" fill="#87a294" stroke="#6c7b64" strokeWidth="2" /><path d="M579 211V251M565 218 579 226 593 218" fill="none" stroke="#d7c59c" strokeWidth="2.5" /></g> : null },
  { id: "beacon-crown", label: "Beacon copper crown", district: "The beacon", layer: 335, draw: restored => restored ? <g><path d="M558 218 579 193 600 218 579 228Z" fill="#577e70" stroke="#426459" strokeWidth="1.5" /><path d="M579 193V228L600 218Z" fill="#3e635b" /><path d="M579 191V183" stroke="#b7a174" strokeWidth="3" /><circle cx="579" cy="181" r="3" fill="#d8bc83" /></g> : null },
  { id: "beacon-emblem", label: "Beacon Commonwealth emblem", district: "The beacon", layer: 336, draw: restored => restored ? <g><path d="M577 277 584 283 577 291 570 283Z" fill="#bd9c5f" stroke="#ead19a" strokeWidth="1.5" /><path d="M577 280V286M574 283H580" stroke="#f4dfad" strokeWidth="1.5" /></g> : null },
  { id: "beacon-light", label: "The beacon light", district: "The beacon", layer: 337, draw: restored => restored ? <g className="horizon-beacon-light"><circle cx="579" cy="235" r="48" fill="#ffe2a1" opacity=".12" /><circle cx="579" cy="235" r="26" fill="#ffe4a6" opacity=".18" /><path d="M570 240V224L577 228V244Z" fill="#ffeab6" /><path d="M582 228 589 224V240L582 244Z" fill="#eac77d" /><path d="M575 233 579 226 583 233 579 242Z" fill="#fff3cb" /></g> : null },
);

/** Stable authored order: one complete entry per valid issued BC; no saved scene state. */
export const RESTORATION_UNITS = Object.freeze(units.map(({ id, label, district }, index) => Object.freeze({ id, label, district, ordinal: index + 1 })));
const paintedUnits = units.map((unit, index) => ({ ...unit, ordinal: index + 1 })).sort((a, b) => a.layer - b.layer || a.ordinal - b.ordinal);

export type HorizonSceneProps = {
  /** Committed Stage 3b issuance including courtesy credits retained after corrections. */
  issuedBC: number;
  /** A smaller view of exactly the same scene and policy; never synthetic progress. */
  preview?: boolean;
};

export function HorizonScene({ issuedBC, preview = false }: HorizonSceneProps) {
  const uid = useId().replace(/:/g, "");
  const restored = getRestoredUnitCount(issuedBC);
  const validBalance = Number.isSafeInteger(issuedBC) && issuedBC >= 0;
  const summary = `${restored} of 64 restoration units complete`;
  const latest = RESTORATION_UNITS[restored - 1];

  return <figure className={`horizon-scene${preview ? " horizon-preview" : ""}`} data-horizon-policy={HORIZON_POLICY} data-restored-units={restored} aria-labelledby={`${uid}-heading`}>
    <figcaption className="horizon-heading">
      <div><p className="horizon-eyebrow">Horizon 1 · The river settlement</p><h3 id={`${uid}-heading`}>The Beacon Commonwealth</h3></div>
      <p className="horizon-count"><strong>{restored}<span aria-hidden="true"> / 64</span></strong><span>restoration units<span className="horizon-sr-only"> of 64 complete</span></span></p>
    </figcaption>
    <svg className="horizon-art" viewBox="0 0 960 580" role="img" aria-labelledby={`${uid}-title ${uid}-description`} preserveAspectRatio="xMidYMid meet">
      <title id={`${uid}-title`}>{summary}</title>
      <desc id={`${uid}-description`}>A quiet settlement beside a winding river, with stone paths, a bridge, six houses, garden terraces, a landing jetty and a central beacon. {restored === 0 ? "The settlement awaits restoration; its paths, roofs and beacon are incomplete." : `Restored through ${latest.label.toLowerCase()}.`} {restored === 64 ? "The beacon is lit and the fixed settlement is fully restored." : "Unrestored structures retain broken outlines; completed structures have complete stonework, roofs, planting or light."}</desc>
      <defs>
        <linearGradient id={`${uid}-sky`} x2="0" y2="1"><stop stopColor="#f5dfbb" /><stop offset="1" stopColor="#e7edda" /></linearGradient>
        <linearGradient id={`${uid}-river`} x1="0" y1="0" x2=".8" y2="1"><stop stopColor="#9cbbab" /><stop offset=".55" stopColor="#679b91" /><stop offset="1" stopColor="#477d77" /></linearGradient>
        <linearGradient id={`${uid}-land`} x2=".25" y2="1"><stop stopColor="#bec8a0" /><stop offset="1" stopColor="#8fa780" /></linearGradient>
        <linearGradient id={`${uid}-foreground`} x2=".8" y2="1"><stop stopColor="#adc094" /><stop offset="1" stopColor="#6f8f70" /></linearGradient>
      </defs>
      <g aria-hidden="true">
        <rect width="960" height="580" fill={`url(#${uid}-sky)`} />
        <circle cx="725" cy="91" r="35" fill="#fff1cd" opacity=".74" />
        <path d="M0 190 74 126 125 150 221 74 334 162 412 110 498 179 597 105 688 153 805 88 960 174V300H0Z" fill="#b4bfa4" opacity=".42" />
        <path d="M0 203Q94 155 181 193T346 187Q414 139 497 181T667 171Q779 135 960 187V302H0Z" fill="#a5b696" />
        <path d="M0 239Q142 201 253 212T459 194Q682 159 960 219V580H0Z" fill={`url(#${uid}-land)`} />
        <path d="M352 254Q568 173 831 221M399 281Q661 217 902 262M665 311Q789 285 943 321" fill="none" stroke="#d7d9ad" strokeWidth="3" opacity=".55" />
        <path d="M185 190C104 254 109 310 252 380S643 429 1013 597" fill="none" stroke="#c9c9a2" strokeWidth="149" />
        <path d="M185 190C104 254 109 310 252 380S643 429 1013 597" fill="none" stroke={`url(#${uid}-river)`} strokeWidth="133" />
        <path d="M165 202C101 262 130 319 262 376S642 435 1001 592" fill="none" stroke="#c2d4ba" strokeWidth="2" opacity=".47" />
        <path d="M132 239 150 236M126 257 151 258M148 294 177 302M179 324 216 338M252 377 294 393M326 416 364 422M411 422 441 426M486 447 532 452M560 458 600 465M716 506 755 520M834 521 864 533M868 555 912 575" stroke="#c6d8bc" strokeWidth="2" strokeLinecap="round" opacity=".58" />
        <path d="M0 303Q87 364 180 413T421 492Q529 537 576 580H0Z" fill={`url(#${uid}-foreground)`} />
        <path d="M0 380Q89 413 133 458M25 532Q86 492 130 495M319 522Q386 536 410 570" fill="none" stroke="#c9d0a0" strokeWidth="3" opacity=".5" />
        <path d="M118 554 269 441M379 358 537 337" fill="none" stroke="#b5af8a" strokeWidth="33" strokeLinecap="round" opacity=".7" />
        <path d="M427 290 449 326M570 242 552 293M665 292 636 324M703 373 640 350M799 305 773 330" fill="none" stroke="#beb693" strokeWidth="17" strokeLinecap="round" opacity=".7" />
        <g fill="#6e8968" opacity=".52"><path d="M319 211 328 187 337 211ZM342 206 353 179 364 206ZM368 202 377 183 386 202ZM890 221 900 194 910 221ZM916 224 928 191 940 224Z" /></g>
        <Tree x={331} y={270} /><Tree x={306} y={285} small /><Tree x={888} y={282} /><Tree x={914} y={301} small />
        <g fill="#a5ac89" stroke="#8d9b7d"><path d="M358 307 371 299 386 305 380 312H362Z" /><path d="M873 392 885 386 897 395 883 399Z" /><path d="M349 470 365 465 379 473 360 477Z" /></g>
      </g>
      {paintedUnits.map(unit => <g key={unit.id} data-unit-id={unit.id} data-unit-order={unit.ordinal} data-restored={unit.ordinal <= restored ? "true" : "false"} className={unit.ordinal <= restored ? "horizon-unit is-restored" : "horizon-unit is-unrestored"} aria-hidden="true">{unit.draw(unit.ordinal <= restored)}</g>)}
      <g aria-hidden="true">
        <Tree x={59} y={467} /><Tree x={90} y={480} small /><Tree x={39} y={497} />
        <Tree x={359} y={569} /><Tree x={398} y={579} small />
        <g fill="#d5d6ad"><path d="M87 545 98 538 109 545 101 551Z" /><path d="M454 564 465 558 477 567 463 571Z" /></g>
        <g stroke="#74906a" strokeWidth="2" fill="none"><path d="M222 560V550M218 555 222 560 228 550M317 516V505M311 511 317 516 322 507M882 365V355M877 360 882 365 886 357" /></g>
        <path d="M385 118Q390 114 395 118Q400 114 405 118M414 110Q418 107 422 110Q426 107 430 110" fill="none" stroke="#8c9a84" strokeWidth="2" strokeLinecap="round" />
      </g>
    </svg>
    <div className="horizon-description">
      <p>{restored === 0 ? "A quiet place to begin. The river settlement is waiting to be restored." : restored === 64 ? "The beacon is lit. This river settlement is fully restored." : <>Restored through <strong>{latest.label.toLowerCase()}</strong>.</>}</p>
      <p className="horizon-policy-note">One issued Build Credit restores one unit. Your construction stays when you step away.</p>
      {validBalance && issuedBC > HORIZON_UNIT_COUNT && <p className="horizon-overflow">All {issuedBC} issued Build Credits remain in your ledger. This fixed scene stays at 64 units.</p>}
      {!validBalance && <p role="alert">Construction is waiting for a valid committed Build Credit total.</p>}
      {!preview && <details className="horizon-restoration-list"><summary>Restoration record · {summary}</summary><ol>{RESTORATION_UNITS.map(unit => <li key={unit.id}><span>{unit.label}</span><span className="horizon-unit-status">{unit.ordinal <= restored ? "Complete" : "Unrestored"}</span></li>)}</ol></details>}
    </div>
  </figure>;
}

export default HorizonScene;
