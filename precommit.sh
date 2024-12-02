cargo test || exit 1
cargo clippy || exit 1
cargo fmt || exit 1

git switch -C docs
cargo doc --no-deps || exit 1
echo "<meta http-equiv=\"refresh\" content=\"0; url=reggy\">" > target/doc/index.html || exit 1
cp -r target/doc ./docs || exit 1
git add . || exit 1
git commit -m "build docs $(date +%Y%m%d%H%M%S)" || exit 1
git switch -f main