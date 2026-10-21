//! The fixed River Reach chapter. This module spends existing issuance only.
//! Commands are small chapter-specific durable decisions, not a general event engine.
use crate::domain::{CommandError, RecordSet, now, valid_id};
use crate::progress::ProgressState;
use crate::study::{Phase, StudyStatus};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

pub const CONTENT_VERSION: &str = "river-reach-v1";

fn required<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<T, D::Error> {
    T::deserialize(d)
}
macro_rules! model {
    ($name:ident {$($field:ident:$ty:ty),* $(,)?}) => {
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $name {$(#[serde(deserialize_with="required")] pub $field:$ty,)*}
    };
}
macro_rules! enumeration {
    ($name:ident {$($variant:ident),*}) => {
        #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[serde(rename_all="snake_case")]
        pub enum $name {$($variant),*}
    };
}
enumeration!(Site {
    A1,
    A2,
    A3,
    B1,
    B2,
    B3
});
enumeration!(BuildingType {
    Foundry,
    GrandArchive,
    Embassy,
    SurveyLodge,
    Observatory,
    CaravanHouse
});
enumeration!(Route {
    StoneCauseway,
    RiverCompact,
    BeaconMesh
});

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub engineering: u32,
    pub knowledge: u32,
    pub diplomacy: u32,
}
model!(ParcelDefinition {
    id: Site,
    row: u32,
    column: u32,
    label: String
});
model!(BuildingDefinition {
    building_type: BuildingType,
    name: String,
    footprint: u32,
    cost_bc: u32,
    capabilities: Capabilities,
    copy_limit: u32,
    description: String
});
model!(RouteDefinition {
    route: Route,
    name: String,
    requirements: Capabilities,
    description: String,
    ending: String
});
model!(PlacementInput {id:Option<String>,building_type:BuildingType,parcels:Vec<Site>});
model!(BuildingPlacement {id:String,building_type:BuildingType,parcels:Vec<Site>});
model!(Blueprint {id:String,building_type:BuildingType,parcels:Vec<Site>,reserved_bc:u32,created_at:String});
model!(ChapterEnding {id:String,chapter_id:String,content_version:String,route:Route,layout:Vec<BuildingPlacement>,capabilities:Capabilities,funded_site_count:u32,spent_bc:u32,completed_at:String,command_id:String,narrative:String});
model!(WorldChapter {id:String,content_version:String,revision:u32,created_at:String,funded_site_count:u32,layout:Vec<BuildingPlacement>,queue:Option<Blueprint>,selected_route:Option<Route>,previous_layout:Option<Vec<BuildingPlacement>>,completion:Option<ChapterEnding>,sandbox_alternatives:u32});
model!(FundingAllocation {
    issuance_entry_id: String,
    amount_bc: u32
});
model!(WorldSpend {id:String,chapter_id:String,content_version:String,sequence:u32,amount_bc:u32,funded_sites_added:u32,ledger_sequence:u32,allocations:Vec<FundingAllocation>,created_at:String,command_id:String});

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorldOperation {
    Initialize,
    SelectRoute {
        route: Route,
    },
    Queue {
        building_type: BuildingType,
        parcels: Vec<Site>,
        reserve_available: bool,
    },
    ReplaceQueue {
        building_type: BuildingType,
        parcels: Vec<Site>,
        reserve_available: bool,
    },
    Reserve,
    CancelQueue,
    CompleteConstruction,
    Reconfigure {
        placements: Vec<PlacementInput>,
    },
    Undo,
    CompleteChapter,
}
model!(WorldCommandInput {command_id:String,schema_version:u32,content_version:String,expected_world_revision:u32,chapter_id:Option<String>,operation:WorldOperation});
model!(WorldCommandResult {command_id:String,chapter_id:String,world_revision:u32,spent_bc:u32,reserved_bc:u32,available_bc:u32,funded_site_count:u32,blueprint_id:Option<String>,building_id:Option<String>,spend_id:Option<String>,ending_id:Option<String>});
model!(CommittedWorldCommand {
    id: String,
    sequence: u32,
    payload_digest: String,
    payload: WorldCommandInput,
    ledger_sequence: u32,
    issued_bc: u32,
    created_at: String,
    result: WorldCommandResult
});

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldState {
    #[serde(deserialize_with = "required")]
    pub chapter: Option<WorldChapter>,
    pub spending: Vec<WorldSpend>,
    pub commands: Vec<CommittedWorldCommand>,
}
impl WorldState {
    pub fn record_count(&self) -> usize {
        usize::from(self.chapter.is_some()) + self.spending.len() + self.commands.len()
    }
    pub fn has_history(&self) -> bool {
        self.chapter.is_some() || !self.spending.is_empty() || !self.commands.is_empty()
    }
    pub fn normalize(&mut self) {
        self.spending.sort_by_key(|s| s.sequence);
        self.commands.sort_by_key(|s| s.sequence);
        if let Some(c) = &mut self.chapter {
            canonical_layout(&mut c.layout);
            if let Some(layout) = &mut c.previous_layout {
                canonical_layout(layout);
            }
            if let Some(q) = &mut c.queue {
                q.parcels.sort();
            }
            if let Some(e) = &mut c.completion {
                canonical_layout(&mut e.layout);
            }
        }
    }
}
model!(BankProjection {
    issued_bc: u32,
    spent_bc: u32,
    reserved_bc: u32,
    available_bc: u32
});
model!(WorldView {content_version:String,chapter:Option<WorldChapter>,bank:BankProjection,capabilities:Capabilities,parcels:Vec<ParcelDefinition>,buildings:Vec<BuildingDefinition>,routes:Vec<RouteDefinition>,retained_credit_bc:u32});
model!(WorldPreviewInput {chapter_id:String,content_version:String,placements:Vec<PlacementInput>});
model!(RouteProgress {
    route: Route,
    satisfied: bool,
    missing: Capabilities
});
model!(WorldPreview {legal:bool,placement_legal:bool,errors:Vec<String>,occupied_sites:u32,funded_site_count:u32,additional_sites_needed:u32,capabilities:Capabilities,routes:Vec<RouteProgress>});

pub fn building_definition(kind: BuildingType) -> BuildingDefinition {
    use BuildingType::*;
    let (name, footprint, e, k, d, description) = match kind {
        Foundry => (
            "Foundry",
            2,
            5,
            0,
            0,
            "River stone and patient craft become the backbone of new crossings.",
        ),
        GrandArchive => (
            "Grand Archive",
            2,
            0,
            5,
            0,
            "Charts, observations, and shared memory give the province its bearings.",
        ),
        Embassy => (
            "Embassy",
            2,
            0,
            0,
            5,
            "A welcoming assembly hall where neighboring settlements find common ground.",
        ),
        SurveyLodge => (
            "Survey Lodge",
            1,
            2,
            1,
            0,
            "Small teams map stable ground and turn measurements into practical plans.",
        ),
        Observatory => (
            "Observatory",
            1,
            0,
            2,
            1,
            "A clear view of the river and sky supports knowledge shared across communities.",
        ),
        CaravanHouse => (
            "Caravan House",
            1,
            1,
            0,
            2,
            "A sheltered meeting place connects travelers, supplies, and local expertise.",
        ),
    };
    BuildingDefinition {
        building_type: kind,
        name: name.into(),
        footprint,
        cost_bc: footprint * 4,
        capabilities: Capabilities {
            engineering: e,
            knowledge: k,
            diplomacy: d,
        },
        copy_limit: 2,
        description: description.into(),
    }
}
pub fn building_definitions() -> Vec<BuildingDefinition> {
    use BuildingType::*;
    [
        Foundry,
        GrandArchive,
        Embassy,
        SurveyLodge,
        Observatory,
        CaravanHouse,
    ]
    .into_iter()
    .map(building_definition)
    .collect()
}
pub fn route_definition(route: Route) -> RouteDefinition {
    let (name, e, k, d, description, ending) = match route {
        Route::StoneCauseway => (
            "Stone Causeway",
            10,
            4,
            0,
            "Raise a lasting crossing through engineering and knowledge.",
            "Stone arches now carry the road above the river. River Reach becomes a meeting of shores, its crossing held in common for those who follow.",
        ),
        Route::RiverCompact => (
            "River Compact",
            0,
            4,
            10,
            "Bring the settlements together through diplomacy and knowledge.",
            "Lanterns gather beneath the assembly roof. The river settlements seal a compact: passage, learning, and a place at the table will be shared.",
        ),
        Route::BeaconMesh => (
            "Beacon Mesh",
            6,
            6,
            6,
            "Connect a distributed network with three complementary capabilities.",
            "One light answers another across the water. Workshops, watchers, and wayhouses form a beacon mesh, each keeping a different part of the river connected.",
        ),
    };
    RouteDefinition {
        route,
        name: name.into(),
        requirements: Capabilities {
            engineering: e,
            knowledge: k,
            diplomacy: d,
        },
        description: description.into(),
        ending: ending.into(),
    }
}
pub fn route_definitions() -> Vec<RouteDefinition> {
    [Route::StoneCauseway, Route::RiverCompact, Route::BeaconMesh]
        .into_iter()
        .map(route_definition)
        .collect()
}
fn coordinates(site: Site) -> (u32, u32) {
    match site {
        Site::A1 => (0, 0),
        Site::A2 => (0, 1),
        Site::A3 => (0, 2),
        Site::B1 => (1, 0),
        Site::B2 => (1, 1),
        Site::B3 => (1, 2),
    }
}
pub fn parcels() -> Vec<ParcelDefinition> {
    [Site::A1, Site::A2, Site::A3, Site::B1, Site::B2, Site::B3]
        .into_iter()
        .map(|id| {
            let (row, column) = coordinates(id);
            ParcelDefinition {
                id,
                row,
                column,
                label: format!("{}{}", if row == 0 { "A" } else { "B" }, column + 1),
            }
        })
        .collect()
}
fn invalid(message: &str) -> CommandError {
    CommandError::invalid(message)
}
fn db<T>(value: rusqlite::Result<T>) -> Result<T, CommandError> {
    value.map_err(|_| CommandError::storage())
}
fn encode<T: Serialize>(value: &T) -> Result<String, CommandError> {
    serde_json::to_string(value).map_err(|_| CommandError::storage())
}
fn hash<T: Serialize>(value: &T) -> Result<String, CommandError> {
    Ok(format!("{:x}", Sha256::digest(encode(value)?.as_bytes())))
}
fn identity(command: &str, purpose: &str) -> String {
    let hash = Sha256::digest(format!("ascension/{CONTENT_VERSION}/{command}/{purpose}"));
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    bytes[6] = (bytes[6] & 15) | 0x40;
    bytes[8] = (bytes[8] & 63) | 0x80;
    uuid::Uuid::from_bytes(bytes).to_string()
}
fn timestamp(value: &str) -> Result<(), CommandError> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| invalid("A world timestamp is invalid."))?;
    if parsed
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true)
        != value
    {
        return Err(invalid(
            "World timestamps require canonical UTC milliseconds.",
        ));
    }
    Ok(())
}
fn canonical_layout(layout: &mut [BuildingPlacement]) {
    for p in layout.iter_mut() {
        p.parcels.sort();
    }
    layout.sort_by(|a, b| a.id.cmp(&b.id));
}
fn canonical_input(mut input: WorldCommandInput) -> Result<WorldCommandInput, CommandError> {
    valid_id(&input.command_id)?;
    if input.schema_version != 1 || input.content_version != CONTENT_VERSION {
        return Err(invalid(
            "The world command or content version is unsupported.",
        ));
    }
    if let Some(id) = &input.chapter_id {
        valid_id(id)?;
    }
    match &mut input.operation {
        WorldOperation::Queue { parcels, .. } | WorldOperation::ReplaceQueue { parcels, .. } => {
            parcels.sort()
        }
        WorldOperation::Reconfigure { placements } => {
            if placements.len() > 6 {
                return Err(invalid("A layout may contain at most six buildings."));
            }
            for p in placements.iter_mut() {
                if let Some(id) = &p.id {
                    valid_id(id)?;
                }
                p.parcels.sort();
            }
            placements.sort_by(|a, b| {
                (&a.id, a.building_type, &a.parcels).cmp(&(&b.id, b.building_type, &b.parcels))
            });
        }
        _ => {}
    }
    Ok(input)
}
pub fn capabilities(layout: &[BuildingPlacement]) -> Capabilities {
    let mut result = Capabilities::default();
    for p in layout {
        let c = building_definition(p.building_type).capabilities;
        result.engineering += c.engineering;
        result.knowledge += c.knowledge;
        result.diplomacy += c.diplomacy;
    }
    result
}
fn occupied(layout: &[BuildingPlacement]) -> u32 {
    layout.iter().map(|p| p.parcels.len() as u32).sum()
}
fn satisfies(c: &Capabilities, route: Route) -> bool {
    let r = route_definition(route).requirements;
    c.engineering >= r.engineering && c.knowledge >= r.knowledge && c.diplomacy >= r.diplomacy
}
pub fn validate_layout(
    layout: &[BuildingPlacement],
    funded_sites: u32,
) -> Result<(), CommandError> {
    if layout.len() > 6 || funded_sites > 6 {
        return Err(invalid("River Reach has exactly six parcels."));
    }
    let mut ids = HashSet::new();
    let mut sites = HashSet::new();
    let mut copies = HashMap::new();
    for p in layout {
        valid_id(&p.id)?;
        if !ids.insert(&p.id) {
            return Err(invalid(
                "A building identity may occur only once in a layout.",
            ));
        }
        let definition = building_definition(p.building_type);
        if p.parcels.len() != definition.footprint as usize {
            return Err(invalid(
                "The building footprint has the wrong number of parcels.",
            ));
        }
        for site in &p.parcels {
            if !sites.insert(site) {
                return Err(invalid("Two buildings cannot occupy the same parcel."));
            }
        }
        if p.parcels.len() == 2 {
            let (ar, ac) = coordinates(p.parcels[0]);
            let (br, bc) = coordinates(p.parcels[1]);
            if ar.abs_diff(br) + ac.abs_diff(bc) != 1 {
                return Err(invalid(
                    "A specialist requires two orthogonally adjacent parcels.",
                ));
            }
        }
        let n = copies.entry(p.building_type).or_insert(0);
        *n += 1;
        if *n > definition.copy_limit {
            return Err(invalid(
                "At most two copies of a building type may occupy River Reach.",
            ));
        }
    }
    if sites.len() > funded_sites as usize {
        return Err(invalid(
            "This layout needs more funded capacity. Queue construction to purchase additional capacity.",
        ));
    }
    Ok(())
}
fn bank(state: &WorldState, issued_bc: u32) -> Result<BankProjection, CommandError> {
    let spent_bc = state
        .spending
        .iter()
        .try_fold(0u32, |n, s| n.checked_add(s.amount_bc))
        .ok_or_else(|| invalid("Construction spending exceeds its limit."))?;
    let reserved_bc = state
        .chapter
        .as_ref()
        .and_then(|c| c.queue.as_ref())
        .map_or(0, |q| q.reserved_bc);
    let available_bc = issued_bc
        .checked_sub(spent_bc)
        .and_then(|n| n.checked_sub(reserved_bc))
        .ok_or_else(|| invalid("Spending and reservations exceed issued Build Credits."))?;
    if spent_bc > 24 || spent_bc % 4 != 0 {
        return Err(invalid("The chapter construction budget is invalid."));
    }
    Ok(BankProjection {
        issued_bc,
        spent_bc,
        reserved_bc,
        available_bc,
    })
}
fn prefix(progress: &ProgressState, sequence: u32) -> Result<u32, CommandError> {
    if sequence as usize > progress.ledger.len() {
        return Err(invalid("A world command references unavailable issuance."));
    }
    progress
        .ledger
        .iter()
        .filter(|l| l.sequence <= sequence)
        .try_fold(0u32, |n, l| n.checked_add(l.amount_bc))
        .ok_or_else(|| invalid("Issuance exceeds its supported limit."))
}
fn allocate(
    progress: &ProgressState,
    state: &WorldState,
    sequence: u32,
    amount: u32,
) -> Result<Vec<FundingAllocation>, CommandError> {
    let mut used: HashMap<&str, u32> = HashMap::new();
    for spend in &state.spending {
        for a in &spend.allocations {
            *used.entry(&a.issuance_entry_id).or_default() += a.amount_bc;
        }
    }
    let mut ledger: Vec<_> = progress
        .ledger
        .iter()
        .filter(|l| l.sequence <= sequence)
        .collect();
    ledger.sort_by_key(|l| l.sequence);
    let mut remaining = amount;
    let mut result = vec![];
    for entry in ledger {
        let available = entry
            .amount_bc
            .checked_sub(*used.get(entry.id.as_str()).unwrap_or(&0))
            .ok_or_else(|| invalid("Construction attribution overuses an issuance entry."))?;
        let take = remaining.min(available);
        if take > 0 {
            result.push(FundingAllocation {
                issuance_entry_id: entry.id.clone(),
                amount_bc: take,
            });
            remaining -= take;
        }
        if remaining == 0 {
            break;
        }
    }
    if remaining != 0 {
        return Err(invalid("Issued credit cannot cover this construction."));
    }
    Ok(result)
}
fn initial_chapter(command: &str, at: &str) -> WorldChapter {
    WorldChapter {
        id: identity(command, "chapter"),
        content_version: CONTENT_VERSION.into(),
        revision: 1,
        created_at: at.into(),
        funded_site_count: 0,
        layout: vec![],
        queue: None,
        selected_route: None,
        previous_layout: None,
        completion: None,
        sandbox_alternatives: 0,
    }
}
fn queue_layout(chapter: &WorldChapter, q: &Blueprint) -> Vec<BuildingPlacement> {
    let mut layout = chapter.layout.clone();
    layout.push(BuildingPlacement {
        id: q.id.clone(),
        building_type: q.building_type,
        parcels: q.parcels.clone(),
    });
    layout
}
fn queue_cost(chapter: &WorldChapter, q: &Blueprint) -> u32 {
    (occupied(&chapter.layout) + building_definition(q.building_type).footprint)
        .saturating_sub(chapter.funded_site_count)
        * 4
}

