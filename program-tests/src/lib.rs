//! Rust-side integration tests for eros_marketplace_sale.
//! Each top-level test function is `#[tokio::test]` and uses solana-program-test
//! to spin a lightweight in-process bank with our program loaded.

#[cfg(test)]
mod tests {
    use solana_program_test::ProgramTest;

    #[tokio::test]
    async fn harness_compiles_and_runs() {
        // Use None processor and set SBF_OUT_DIR so solana-program-test can
        // locate the compiled .so artifact produced by `anchor build`.
        std::env::set_var(
            "SBF_OUT_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../target/deploy"),
        );
        let pt = ProgramTest::new("eros_marketplace_sale", eros_marketplace_sale::ID, None);
        let _ctx = pt.start_with_context().await;
        // No assertions — this test exists to prove the harness wires up.
    }
}
