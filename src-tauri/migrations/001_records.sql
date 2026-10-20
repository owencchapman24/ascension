CREATE TABLE projects (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision BETWEEN 1 AND 4294967295),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE objectives (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id),
    title TEXT NOT NULL,
    criterion TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision BETWEEN 1 AND 4294967295),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX objectives_project ON objectives(project_id);
CREATE TABLE actions (
    id TEXT PRIMARY KEY NOT NULL,
    objective_id TEXT NOT NULL UNIQUE REFERENCES objectives(id),
    description TEXT NOT NULL,
    stop_condition TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision BETWEEN 1 AND 4294967295),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE notes (
    id TEXT PRIMARY KEY NOT NULL,
    objective_id TEXT NOT NULL UNIQUE REFERENCES objectives(id),
    text TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision BETWEEN 1 AND 4294967295),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
