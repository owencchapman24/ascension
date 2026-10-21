export type BuildingType = "foundry" | "grand_archive" | "embassy" | "survey_lodge" | "observatory" | "caravan_house";
export type Site = "a1" | "a2" | "a3" | "b1" | "b2" | "b3";
export type Route = "stone_causeway" | "river_compact" | "beacon_mesh";
export interface Capabilities { engineering: number; knowledge: number; diplomacy: number }
export interface BuildingDefinition { building_type: BuildingType; name: string; footprint: number; cost_bc: number; capabilities: Capabilities; copy_limit: number; description: string }
export interface RouteDefinition { route: Route; name: string; requirements: Capabilities; description: string; ending: string }
export interface PlacementInput { id: string | null; building_type: BuildingType; parcels: Site[] }
export interface BuildingPlacement extends PlacementInput { id: string }
export interface Blueprint { id: string; building_type: BuildingType; parcels: Site[]; reserved_bc: number; created_at: string }
export interface ChapterEnding { id: string; chapter_id: string; content_version: string; route: Route; layout: BuildingPlacement[]; capabilities: Capabilities; funded_site_count: number; spent_bc: number; completed_at: string; command_id: string; narrative: string }
export interface WorldChapter { id: string; content_version: string; revision: number; created_at: string; funded_site_count: number; layout: BuildingPlacement[]; queue: Blueprint | null; selected_route: Route | null; previous_layout: BuildingPlacement[] | null; completion: ChapterEnding | null; sandbox_alternatives: number }
export interface FundingAllocation { issuance_entry_id: string; amount_bc: number }
export interface WorldSpend { id: string; chapter_id: string; content_version: string; sequence: number; amount_bc: number; funded_sites_added: number; ledger_sequence: number; allocations: FundingAllocation[]; created_at: string; command_id: string }
export interface WorldCommandResult { command_id: string; chapter_id: string; world_revision: number; spent_bc: number; reserved_bc: number; available_bc: number; funded_site_count: number; blueprint_id: string | null; building_id: string | null; spend_id: string | null; ending_id: string | null }
export interface CommittedWorldCommand { id: string; sequence: number; payload_digest: string; payload: WorldCommandInput; ledger_sequence: number; issued_bc: number; created_at: string; result: WorldCommandResult }
export interface WorldState { chapter: WorldChapter | null; spending: WorldSpend[]; commands: CommittedWorldCommand[] }
export interface WorldView { content_version: "river-reach-v1"; chapter: WorldChapter | null; bank: { issued_bc: number; spent_bc: number; reserved_bc: number; available_bc: number }; capabilities: Capabilities; parcels: { id: Site; row: number; column: number; label: string }[]; buildings: BuildingDefinition[]; routes: RouteDefinition[]; retained_credit_bc: number }
export interface WorldPreview { legal: boolean; placement_legal: boolean; errors: string[]; occupied_sites: number; funded_site_count: number; additional_sites_needed: number; capabilities: Capabilities; routes: { route: Route; satisfied: boolean; missing: Capabilities }[] }
export interface WorldPreviewInput { chapter_id: string; content_version: "river-reach-v1"; placements: PlacementInput[] }
export type WorldOperation = { kind: "initialize" } | { kind: "select_route"; route: Route } | { kind: "queue" | "replace_queue"; building_type: BuildingType; parcels: Site[]; reserve_available: boolean } | { kind: "reserve" } | { kind: "cancel_queue" } | { kind: "complete_construction" } | { kind: "reconfigure"; placements: PlacementInput[] } | { kind: "undo" } | { kind: "complete_chapter" };
export interface WorldCommandInput { command_id: string; schema_version: 1; content_version: "river-reach-v1"; expected_world_revision: number; chapter_id: string | null; operation: WorldOperation }
