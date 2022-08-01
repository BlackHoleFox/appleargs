// This script is based off the one developed by the Rust developers
// back in 2017. It was used in projects like `packed_simd` and `libc`.

use std::env;
use std::fs;
use std::path::Path;
use std::process::{self, Command};

const SIM_APP_PATH: &str = "./target/ios_simulator_app";
const SIM_NAME: &str = concat!("rust_ios", "_", env!("ios_runner_crate"));
const BUNDLE_ID: &str = concat!("com.", env!("ios_runner_crate"), ".unittests");

fn package_as_simulator_app(crate_name: &str, test_binary_path: &Path) {
    println!("Packaging simulator app");
    drop(fs::remove_dir_all(SIM_APP_PATH));
    fs::create_dir(SIM_APP_PATH).expect("failed to make sim app dir");
    fs::copy(test_binary_path, Path::new(SIM_APP_PATH).join(crate_name)).unwrap();

    std::fs::write(
        format!("{SIM_APP_PATH}/Info.plist"),
        format!(
            r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <!DOCTYPE plist PUBLIC
                    "-//Apple//DTD PLIST 1.0//EN"
                    "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
            <plist version="1.0">
                <dict>
                    <key>CFBundleExecutable</key>
                    <string>{crate_name}</string>
                    <key>CFBundleIdentifier</key>
                    <string>{BUNDLE_ID}</string>
                    <key>CFBundleVersion</key>
                    <string>2022.07</string>
                    <key>CFBundleShortVersionString</key>
                    <string>0.1.0</string>
                </dict>
            </plist>
        "#
        ),
    )
    .unwrap();
}

fn start_simulator() {
    println!("Looking for iOS simulator");
    let output = Command::new("xcrun")
        .arg("simctl")
        .arg("list")
        .output()
        .unwrap();
    assert!(output.status.success(), "failed to list iOS sims");

    let mut simulator_exists = false;
    let mut simulator_booted = false;
    let mut found_rust_sim = false;
    let stdout = String::from_utf8(output.stdout).unwrap();
    for line in stdout.lines() {
        if line.contains(SIM_NAME) {
            if found_rust_sim {
                panic!(
                    "Duplicate {SIM_NAME} simulators found. Please \
                        double-check xcrun simctl list."
                );
            }
            simulator_exists = true;
            simulator_booted = line.contains("(Booted)");
            found_rust_sim = true;
        }
    }

    if simulator_exists == false {
        println!("Creating iOS simulator");
        Command::new("xcrun")
            .arg("simctl")
            .arg("create")
            .arg(SIM_NAME)
            .arg("com.apple.CoreSimulator.SimDeviceType.iPhone-SE")
            .arg("com.apple.CoreSimulator.SimRuntime.iOS-12-4")
            .check_status();
    } else if simulator_booted == true {
        println!("Shutting down already-booted simulator");
        Command::new("xcrun")
            .arg("simctl")
            .arg("shutdown")
            .arg(SIM_NAME)
            .check_status();
    }

    println!("Starting iOS simulator");
    // We can't uninstall the app (if present) as that will hang if the
    // simulator isn't completely booted; just erase the simulator instead.
    Command::new("xcrun")
        .arg("simctl")
        .arg("erase")
        .arg(SIM_NAME)
        .check_status();
    Command::new("xcrun")
        .arg("simctl")
        .arg("boot")
        .arg(SIM_NAME)
        .check_status();
}

fn install_app_to_simulator() {
    println!("Installing app to simulator");
    Command::new("xcrun")
        .arg("simctl")
        .arg("install")
        .arg("booted")
        .arg(SIM_APP_PATH)
        .check_status();
}

fn run_app_on_simulator(other_args: &[&str]) {
    println!("Running app");
    let output = Command::new("xcrun")
        .arg("simctl")
        .arg("launch")
        .arg("--console")
        .arg("booted")
        .arg(BUNDLE_ID)
        .args(other_args)
        .output()
        .unwrap();

    println!("stdout --\n{}\n", String::from_utf8_lossy(&output.stdout));
    println!("stderr --\n{}\n", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let failed = stdout
        .lines()
        .find(|l| l.contains("FAILED"))
        .map(|l| l.contains("FAILED"))
        .unwrap_or(false);

    let passed = stdout
        .lines()
        .find(|l| l.contains("test result: ok"))
        .map(|l| l.contains("test result: ok"))
        .unwrap_or(false);

    println!("Shutting down simulator");
    Command::new("xcrun")
        .arg("simctl")
        .arg("shutdown")
        .arg(SIM_NAME)
        .check_status();

    if !(passed && !failed) {
        panic!("tests didn't pass");
    }
}

trait CheckStatus {
    fn check_status(&mut self);
}

impl CheckStatus for Command {
    fn check_status(&mut self) {
        println!("\trunning: {:?}", self);
        assert!(self.status().unwrap().success(), "command failed");
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!(
            "Usage: {} <executable> [<argv 1> <argv 2> ... <argv n>]",
            args[0]
        );
        process::exit(-1);
    }

    let test_binary_path = Path::new(&args[1]);
    let crate_name = test_binary_path.file_name().unwrap();
    let test_binary_args: Vec<&str> = args.iter().skip(2).map(String::as_str).collect();

    package_as_simulator_app(crate_name.to_str().unwrap(), test_binary_path);
    start_simulator();
    install_app_to_simulator();
    run_app_on_simulator(&test_binary_args);
}
