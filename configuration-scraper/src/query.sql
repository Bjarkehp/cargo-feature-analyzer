SELECT  
    c.name AS dependent_crate, -- The name of the dependent crate.
    v.num AS dependent_version, -- The SemVer number for the dependent crate.
    d.req AS dependency_requirement, -- The version requirement of the dependency e.g. ">=1.0.0", "~1.2.3", "1.*", etc.
    d.features, -- The features enabled by the dependent crate.
    d.default_features -- If default features are enabled or not.
FROM dependencies d
-- Join each dependency on the corresponding *dependent* version.
-- A dependency does not reference the dependent
JOIN versions v ON d.version_id = v.id 
-- Information regarding the crate is also needed.
JOIN crates c ON v.crate_id = c.id
-- Only dependencies of the given crate ($1) should be returned.
WHERE d.crate_id = (
    SELECT id as crate_id
    FROM crates
    WHERE name = $1
)
-- Order such that new crates are preferred over old.
ORDER BY v.created_at DESC