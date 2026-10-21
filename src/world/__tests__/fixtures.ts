import type { BuildingDefinition, Capabilities, RouteDefinition, WorldPreview, WorldView } from "../types";
import { id, timestamp } from "../../progress/__tests__/fixtures";

const caps = (engineering = 0, knowledge = 0, diplomacy = 0): Capabilities => ({ engineering, knowledge, diplomacy });
export const buildings: BuildingDefinition[] = [
  { building_type: "foundry", name: "Foundry", footprint: 2, cost_bc: 8, capabilities: caps(5), copy_limit: 2, description: "Concentrated engineering for the river crossing." },
  { building_type: "grand_archive", name: "Grand Archive", footprint: 2, cost_bc: 8, capabilities: caps(0, 5), copy_limit: 2, description: "A home for maps and navigation." },
  { building_type: "embassy", name: "Embassy", footprint: 2, cost_bc: 8, capabilities: caps(0, 0, 5), copy_limit: 2, description: "A meeting place for the river settlements." },
  { building_type: "survey_lodge", name: "Survey Lodge", footprint: 1, cost_bc: 4, capabilities: caps(2, 1), copy_limit: 2, description: "A compact base for surveying the terrace." },
  { building_type: "observatory", name: "Observatory", footprint: 1, cost_bc: 4, capabilities: caps(0, 2, 1), copy_limit: 2, description: "The river's navigators chart the stars." },
  { building_type: "caravan_house", name: "Caravan House", footprint: 1, cost_bc: 4, capabilities: caps(1, 0, 2), copy_limit: 2, description: "A welcome for travelers and trade." },
];
export const routes: RouteDefinition[] = [
  { route: "stone_causeway", name: "Stone Causeway", requirements: caps(10, 4), description: "Connect the banks through durable engineering.", ending: "Stone arches join the banks across the Reach." },
  { route: "river_compact", name: "River Compact", requirements: caps(0, 4, 10), description: "Bring the river settlements into an assembly.", ending: "The settlements gather beneath a shared pavilion." },
  { route: "beacon_mesh", name: "Beacon Mesh", requirements: caps(6, 6, 6), description: "Connect the province through complementary capabilities.", ending: "Signals pass along a network of quiet beacons." },
];
export const world: WorldView = {
  content_version: "river-reach-v1", chapter: { id: id(201), content_version: "river-reach-v1", revision: 1, created_at: timestamp, funded_site_count: 0, layout: [], queue: null, selected_route: null, previous_layout: null, completion: null, sandbox_alternatives: 0 },
  bank: { issued_bc: 4, spent_bc: 0, reserved_bc: 0, available_bc: 4 }, capabilities: caps(), retained_credit_bc: 0, buildings, routes,
  parcels: ["a1", "a2", "a3", "b1", "b2", "b3"].map((site, i) => ({ id: site as WorldView["parcels"][number]["id"], row: Math.floor(i / 3), column: i % 3, label: site.toUpperCase() })),
};
export const validPreview: WorldPreview = { legal: true, placement_legal: true, errors: [], occupied_sites: 0, funded_site_count: 0, additional_sites_needed: 0, capabilities: caps(), routes: routes.map(item => ({ route: item.route, satisfied: false, missing: item.requirements })) };
export const commandResult = { command_id: id(230), chapter_id: id(201), world_revision: 2, spent_bc: 0, reserved_bc: 0, available_bc: 4, funded_site_count: 0, blueprint_id: null, building_id: null, spend_id: null, ending_id: null };
export const queuedWorld: WorldView = { ...world, bank: { ...world.bank, reserved_bc: 4, available_bc: 0 }, chapter: { ...world.chapter!, revision: 2, queue: { id: id(202), building_type: "survey_lodge", parcels: ["a1"], reserved_bc: 4, created_at: timestamp } } };
export const fundedWorld: WorldView = { ...world, bank: { issued_bc: 8, spent_bc: 4, reserved_bc: 0, available_bc: 4 }, capabilities: caps(2, 1), chapter: { ...world.chapter!, funded_site_count: 1, revision: 3, layout: [{ id: id(203), building_type: "survey_lodge", parcels: ["a1"] }] } };
export const completeWorld: WorldView = { ...world, bank: { issued_bc: 24, spent_bc: 24, reserved_bc: 0, available_bc: 0 }, capabilities: caps(10, 5), chapter: { ...world.chapter!, revision: 10, funded_site_count: 6, selected_route: "stone_causeway", layout: [{ id: id(204), building_type: "foundry", parcels: ["a1", "b1"] }, { id: id(205), building_type: "foundry", parcels: ["a2", "b2"] }, { id: id(206), building_type: "grand_archive", parcels: ["a3", "b3"] }] } };
