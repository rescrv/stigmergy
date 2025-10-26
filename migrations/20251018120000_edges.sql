CREATE TABLE edges (
    src_entity BYTEA NOT NULL REFERENCES entities(entity_id) ON DELETE CASCADE,
    dst_entity BYTEA NOT NULL REFERENCES entities(entity_id) ON DELETE CASCADE,
    label_entity BYTEA NOT NULL REFERENCES entities(entity_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (src_entity, dst_entity, label_entity),
    CONSTRAINT src_entity_length CHECK (octet_length(src_entity) = 32),
    CONSTRAINT dst_entity_length CHECK (octet_length(dst_entity) = 32),
    CONSTRAINT label_entity_length CHECK (octet_length(label_entity) = 32)
);

-- Index remaining permutations for efficient queries (primary key already covers src_entity, dst_entity, label_entity)
CREATE INDEX idx_edges_src_label_dst ON edges(src_entity, label_entity, dst_entity);
CREATE INDEX idx_edges_dst_src_label ON edges(dst_entity, src_entity, label_entity);
CREATE INDEX idx_edges_dst_label_src ON edges(dst_entity, label_entity, src_entity);
CREATE INDEX idx_edges_label_src_dst ON edges(label_entity, src_entity, dst_entity);
CREATE INDEX idx_edges_label_dst_src ON edges(label_entity, dst_entity, src_entity);
