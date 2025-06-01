#!/bin/bash
cargo run --bin feature-model-generator -- configurations/"$1" models/"$1".uvl --ac-poset models/"$1".acposet.dot