// The same pure transition is used by live commands and complete import validation.
fn apply(
    state: &mut WorldState,
    input: &WorldCommandInput,
    progress: &ProgressState,
    ledger_sequence: u32,
    at: &str,
) -> Result<WorldCommandResult, CommandError> {
    let issued = prefix(progress, ledger_sequence)?;
    let before = bank(state, issued)?;
    let mut blueprint_id = None;
    let mut building_id = None;
    let mut spend_id = None;
    let mut ending_id = None;
    if matches!(input.operation, WorldOperation::Initialize) {
        if state.has_history() || input.chapter_id.is_some() || input.expected_world_revision != 0 {
            return Err(invalid(
                "River Reach has already been initialized or this initialization is stale.",
            ));
        }
        state.chapter = Some(initial_chapter(&input.command_id, at));
    } else {
        let mut chapter = state
            .chapter
            .clone()
            .ok_or_else(|| invalid("Enter River Reach before changing the province."))?;
        if input.chapter_id.as_ref() != Some(&chapter.id)
            || input.expected_world_revision != chapter.revision
        {
            return Err(CommandError::conflict());
        }
        match &input.operation {
            WorldOperation::Initialize => unreachable!(),
            WorldOperation::SelectRoute { route } => {
                if chapter.completion.is_some() {
                    return Err(invalid(
                        "The original ending is complete. Explore alternate layouts in the sandbox.",
                    ));
                }
                chapter.selected_route = Some(*route);
            }
            WorldOperation::Queue {
                building_type,
                parcels,
                reserve_available,
            }
            | WorldOperation::ReplaceQueue {
                building_type,
                parcels,
                reserve_available,
            } => {
                if chapter.completion.is_some() {
                    return Err(invalid(
                        "Completed chapters use free sandbox reconfiguration.",
                    ));
                }
                let replace = matches!(input.operation, WorldOperation::ReplaceQueue { .. });
                if chapter.queue.is_some() && !replace {
                    return Err(invalid(
                        "Cancel or explicitly replace the current blueprint before choosing another.",
                    ));
                }
                if chapter.queue.is_none() && replace {
                    return Err(invalid("There is no active blueprint to replace."));
                }
                let mut queue = Blueprint {
                    id: identity(&input.command_id, "blueprint"),
                    building_type: *building_type,
                    parcels: parcels.clone(),
                    reserved_bc: 0,
                    created_at: at.into(),
                };
                validate_layout(&queue_layout(&chapter, &queue), 6)?;
                let cost = queue_cost(&chapter, &queue);
                if before.spent_bc + cost > 24 {
                    return Err(invalid(
                        "This construction exceeds the six-parcel chapter budget.",
                    ));
                }
                if *reserve_available {
                    queue.reserved_bc = cost
                        .min(before.available_bc + if replace { before.reserved_bc } else { 0 });
                }
                blueprint_id = Some(queue.id.clone());
                chapter.queue = Some(queue);
            }
            WorldOperation::Reserve => {
                let queue = chapter
                    .queue
                    .as_ref()
                    .ok_or_else(|| invalid("There is no queued blueprint."))?;
                let additional = queue_cost(&chapter, queue)
                    .saturating_sub(queue.reserved_bc)
                    .min(before.available_bc);
                if additional == 0 {
                    return Err(invalid(
                        "No additional available Build Credits can be reserved for this blueprint.",
                    ));
                }
                let queue = chapter.queue.as_mut().expect("queue checked");
                queue.reserved_bc += additional;
                blueprint_id = Some(queue.id.clone());
            }
            WorldOperation::CancelQueue => {
                let queue = chapter
                    .queue
                    .take()
                    .ok_or_else(|| invalid("There is no active blueprint to cancel."))?;
                blueprint_id = Some(queue.id);
            }
            WorldOperation::CompleteConstruction => {
                if chapter.completion.is_some() {
                    return Err(invalid("The original chapter is already complete."));
                }
                let queue = chapter
                    .queue
                    .clone()
                    .ok_or_else(|| invalid("Queue a blueprint before completing construction."))?;
                validate_layout(&queue_layout(&chapter, &queue), 6)?;
                let cost = queue_cost(&chapter, &queue);
                if queue.reserved_bc > cost || cost > queue.reserved_bc + before.available_bc {
                    return Err(invalid(
                        "The blueprint needs additional legitimately issued Build Credits.",
                    ));
                }
                if before.spent_bc + cost > 24 {
                    return Err(invalid(
                        "The chapter budget cannot exceed 24 Build Credits.",
                    ));
                }
                if cost > 0 {
                    let id = identity(&input.command_id, "spend");
                    let allocations = allocate(progress, state, ledger_sequence, cost)?;
                    state.spending.push(WorldSpend {
                        id: id.clone(),
                        chapter_id: chapter.id.clone(),
                        content_version: CONTENT_VERSION.into(),
                        sequence: state.spending.len() as u32 + 1,
                        amount_bc: cost,
                        funded_sites_added: cost / 4,
                        ledger_sequence,
                        allocations,
                        created_at: at.into(),
                        command_id: input.command_id.clone(),
                    });
                    spend_id = Some(id);
                    chapter.funded_site_count += cost / 4;
                }
                let id = identity(&input.command_id, "building");
                chapter.layout.push(BuildingPlacement {
                    id: id.clone(),
                    building_type: queue.building_type,
                    parcels: queue.parcels,
                });
                building_id = Some(id);
                blueprint_id = Some(queue.id);
                chapter.queue = None;
                chapter.previous_layout = None;
            }
            WorldOperation::Reconfigure { placements } => {
                if chapter.queue.is_some() {
                    return Err(invalid(
                        "Cancel the queued blueprint before reconfiguring funded capacity.",
                    ));
                }
                let mut proposed = vec![];
                for (index, p) in placements.iter().enumerate() {
                    let id = if let Some(id) = &p.id {
                        if !chapter
                            .layout
                            .iter()
                            .any(|old| old.id == *id && old.building_type == p.building_type)
                        {
                            return Err(invalid(
                                "A retained building identity must keep its current building type.",
                            ));
                        }
                        id.clone()
                    } else {
                        identity(
                            &input.command_id,
                            &format!("configuration-building-{index}"),
                        )
                    };
                    proposed.push(BuildingPlacement {
                        id,
                        building_type: p.building_type,
                        parcels: p.parcels.clone(),
                    });
                }
                canonical_layout(&mut proposed);
                validate_layout(&proposed, chapter.funded_site_count)?;
                if proposed == chapter.layout {
                    return Err(invalid("The proposed layout is already active."));
                }
                chapter.previous_layout = Some(chapter.layout);
                chapter.layout = proposed;
                if chapter.completion.is_some() {
                    chapter.sandbox_alternatives = chapter
                        .sandbox_alternatives
                        .checked_add(1)
                        .ok_or_else(|| invalid("The sandbox history is full."))?;
                }
            }
            WorldOperation::Undo => {
                if chapter.queue.is_some() {
                    return Err(invalid(
                        "Cancel the queued blueprint before undoing a configuration.",
                    ));
                }
                let previous = chapter
                    .previous_layout
                    .take()
                    .ok_or_else(|| invalid("There is no recent configuration to undo."))?;
                validate_layout(&previous, chapter.funded_site_count)?;
                chapter.layout = previous;
            }
            WorldOperation::CompleteChapter => {
                if chapter.completion.is_some() {
                    return Err(invalid(
                        "The original River Reach ending is already preserved.",
                    ));
                }
                if chapter.queue.is_some() {
                    return Err(invalid(
                        "Complete or cancel the queued blueprint before committing the chapter ending.",
                    ));
                }
                let route = chapter.selected_route.ok_or_else(|| {
                    invalid("Select a development route before committing an ending.")
                })?;
                let capabilities = capabilities(&chapter.layout);
                if !satisfies(&capabilities, route) {
                    return Err(invalid(
                        "The committed layout does not yet satisfy the selected route.",
                    ));
                }
                let id = identity(&input.command_id, "ending");
                chapter.completion = Some(ChapterEnding {
                    id: id.clone(),
                    chapter_id: chapter.id.clone(),
                    content_version: CONTENT_VERSION.into(),
                    route,
                    layout: chapter.layout.clone(),
                    capabilities,
                    funded_site_count: chapter.funded_site_count,
                    spent_bc: before.spent_bc,
                    completed_at: at.into(),
                    command_id: input.command_id.clone(),
                    narrative: route_definition(route).ending,
                });
                chapter.previous_layout = None;
                ending_id = Some(id);
            }
        }
        chapter.revision = chapter
            .revision
            .checked_add(1)
            .ok_or_else(|| invalid("The world revision is exhausted."))?;
        canonical_layout(&mut chapter.layout);
        validate_layout(&chapter.layout, chapter.funded_site_count)?;
        state.chapter = Some(chapter);
    }
    let after = bank(state, issued)?;
    let chapter = state.chapter.as_ref().expect("initialized chapter");
    if chapter.funded_site_count != after.spent_bc / 4 {
        return Err(invalid("Funded capacity must match committed spending."));
    }
    Ok(WorldCommandResult {
        command_id: input.command_id.clone(),
        chapter_id: chapter.id.clone(),
        world_revision: chapter.revision,
        spent_bc: after.spent_bc,
        reserved_bc: after.reserved_bc,
        available_bc: after.available_bc,
        funded_site_count: chapter.funded_site_count,
        blueprint_id,
        building_id,
        spend_id,
        ending_id,
    })
}

