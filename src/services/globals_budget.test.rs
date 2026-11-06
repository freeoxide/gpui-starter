use std::fs;
use std::path::Path;

const GLOBAL_IMPL_BUDGET: usize = 45;

// Text scan: cfg-gated variants (native/wasm, feature-gated) must count the
// same on every target.
fn count_global_impls(src: &Path) -> usize {
    let mut stack = vec![src.to_path_buf()];
    let mut count = 0;
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read source dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let source = fs::read_to_string(&path).expect("read source file");
                count += source
                    .lines()
                    .filter(|line| line.trim_start().starts_with("impl Global for"))
                    .count();
            }
        }
    }
    count
}

#[test]
fn global_impl_sites_stay_within_budget() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let count = count_global_impls(&src);
    assert!(
        count <= GLOBAL_IMPL_BUDGET,
        "{count} global impl sites in src/, budget is {GLOBAL_IMPL_BUDGET}; new feature \
         state goes on AppRoot-owned entities passed at construction"
    );
}
