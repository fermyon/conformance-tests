#!/bin/bash

# Creates a tarball containing the compiled WebAssembly test binaries, 
# along with their configuration and manifest files.

set -e

OUTPUT_TAR="tests.tar.gz"

# Create a temporary directory to store the files to be archived
TEMP_DIR=$(mktemp -d)
touch "$TEMP_DIR/.gitkeep"

# Loop over each subdirectory in the base directory
for SUBDIR in tests/*/; do
    # Check if it is indeed a directory
    if [ -d "$SUBDIR" ]; then
        echo "Processing $SUBDIR..."
        
        # Navigate into the subdirectory
        cd "$SUBDIR"
        BASENAME=$(basename "$SUBDIR")

        # Create a directory in the temporary directory to store the files
        mkdir -p "$TEMP_DIR/$BASENAME/target/wasm32-unknown-unknown/release"
        
        # Build the test, and copy the build artifact to the temporary directory
        cargo build --release --target=wasm32-unknown-unknown --target-dir=target
        BUILD_ARTIFACT=$(find target/wasm32-unknown-unknown/release -maxdepth 1 -type f -name "*.wasm")
        cp "$BUILD_ARTIFACT" "$TEMP_DIR/$BASENAME/target/wasm32-unknown-unknown/release"
        
        # Copy the configuration and manifest files to the temporary directory
        cp test.json5 "$TEMP_DIR/$BASENAME"
        cp spin.toml "$TEMP_DIR/$BASENAME"
        
        # Navigate back to the base directory
        cd - > /dev/null
    fi
done

# Create the tarball from the temporary directory
tar -czf "$OUTPUT_TAR" -C "$TEMP_DIR" .

# Clean up the temporary directory
rm -rf "$TEMP_DIR"

echo "Tarball created: $OUTPUT_TAR"