fn rows<T: DeserializeOwned>(c: &Connection, table: &str) -> Result<Vec<T>, CommandError> {
    let mut statement = db(c.prepare(&format!("SELECT payload FROM {table} ORDER BY id")))?;
    let payloads = db(statement.query_map([], |r| r.get::<_, String>(0)))?;
    payloads
        .map(|s| serde_json::from_str(&db(s)?).map_err(|_| CommandError::storage()))
        .collect()
}
pub(crate) fn read(c: &Connection) -> Result<WorldState, CommandError> {
    let chapters: Vec<WorldChapter> = rows(c, "world_chapter")?;
    if chapters.len() > 1 {
        return Err(CommandError::storage());
    }
    let mut state = WorldState {
        chapter: chapters.into_iter().next(),
        spending: rows(c, "world_spending")?,
        commands: rows(c, "world_commands")?,
    };
    state.normalize();
    Ok(state)
}
fn insert<T: Serialize>(
    c: &Connection,
    table: &str,
    id: &str,
    value: &T,
) -> Result<(), CommandError> {
    db(c.execute(
        &format!("INSERT INTO {table}(id,payload) VALUES (?1,?2)"),
        params![id, encode(value)?],
    ))?;
    Ok(())
}
fn put_chapter(c: &Connection, chapter: &WorldChapter, new: bool) -> Result<(), CommandError> {
    if new {
        db(c.execute(
            "INSERT INTO world_chapter(id,singleton,payload) VALUES (?1,1,?2)",
            params![chapter.id, encode(chapter)?],
        ))?;
    } else {
        db(c.execute(
            "UPDATE world_chapter SET payload=?2 WHERE id=?1",
            params![chapter.id, encode(chapter)?],
        ))?;
    }
    Ok(())
}
pub(crate) fn clear(c: &Connection) -> Result<(), CommandError> {
    db(c.execute_batch(
        "DELETE FROM world_commands; DELETE FROM world_spending; DELETE FROM world_chapter;",
    ))?;
    Ok(())
}
pub(crate) fn restore(c: &Connection, state: &WorldState) -> Result<(), CommandError> {
    if let Some(chapter) = &state.chapter {
        let mut shell = chapter.clone();
        shell.funded_site_count = 0;
        shell.layout.clear();
        shell.queue = None;
        shell.previous_layout = None;
        shell.completion = None;
        put_chapter(c, &shell, true)?;
        let mut spends: Vec<_> = state.spending.iter().collect();
        spends.sort_by_key(|s| s.sequence);
        for spend in spends {
            insert(c, "world_spending", &spend.id, spend)?;
        }
        put_chapter(c, chapter, false)?;
        for command in &state.commands {
            insert(c, "world_commands", &command.id, command)?;
        }
    }
    Ok(())
}
fn allowed(c: &Connection) -> Result<(), CommandError> {
    let study = crate::study::read(c)?;
    if study.manifests.iter().any(|m| {
        matches!(m.phase, Phase::E00 | Phase::E01) && study.status(&m.id) == StudyStatus::Active
    }) {
        return Err(CommandError::new(
            "study_active",
            "River Reach is withheld during an active E00 or E01 study. Explicitly pause or leave the study before playing; existing assignments remain preserved.",
        ));
    }
    Ok(())
}
pub(crate) fn execute(
    c: &Connection,
    input: WorldCommandInput,
) -> Result<WorldCommandResult, CommandError> {
    let input = canonical_input(input)?;
    let digest = hash(&input)?;
    let previous: Option<String> = db(c
        .query_row(
            "SELECT payload FROM world_commands WHERE id=?1",
            [&input.command_id],
            |r| r.get(0),
        )
        .optional())?;
    if let Some(payload) = previous {
        let command: CommittedWorldCommand =
            serde_json::from_str(&payload).map_err(|_| CommandError::storage())?;
        if command.payload_digest != digest {
            return Err(CommandError::new(
                "idempotency_conflict",
                "This world command ID was already committed with a different payload.",
            ));
        }
        return Ok(command.result);
    }
    allowed(c)?;
    let progress = crate::progress::read(c)?;
    let mut state = read(c)?;
    let initial = state.chapter.is_none();
    let previous_spending = state.spending.len();
    let at = now();
    let sequence = progress.ledger.len() as u32;
    let result = apply(&mut state, &input, &progress, sequence, &at)?;
    let command = CommittedWorldCommand {
        id: input.command_id.clone(),
        sequence: state.commands.len() as u32 + 1,
        payload_digest: digest,
        payload: input,
        ledger_sequence: sequence,
        issued_bc: progress.wallet_bc,
        created_at: at,
        result: result.clone(),
    };
    let chapter = state.chapter.as_ref().expect("applied chapter");
    if initial {
        put_chapter(c, chapter, true)?;
    }
    for spend in &state.spending[previous_spending..] {
        insert(c, "world_spending", &spend.id, spend)?;
    }
    if !initial {
        put_chapter(c, chapter, false)?;
    }
    insert(c, "world_commands", &command.id, &command)?;
    db(c.execute(
        "UPDATE workspace_state SET generation=?1 WHERE singleton=1",
        [crate::domain::new_id()],
    ))?;
    Ok(result)
}
pub(crate) fn view(c: &Connection) -> Result<WorldView, CommandError> {
    allowed(c)?;
    let state = read(c)?;
    let progress = crate::progress::read(c)?;
    let bank = bank(&state, progress.wallet_bc)?;
    let corrected: HashSet<_> = progress
        .corrections
        .iter()
        .map(|correction| &correction.acceptance_id)
        .collect();
    let grants: HashSet<_> = progress
        .grants
        .iter()
        .filter(|grant| corrected.contains(&grant.acceptance_id))
        .map(|grant| &grant.id)
        .collect();
    let entries: HashSet<_> = progress
        .ledger
        .iter()
        .filter(|entry| grants.contains(&entry.grant_id))
        .map(|entry| &entry.id)
        .collect();
    let retained_credit_bc = state
        .spending
        .iter()
        .flat_map(|s| &s.allocations)
        .filter(|a| entries.contains(&a.issuance_entry_id))
        .map(|a| a.amount_bc)
        .sum();
    let capabilities = state
        .chapter
        .as_ref()
        .map_or_else(Capabilities::default, |chapter| {
            capabilities(&chapter.layout)
        });
    Ok(WorldView {
        content_version: CONTENT_VERSION.into(),
        chapter: state.chapter,
        bank,
        capabilities,
        parcels: parcels(),
        buildings: building_definitions(),
        routes: route_definitions(),
        retained_credit_bc,
    })
}
pub(crate) fn preview(
    c: &Connection,
    input: WorldPreviewInput,
) -> Result<WorldPreview, CommandError> {
    allowed(c)?;
    valid_id(&input.chapter_id)?;
    if input.content_version != CONTENT_VERSION {
        return Err(invalid("The preview content version is unsupported."));
    }
    if input.placements.len() > 6 {
        return Err(invalid("A preview may contain at most six buildings."));
    }
    let state = read(c)?;
    let chapter = state
        .chapter
        .ok_or_else(|| invalid("Enter River Reach before previewing a layout."))?;
    if chapter.id != input.chapter_id {
        return Err(CommandError::conflict());
    }
    let layout: Vec<_> = input
        .placements
        .iter()
        .enumerate()
        .map(|(i, p)| BuildingPlacement {
            id: p
                .id
                .clone()
                .unwrap_or_else(|| identity(&chapter.id, &format!("preview-{i}"))),
            building_type: p.building_type,
            parcels: p.parcels.clone(),
        })
        .collect();
    let mut placement_legal = validate_layout(&layout, 6).is_ok();
    let mut errors = vec![];
    if let Err(error) = validate_layout(&layout, chapter.funded_site_count) {
        errors.push(error.message);
    }
    for p in &input.placements {
        if let Some(id) = &p.id
            && !chapter
                .layout
                .iter()
                .any(|old| old.id == *id && old.building_type == p.building_type)
        {
            placement_legal = false;
            errors.push("A retained building identity must keep its current building type.".into());
            break;
        }
    }
    let capabilities = capabilities(&layout);
    let occupied_sites = occupied(&layout);
    let routes = route_definitions()
        .into_iter()
        .map(|r| RouteProgress {
            route: r.route,
            satisfied: placement_legal && satisfies(&capabilities, r.route),
            missing: Capabilities {
                engineering: r
                    .requirements
                    .engineering
                    .saturating_sub(capabilities.engineering),
                knowledge: r
                    .requirements
                    .knowledge
                    .saturating_sub(capabilities.knowledge),
                diplomacy: r
                    .requirements
                    .diplomacy
                    .saturating_sub(capabilities.diplomacy),
            },
        })
        .collect();
    Ok(WorldPreview {
        legal: errors.is_empty(),
        placement_legal,
        errors,
        occupied_sites,
        funded_site_count: chapter.funded_site_count,
        additional_sites_needed: occupied_sites.saturating_sub(chapter.funded_site_count),
        capabilities,
        routes,
    })
}

