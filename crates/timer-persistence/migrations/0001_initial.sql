CREATE TABLE timers (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    green_duration_ns INTEGER NOT NULL,
    red_duration_ns INTEGER NOT NULL,
    created_at_ns INTEGER NOT NULL,
    updated_at_ns INTEGER NOT NULL,
    archived_at_ns INTEGER,
    CHECK(length(trim(name)) > 0),
    CHECK(green_duration_ns > 0),
    CHECK(red_duration_ns > 0)
);

CREATE TABLE runs (
    id TEXT PRIMARY KEY NOT NULL,
    timer_id TEXT NOT NULL,
    started_at_ns INTEGER NOT NULL,
    ended_at_ns INTEGER NOT NULL,
    end_reason TEXT NOT NULL,
    last_cycle_index INTEGER NOT NULL,
    created_at_ns INTEGER NOT NULL,
    FOREIGN KEY(timer_id) REFERENCES timers(id),
    CHECK(ended_at_ns >= started_at_ns),
    CHECK(last_cycle_index > 0),
    CHECK(end_reason IN ('user_stop'))
);

CREATE TABLE phases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL,
    sequence_index INTEGER NOT NULL,
    cycle_index INTEGER NOT NULL,
    phase_type TEXT NOT NULL,
    outcome TEXT NOT NULL,
    started_at_ns INTEGER NOT NULL,
    ended_at_ns INTEGER NOT NULL,
    allocated_duration_ns INTEGER NOT NULL,
    actual_duration_ns INTEGER NOT NULL,
    FOREIGN KEY(run_id) REFERENCES runs(id) ON DELETE CASCADE,
    UNIQUE(run_id, sequence_index),
    CHECK(sequence_index >= 0),
    CHECK(cycle_index > 0),
    CHECK(ended_at_ns >= started_at_ns),
    CHECK(allocated_duration_ns > 0),
    CHECK(actual_duration_ns >= 0),
    CHECK(actual_duration_ns <= allocated_duration_ns),
    CHECK(phase_type IN ('green', 'red')),
    CHECK(outcome IN ('completed_early', 'completed', 'expired', 'interrupted'))
);

CREATE INDEX idx_runs_timer_ended ON runs(timer_id, ended_at_ns DESC);
CREATE INDEX idx_phases_run_sequence ON phases(run_id, sequence_index);
