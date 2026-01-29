-- Flag projects that need maintainer to fill in metadata (e.g. after GitHub App install)
ALTER TABLE projects
  ADD COLUMN IF NOT EXISTS needs_metadata BOOLEAN NOT NULL DEFAULT false,
  ADD COLUMN IF NOT EXISTS description TEXT;

CREATE INDEX IF NOT EXISTS idx_projects_needs_metadata ON projects(needs_metadata) WHERE needs_metadata = true;
