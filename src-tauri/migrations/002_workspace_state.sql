CREATE TABLE workspace_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    generation TEXT NOT NULL,
    selected_objective_id TEXT REFERENCES objectives(id)
);
