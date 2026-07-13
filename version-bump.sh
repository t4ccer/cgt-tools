NEW_VERSION="${1}"
TOML_FILES="$(git ls-files '*Cargo.toml' '*pyproject.toml')"
sed -i -E "s/^version = .*\$/version = \"$NEW_VERSION\"/" $TOML_FILES
sed -i -E "s/^(cgt.*version = )\"[^\"]*\"/\\1\"$NEW_VERSION\"/" $TOML_FILES
sed -i -E "s/^version = .*\$/version = \"$NEW_VERSION\"/" $TOML_FILES
sed -i -E "s/^CGT_VERSION = .*\$/CGT_VERSION = $NEW_VERSION/" Makefile
cargo metadata --format-version 1 > /dev/null
