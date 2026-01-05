SELECT
    c.name AS crate_name,
    v.num AS num,
    cd.downloads AS downloads
    -- v.crate_size AS crate_size
FROM crate_downloads cd
JOIN crates c ON cd.crate_id = c.id
JOIN (
    SELECT DISTINCT ON (crate_id) *
    FROM versions
    ORDER BY crate_id, created_at DESC
) v ON v.crate_id = c.id
WHERE v.features <> '{}'
ORDER BY cd.downloads DESC
LIMIT $1;