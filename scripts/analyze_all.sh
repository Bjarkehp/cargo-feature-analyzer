connection_string="postgresql://crates:crates@localhost:5432/crates_db"

for crate in $(cat "$1"); do
    if ! cargo run --bin flat-feature-model -- --name "$crate" models/"$crate"-flat.uvl; then
        echo "$crate has no features"
        continue;
    fi
    cargo run --bin feature-configuration-postgres "$crate" "$connection_string" configurations/"$crate" --limit 1000
    cargo run --bin feature-model-generator configurations/"$crate" models/"$crate".uvl 
    echo "$crate (flat)"
    scripts/analyze.sh models/"$crate"-flat.uvl
    echo "$crate (FCA)"
    scripts/analyze.sh models/"$crate".uvl
done
