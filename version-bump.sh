NEW_VERSION="${1}"
TOML_FILES="$(git ls-files '*Cargo.toml')"
sed -i -E "s/^version = .*\$/version = \"$NEW_VERSION\"/" $TOML_FILES
sed -i -E "s/^(cgt.*version = )\"[^\"]*\"/\\1\"$NEW_VERSION\"/" $TOML_FILES
cargo metadata --format-version 1 > /dev/null
