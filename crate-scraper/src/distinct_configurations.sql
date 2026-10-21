SELECT
    c.name as crate_name,
    v.num AS num,
    v.has_lib AS has_lib,
    v.bin_names AS bin_names,
    cd.downloads AS downloads
FROM crates C
JOIN crate_downloads cd ON cd.crate_id = c.id
JOIN (
    SELECT DISTINCT ON (crate_id) *
    FROM versions
    WHERE created_at < '2025-10-01'
    ORDER BY crate_id, created_at DESC
) v ON v.crate_id = c.id
WHERE v.feature_count >= $2 AND v.feature_count <= $3
ORDER BY c.distinct_configs DESC
LIMIT $1;