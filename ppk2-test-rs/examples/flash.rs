use ppk2_test_rs::{Rate, Setup};
use std::time::Duration;

fn main() {
    let mut setup = Setup::new(None, Rate::from_sps(100_000));
    let flash_script = r#"
        cd ~/courses/STAGE/epcp-rs/experiments
        rustup update
        rustup target add thumbv7em-none-eabi
        echo "FLASHING" && sleep 2
        cargo flash --chip nRF52840_xxAA --release --bin exp_measure_micros
        exit_code=$?
        cd ~/courses/STAGE/epcp-rs/ppk2-test-rs/
        exit $exit_code
        "#;
    setup.flash(flash_script);
    print!("{}", setup.measure(Duration::from_secs(2)).unwrap());
    print!("{}", setup.measure(Duration::from_secs(2)).unwrap());
}
