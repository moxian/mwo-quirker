git checkout main
git branch -f for-gh
git checkout for-gh
cargo run --release "D:\games\SteamLibrary\steamapps\common\MechWarrior Online"
wasm-pack build --release --target web --out-name wasm --out-dir ./static/pkg
Remove-Item static/.gitignore, static/pkg/.gitignore
git add static
git commit static/ -m "gh-pages commit"
git push --force origin for-gh
git checkout main
git checkout .