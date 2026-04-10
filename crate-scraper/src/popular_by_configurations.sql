SELECT
    c.name as crate_name,
    v.num AS num,
    v.has_lib AS has_lib,
    v.bin_names AS bin_names,
    cd.downloads AS downloads,
    COUNT(*) AS configurations
FROM dependencies d
JOIN crates c ON c.id = d.crate_id
JOIN crate_downloads cd ON cd.crate_id = d.crate_id
JOIN (
    SELECT DISTINCT ON (crate_id) *
    FROM versions
    WHERE created_at < '2025-10-01'
    ORDER BY crate_id, created_at DESC
) v ON v.crate_id = c.id
WHERE v.features <> '{}'
GROUP BY d.crate_id, c.name, v.num, v.has_lib, v.bin_names, cd.downloads
ORDER BY configurations DESC
LIMIT $1