ALTER TABLE versions ADD COLUMN feature_count integer;

UPDATE versions set feature_count = (
    SELECT COUNT(*)
    FROM jsonb_object_keys(features)
);

ALTER TABLE crates ADD COLUMN distinct_configs integer;

UPDATE crates c SET distinct_configs = (
    SELECT COUNT(*)
    FROM (
        SELECT DISTINCT d.default_features, d.features
        FROM dependencies d
        WHERE c.id = d.crate_id
    )
);