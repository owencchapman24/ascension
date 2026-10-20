import type { StudyView } from "../platform/client";

/** Empty renderer fixture only. Durable initialization remains Rust-owned. */
export const emptyStudy: StudyView = {
  epochs: [], manifests: [], protocol_events: [], opportunities: [], assignments: [], outcomes: [], exposures: [], burdens: [], reflections: [], preferences: [],
  presentation: { opportunity_id: null, condition: null, boundary: "none", world_allowed: true, enabled: true },
};
