fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    sindri_causeway::run();
}
