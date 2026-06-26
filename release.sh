#!/bin/bash

# Usage: ./release.sh 1.0.0

if [ -z "$1" ]; then
    echo "Usage: ./release.sh <version>"
    echo "Example: ./release.sh 1.0.0"
    exit 1
fi

VERSION="v$1"

echo "Releasing version $VERSION..."

# Ensure we are on main branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$CURRENT_BRANCH" != "main" ]; then
    echo "Error: You must be on the main branch to release."
    exit 1
fi

# Create and push tag
git tag -a "$VERSION" -m "Release $VERSION"
git push origin main
git push origin "$VERSION"

echo "Tag $VERSION pushed. GitHub Actions will now build and create the release."