pub(crate) fn validate(state: &WorldState, records: &RecordSet) -> Result<(), CommandError> {
    let progress = records
        .progress
        .as_ref()
        .ok_or_else(|| invalid("World state requires complete issuance history."))?;
    if state.commands.len() > 10_000 || state.spending.len() > 6 {
        return Err(invalid("The world history exceeds its bounded limits."));
    }
    let mut ordered: Vec<_> = state.commands.iter().collect();
    ordered.sort_by_key(|c| c.sequence);
    let mut rebuilt = WorldState::default();
    let mut command_ids = HashSet::new();
    let mut last_prefix = 0;
    for (index, command) in ordered.into_iter().enumerate() {
        valid_id(&command.id)?;
        timestamp(&command.created_at)?;
        if !command_ids.insert(&command.id)
            || command.sequence != index as u32 + 1
            || command.id != command.payload.command_id
        {
            return Err(invalid(
                "World command identities and sequence are inconsistent.",
            ));
        }
        let canonical = canonical_input(command.payload.clone())?;
        if canonical != command.payload || hash(&canonical)? != command.payload_digest {
            return Err(invalid(
                "A world command has an invalid canonical payload or digest.",
            ));
        }
        if command.ledger_sequence < last_prefix
            || prefix(progress, command.ledger_sequence)? != command.issued_bc
        {
            return Err(invalid("World command issuance prefixes are inconsistent."));
        }
        last_prefix = command.ledger_sequence;
        let result = apply(
            &mut rebuilt,
            &canonical,
            progress,
            command.ledger_sequence,
            &command.created_at,
        )?;
        if result != command.result {
            return Err(invalid(
                "A world command result does not match its historical transition.",
            ));
        }
        rebuilt.commands.push(command.clone());
    }
    let mut actual = state.clone();
    actual.normalize();
    rebuilt.normalize();
    if actual != rebuilt {
        return Err(invalid(
            "The world layout, spending, reservation, undo, or Chronicle history is inconsistent.",
        ));
    }
    bank(state, progress.wallet_bc)?;
    // Identity collisions across authority domains are rejected, including IDs
    // that only survive in historical layouts after free reconfiguration.
    let mut external = HashSet::new();
    macro_rules! ids {
        ($rows:expr) => {
            for row in $rows {
                external.insert(row.id.as_str());
            }
        };
    }
    ids!(&records.projects);
    ids!(&records.objectives);
    ids!(&records.actions);
    ids!(&records.notes);
    ids!(&progress.slots);
    ids!(&progress.criteria);
    ids!(&progress.evidence);
    ids!(&progress.validators);
    ids!(&progress.snapshots);
    ids!(&progress.acceptances);
    ids!(&progress.corrections);
    ids!(&progress.grants);
    ids!(&progress.ledger);
    ids!(&progress.audit);
    ids!(&progress.commands);
    if let Some(s) = &records.study {
        ids!(&s.epochs);
        ids!(&s.manifests);
        ids!(&s.protocol_events);
        ids!(&s.opportunities);
        ids!(&s.assignments);
        ids!(&s.outcomes);
        ids!(&s.exposures);
        ids!(&s.burdens);
        ids!(&s.reflections);
        ids!(&s.preferences);
    }
    if let Some(daily) = &records.daily {
        ids!(&daily.checkpoints);
        ids!(&daily.commands);
    }
    let mut owned = HashSet::new();
    for command in &state.commands {
        let mut created = vec![command.id.clone()];
        if matches!(command.payload.operation, WorldOperation::Initialize) {
            created.push(command.result.chapter_id.clone());
        }
        if matches!(
            command.payload.operation,
            WorldOperation::Queue { .. } | WorldOperation::ReplaceQueue { .. }
        ) {
            created.extend(command.result.blueprint_id.clone());
        }
        created.extend(command.result.building_id.clone());
        created.extend(command.result.spend_id.clone());
        created.extend(command.result.ending_id.clone());
        if let WorldOperation::Reconfigure { placements } = &command.payload.operation {
            for (index, p) in placements.iter().enumerate() {
                if p.id.is_none() {
                    created.push(identity(
                        &command.id,
                        &format!("configuration-building-{index}"),
                    ));
                }
            }
        }
        for id in created {
            if external.contains(id.as_str()) || !owned.insert(id) {
                return Err(invalid(
                    "World identities must be distinct from all other records.",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
