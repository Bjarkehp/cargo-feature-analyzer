SELECT
    c.name as crate_name,
    v.num AS num,
    cd.downloads AS downloads,
    COUNT(*) AS configurations
FROM dependencies d
JOIN crates c ON c.id = d.crate_id
JOIN crate_downloads cd ON cd.crate_id = d.crate_id
JOIN (
    SELECT DISTINCT ON (crate_id) *
    FROM versions
    ORDER BY crate_id, created_at DESC
) v ON v.crate_id = c.id
WHERE v.features <> '{}'
GROUP BY d.crate_id, c.name, v.num, cd.downloads
ORDER BY configurations DESC
LIMIT $1