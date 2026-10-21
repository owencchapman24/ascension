/** Original, deterministic River Reach silhouettes. Decorative; names live in HTML. */
export function BuildingArt({ kind, x = 0, y = 0, scale = 1 }: { kind: string; x?: number; y?: number; scale?: number }) {
  const stone = "#dfd5b8", dark = "#354f4a", roof = "#a55c44", gold = "#d8ac5e";
  return <g transform={`translate(${x} ${y}) scale(${scale})`} stroke="#3c534a" strokeWidth="2" strokeLinejoin="round">
    <ellipse cx="0" cy="16" rx="47" ry="14" fill="#254a3c" opacity=".17" stroke="none" />
    {kind === "foundry" ? <>
      <path d="M-38 12V-30L-10-43 24-29V12Z" fill={stone} /><path d="M-43-29-12-49 30-30 1-17Z" fill={roof} />
      <path d="M-22-40V-76H-8V-44M10-34V-62H23V-28" fill="#796e5c" /><path d="M-28 12V-8Q-15-23-2-8V12Z" fill={dark} />
      <path d="M-23 10-17-3-11 4-7-4-3 10Z" fill="#e6aa52" stroke="none" /><path d="M6-10H19V1H6Z" fill={gold} />
    </> : kind === "grand_archive" ? <>
      <path d="M-42 12V-33H42V12Z" fill={stone} /><path d="M-48-32 0-62 48-32Z" fill={roof} />
      <path d="M-35-27H35M-34 10H35" stroke="#9b9877" strokeWidth="5" />
      {[-28, -14, 14, 28].map(v => <path key={v} d={`M${v}-25V9`} strokeWidth="7" stroke="#f1e7ce" />)}
      <path d="M-6 11V-17Q0-25 6-17V11" fill={dark} /><path d="M-8-42H8M0-50V-35" stroke={gold} />
    </> : kind === "embassy" ? <>
      <path d="M-36 12V-29H36V12Z" fill={stone} /><path d="M-42-28 0-51 42-28Z" fill="#5d7c78" />
      <path d="M-13 12V-19H13V12Z" fill={dark} /><path d="M-29-19V0M29-19V0" stroke={gold} strokeWidth="8" />
      <path d="M0-52V-82" /><path d="M1-81 28-72 1-64Z" fill="#b86a52" /><path d="M-42 16H42" strokeWidth="7" stroke={stone} />
    </> : kind === "survey_lodge" ? <>
      <path d="M-34 13V-22L0-39 33-22V13Z" fill="#c3b08a" /><path d="M-41-22-4-50 40-24 5-9Z" fill={roof} />
      <path d="M-25 12V-10H-8V12M8-14H23V0H8Z" fill={dark} /><path d="M39 12 45-26 53 12M43-13H51" fill="none" />
      <path d="M-29-3H-14M-22-10V11" stroke={gold} />
    </> : kind === "observatory" ? <>
      <path d="M-29 13V-28H29V13Z" fill={stone} /><path d="M-35-28A35 35 0 0 1 35-28Z" fill="#638c8d" />
      <path d="M3-63 28-80 35-70 9-52Z" fill={gold} /><path d="M-6 12V-10Q0-18 6-10V12Z" fill={dark} />
      <path d="M-24-20H-16V-7H-24ZM16-20H24V-7H16Z" fill={gold} /><path d="M-34 15H34" strokeWidth="5" />
    </> : <>
      <path d="M-37 12V-23H34V12Z" fill={stone} /><path d="M-44-24-28-47H31L44-24Z" fill={roof} />
      <path d="M-34-17H34L39-4H-39Z" fill="#e0b464" /><path d="M-23 12V-3H-9V12M9 12V-3H23V12" fill={dark} />
      <circle cx="44" cy="9" r="8" fill="#83694c" /><path d="M40-1H55V9H40Z" fill="#bfa47a" /><path d="M-24-46V-66M-23-66-5-60-23-55" fill="#648377" />
    </>}
  </g>;
}

export function BuildingIcon({ kind }: { kind: string }) {
  return <svg className="rr-building-icon" viewBox="-65 -88 130 115" aria-hidden="true"><BuildingArt kind={kind} /></svg>;
}
