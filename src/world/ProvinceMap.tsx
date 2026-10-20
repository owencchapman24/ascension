import { useId } from "react";
import { BuildingArt } from "./BuildingArt";

export const PARCELS = ["a1", "a2", "a3", "b1", "b2", "b3"] as const;
const position = (site: string) => { const index = PARCELS.indexOf(site as typeof PARCELS[number]); return { x: 310 + (index % 3) * 177, y: 255 + Math.floor(index / 3) * 155 }; };
export interface MapPlacement { id?: string | null; building_type: string; parcels: string[] }
interface Props { placements: MapPlacement[]; selected?: string[]; queued?: string[]; fundedSites: number; onSelect?: (parcel: string) => void; names: Record<string, string>; draft?: boolean; completed?: boolean; endingRoute?: string }

export function ProvinceMap({ placements, selected = [], queued = [], fundedSites, onSelect, names, draft = false, completed = false, endingRoute }: Props) {
  const uid = useId().replace(/:/g, "");
  return <div className={`rr-map ${draft ? "rr-map-draft" : ""}`}>
    <svg viewBox="0 0 900 570" role="img" aria-label={`River Reach province. ${placements.length} buildings across six parcels. ${fundedSites} funded sites. ${draft ? "Uncommitted layout preview." : "Saved province."}`}>
      <defs>
        <linearGradient id={`${uid}-land`} x2="0" y2="1"><stop stopColor="#d3d5ac" /><stop offset="1" stopColor="#b1c29c" /></linearGradient>
        <linearGradient id={`${uid}-water`} x1="0" y1="0" x2="1" y2="1"><stop stopColor="#7da9a5" /><stop offset="1" stopColor="#3e7d83" /></linearGradient>
        <pattern id={`${uid}-grain`} width="39" height="37" patternUnits="userSpaceOnUse"><path d="m8 12 3-5m18 20 3-4" stroke="#f4ecd0" opacity=".3" /></pattern>
      </defs>
      <rect width="900" height="570" rx="16" fill={`url(#${uid}-land)`} />
      <path d="M0 0H900V80Q735 33 664 67T458 61 226 86 0 56Z" fill="#789980" opacity=".4" />
      <path d="M0 163Q111 185 126 300T253 461Q297 488 292 570H0Z" fill="#e2d7af" />
      <path d="M0 187Q91 213 104 315T227 478Q263 506 254 570H0Z" fill={`url(#${uid}-water)`} />
      <path d="M1 255Q63 270 65 334T167 471M5 390Q39 438 102 461M129 487Q211 508 204 570" fill="none" stroke="#bad7cb" strokeWidth="3" opacity=".55" />
      <path d="M199 89Q435 75 813 102L827 469Q571 487 253 466L181 332Z" fill="#ced0a7" stroke="#9da884" strokeWidth="3" />
      <path d="M208 332H800M398 130V464M576 130V464" stroke="#e8d9ad" strokeWidth="17" fill="none" />
      <path d="M68 352 184 321 231 334 114 371Z" fill="#9d8866" stroke="#6d725a" strokeWidth="3" /><path d="M81 345 195 318M103 371 218 341" stroke="#e9dfbc" strokeWidth="5" />
      <rect width="900" height="570" fill={`url(#${uid}-grain)`} rx="16" />
      {PARCELS.map(site => { const p = position(site), occupied = placements.some(b => b.parcels.includes(site)); return <g key={site}><rect x={p.x - 73} y={p.y - 91} width="146" height="118" rx="12" fill={selected.includes(site) ? "#f5dfa0" : occupied ? "#c5b98d" : "#dfe0ba"} stroke={selected.includes(site) ? "#9b6b32" : "#8d9d79"} strokeWidth={selected.includes(site) ? 4 : 2} strokeDasharray={queued.includes(site) ? "6 4" : undefined} />{!occupied && <path d={`M${p.x - 17} ${p.y - 25}h34m-17-17v34`} stroke="#849b78" opacity=".5" strokeWidth="2" />}</g>; })}
      {placements.map((b, i) => { const points = b.parcels.map(position); const x = points.reduce((n, p) => n + p.x, 0) / points.length; const y = points.reduce((n, p) => n + p.y, 0) / points.length; return <g key={b.id ?? `${b.building_type}-${i}`}>{points.length === 2 && <path d={`M${points[0].x} ${points[0].y + 9}L${points[1].x} ${points[1].y + 9}`} stroke="#7c7a5b" strokeWidth="30" opacity=".35" />}<BuildingArt kind={b.building_type} x={x} y={y - 3} scale={b.parcels.length === 2 ? 1.15 : .98} /></g>; })}
      <g transform="translate(126 125)" stroke="#45604a" strokeWidth="2"><path d="M-23 18V-30H19V18Z" fill="#b9af8b" /><path d="M-29-29-3-57 25-29Z" fill="#697b61" /><path d="M-10-21H7V-7H-10Z" fill={completed ? "#f1c36c" : "#405c50"} />{completed && <circle cx="-2" cy="-14" r="34" fill="#f7d27b" opacity=".18" stroke="none" />}</g>
      <g transform="translate(149 451)"><path d="M-29 17 0-20 29 17Z" fill="#d9b784" stroke="#897657" strokeWidth="2" /><path d="M0-20V17L15 17Z" fill="#9e7658" /><path d="M36 23 45 12 54 23Z" fill="#b57a4c" /><path d="M40 24H51" stroke="#6d614a" strokeWidth="4" /></g>
      {endingRoute === "stone_causeway" && <g aria-label="Stone causeway arches" transform="translate(63 356) rotate(-15)"><path d="M0 0H164V31H149V20A16 16 0 0 0 117 20V31H100V20A16 16 0 0 0 68 20V31H51V20A16 16 0 0 0 19 20V31H0Z" fill="#ddd3ae" stroke="#70715d" strokeWidth="3" /><path d="M-3-5H169M-3-14H169" stroke="#eae1bd" strokeWidth="7" /><path d="M6-15V-3M38-15V-3M72-15V-3M106-15V-3M140-15V-3M164-15V-3" stroke="#77785f" strokeWidth="3" /></g>}
      {endingRoute === "river_compact" && <g aria-label="River assembly pavilion" transform="translate(169 465)" stroke="#51614d" strokeWidth="2"><ellipse rx="53" ry="21" fill="#c7b58b" /><path d="M-37 4V-29M37 4V-29M-47-29 0-62 47-29Z" fill="#638577" /><path d="M-30-29 0-62 30-29Z" fill="#d7b66c" /><ellipse cy="1" rx="28" ry="9" fill="#917455" />{[-57, 57].map(x => <g key={x}><path d={`M${x} 6V-65`} /><path d={`M${x} -65l22 7-22 10Z`} fill="#ae654e" /></g>)}</g>}
      {endingRoute === "beacon_mesh" && <g aria-label="Linked beacon network"><path d="M124 111 398 522 802 506 842 98" fill="none" stroke="#e9d390" strokeWidth="3" strokeDasharray="4 8" opacity=".8" />{[[398,522],[802,506],[842,98]].map(([x,y],i) => <g key={i} transform={`translate(${x} ${y})`}><path d="M-7 0-4-26H4L7 0Z" fill="#91866a" stroke="#5a6c50" strokeWidth="2" /><circle cy="-29" r="20" fill="#f3d77a" opacity=".25" /><circle cy="-29" r="6" fill="#f9d979" /><path d="M-8-39H8L0-46Z" fill="#667e5a" /></g>)}</g>}
      {[[42,84],[52,116],[825,79],[853,116],[842,486],[802,506],[398,522],[464,511],[702,527]].map(([x,y], i) => <g key={i} transform={`translate(${x} ${y})`}><path d="M0 20V-10" stroke="#667354" strokeWidth="4" /><path d="M-20 4 0-35 20 4-12-1 0-21 12-1Z" fill={i % 2 ? "#608469" : "#73916c"} /></g>)}
      <text x="292" y="114" fill="#526951" fontSize="13" letterSpacing="4">RIVER TERRACE</text><text x="51" y="529" fill="#e2ebd4" fontSize="13" letterSpacing="3" transform="rotate(-18 51 529)">THE REACH</text>
      {PARCELS.map(site => { const p = position(site); return <g key={site} pointerEvents="none"><rect x={p.x - 64} y={p.y - 81} width="38" height="26" rx="6" fill="#fbf6e5" stroke="#647458" strokeWidth="1.5" /><text x={p.x - 45} y={p.y - 62} textAnchor="middle" fill="#294638" fontSize="17" fontWeight="700">{site.toUpperCase()}</text></g>; })}
    </svg>
    {onSelect && <div className="rr-map-controls" aria-label="Choose exact parcels">{PARCELS.map(site => { const p = position(site); const building = placements.find(b => b.parcels.includes(site)); return <button key={site} type="button" className="rr-parcel" style={{ left: `${(p.x - 73) / 9}%`, top: `${(p.y - 91) / 5.7}%`, width: `${146 / 9}%`, height: `${118 / 5.7}%` }} aria-label={`${site}: ${building ? names[building.building_type] ?? building.building_type : "empty"}${queued.includes(site) ? ", queued blueprint" : ""}`} aria-pressed={selected.includes(site)} onClick={() => onSelect(site)}><span className="visually-hidden">Select {site}</span></button>; })}</div>}
    <div className="rr-map-key"><span><i className="rr-key-solid" /> Completed building</span><span><i className="rr-key-dashed" /> Queued blueprint</span><span>{fundedSites} / 6 funded sites · capacity may move freely</span></div>
  </div>;
}
