SELECT  
    c.name AS dependent_crate, 
    v.num AS dependent_version, 
    d.req AS dependency_requirement,
    d.features,
    d.default_features
FROM dependencies d
JOIN versions v ON d.version_id = v.id
JOIN crates c ON v.crate_id = c.id
WHERE d.crate_id = (
    SELECT id as crate_id
    FROM crates
    WHERE name = $1
)
AND d.kind = 0
ORDER BY v.created_at DESC
LIMIT $2
OFFSET $3;