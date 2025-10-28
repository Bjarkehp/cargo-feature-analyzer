#!/bin/bash
if [ -z "$1" ]; then
    echo "Usage: $0 <model.uvl>"
    exit 1
fi

model=$1

if [ ! -e "$model" ]; then
    echo "$model is not a file"
    exit 1
fi

echo "Core features in $model:"
flamapy core_features "$model"
echo

echo "Dead features in $model:"
flamapy dead_features "$model"
echo

echo "False optional features in $model:"
flamapy false_optional_features "$model"
echo

echo "Estimated number of configurations in $model:"
flamapy estimated_number_of_configurations "$model"
echo

echo "Exact number of configurations in $model:"
flamapy configurations_number "$model"
echo